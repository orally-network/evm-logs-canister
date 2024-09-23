use candid::{CandidType, Deserialize, Principal, Nat};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade};
use std::cell::RefCell;
use std::collections::HashMap;

use crate::types::*;
use crate::utils::{generate_sub_id, current_timestamp};


thread_local! {
    static PUBLICATIONS: RefCell<HashMap<Nat, PublicationInfo>> = RefCell::new(HashMap::new());
    static SUBSCRIPTIONS: RefCell<HashMap<Nat, SubscriptionInfo>> = RefCell::new(HashMap::new());
    static PUBLISHERS: RefCell<HashMap<Principal, Vec<Nat>>> = RefCell::new(HashMap::new());
    static SUBSCRIBERS: RefCell<HashMap<Principal, Vec<Nat>>> = RefCell::new(HashMap::new());
    static EVENTS: RefCell<HashMap<Nat, Event>> = RefCell::new(HashMap::new());

    static NEXT_PUBLICATION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    static NEXT_SUBSCRIPTION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    static NEXT_EVENT_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    static NEXT_NOTIFICATION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
}

// #[init]
pub fn init() {
    ic_cdk::println!("SubscriptionManager initialized");
}

// #[pre_upgrade]
pub fn pre_upgrade() {
    let publications = PUBLICATIONS.with(|pubs| pubs.borrow().clone());
    let subscriptions = SUBSCRIPTIONS.with(|subs| subs.borrow().clone());
    let events = EVENTS.with(|evs| evs.borrow().clone());

    ic_cdk::storage::stable_save((publications, subscriptions, events))
        .expect("Failed to save stable state");
}

// #[post_upgrade]
pub fn post_upgrade() {
    let (saved_publications, saved_subscriptions, saved_events): (
        HashMap<Nat, PublicationInfo>,
        HashMap<Nat, SubscriptionInfo>,
        HashMap<Nat, Event>,
    ) = ic_cdk::storage::stable_restore().expect("Failed to restore stable state");

    PUBLICATIONS.with(|pubs| *pubs.borrow_mut() = saved_publications);
    SUBSCRIPTIONS.with(|subs| *subs.borrow_mut() = saved_subscriptions);
    EVENTS.with(|evs| *evs.borrow_mut() = saved_events);
}

/// Publication registration (Orchestrator)
// #[update(name = "icrc72_register_publication")]
// #[candid_method(update)]
pub async fn register_publication(
    registrations: Vec<PublicationRegistration>,
) -> Vec<RegisterPublicationResult> {
    let caller = ic_cdk::caller();
    let mut results = Vec::new();

    for reg in registrations {
        // validation

        let pub_id = NEXT_PUBLICATION_ID.with(|id| {
            let mut id = id.borrow_mut();
            let current_id = id.clone();
            *id += Nat::from(1u32);
            current_id
        });

        let pub_info = PublicationInfo {
            namespace: reg.namespace.clone(),
            config: reg.config.clone(),
            stats: vec![],
        };

        PUBLICATIONS.with(|pubs| {
            pubs.borrow_mut().insert(pub_id.clone(), pub_info);
        });

        PUBLISHERS.with(|pubs| {
            pubs.borrow_mut()
                .entry(caller)
                .or_insert_with(Vec::new)
                .push(pub_id.clone());
        });

        ic_cdk::println!("Publication registered: ID={}, Namespace={}", pub_id, reg.namespace);

        results.push(RegisterPublicationResult::Ok(pub_id));
    }

    results
}

/// Subscription registration  (Orchestrator)
// #[update(name = "icrc72_register_subscription")]
// #[candid_method(update)]
pub async fn register_subscription(
    registrations: Vec<SubscriptionRegistration>,
) -> Vec<RegisterSubscriptionResult> {
    let caller = ic_cdk::caller();
    let mut results = Vec::new();

    for reg in registrations {
        let sub_id = NEXT_SUBSCRIPTION_ID.with(|id| {
            let mut id = id.borrow_mut();
            let current_id = id.clone();
            *id += Nat::from(1u32);
            current_id
        });

        let subscription_info = SubscriptionInfo {
            subscription_id: sub_id.clone(),
            subscriber_principal: caller,
            namespace: reg.namespace.clone(),
            config: reg.config.clone(),
            filter: None, // TODO
            skip: None,   // TODO
            stats: vec![],
        };

        SUBSCRIPTIONS.with(|subs| {
            subs.borrow_mut().insert(sub_id.clone(), subscription_info);
        });

        SUBSCRIBERS.with(|subs| {
            subs.borrow_mut()
                .entry(caller)
                .or_insert_with(Vec::new)
                .push(sub_id.clone());
        });

        ic_cdk::println!(
            "Subscription registered: ID={}, Namespace={}",
            sub_id,
            reg.namespace
        );

        results.push(RegisterSubscriptionResult::Ok(sub_id));
    }

    results
}

/// Event publishing (Broadcaster)
// #[update(name = "icrc72_publish")]
// #[candid_method(update)]
pub async fn publish_events(
    events: Vec<Event>,
) -> Vec<Option<Result<Vec<Nat>, PublishError>>> {
    let mut results = Vec::new();

    for mut event in events {
        let caller = ic_cdk::caller();

        // registration check
        let is_publisher = PUBLISHERS.with(|pubs| {
            pubs.borrow()
                .get(&caller)
                .map_or(false, |pub_ids| {
                    pub_ids.iter().any(|pub_id| {
                        PUBLICATIONS.with(|pubs_info| {
                            pubs_info
                                .borrow()
                                .get(pub_id)
                                .map_or(false, |pub_info| pub_info.namespace == event.namespace)
                        })
                    })
                })
        });

        if !is_publisher {
            results.push(Some(Err(PublishError::Unauthorized)));
            continue;
        }

        // generate event uuid
        let event_id = NEXT_EVENT_ID.with(|id| {
            let mut id = id.borrow_mut();
            let current_id = id.clone();
            *id += Nat::from(1u32);
            current_id
        });

        event.id = event_id.clone();
        event.timestamp = current_timestamp();

        EVENTS.with(|evs| {
            evs.borrow_mut().insert(event_id.clone(), event.clone());
        });

        
        distribute_event(event).await;

        results.push(Some(Ok(vec![event_id])));
    }

    results
}

// distribute event to subscribers
async fn distribute_event(event: Event) {
    let subscriptions = SUBSCRIPTIONS.with(|subs| {
        subs.borrow()
            .values()
            .filter(|sub| sub.namespace == event.namespace)
            .cloned()
            .collect::<Vec<_>>()
    });

    for sub in subscriptions {
        // check on filters
        // ... TODO

        let notification_id = NEXT_NOTIFICATION_ID.with(|id| {
            let mut id = id.borrow_mut();
            let current_id = id.clone();
            *id += Nat::from(1u32);
            current_id
        });

        let notification = EventNotification {
            id: notification_id.clone(),
            event_id: event.id.clone(),
            event_prev_id: event.prev_id.clone(),
            timestamp: current_timestamp(),
            namespace: event.namespace.clone(),
            data: event.data.clone(),
            headers: event.headers.clone(),
            source: ic_cdk::api::id(),
            filter: sub.filter.clone(),
        };

        // Send message to subscriber
        let result: Result<(), String> = ic_cdk::api::call::call(
            sub.subscriber_principal,
            "icrc72_handle_notification",
            (notification.clone(),),
        )
        .await
        .map_err(|e| format!("Failed to send notification: {:?}", e));

        match result {
            Ok(_) => {
                ic_cdk::println!(
                    "Notification sent to subscriber {}: Notification ID={}",
                    sub.subscriber_principal,
                    notification_id
                );
            }
            Err(err) => {
                ic_cdk::println!(
                    "Error sending notification to subscriber {}: {}",
                    sub.subscriber_principal,
                    err
                );
            }
        }
    }
}

/// Process confirmations from subsribers (Broadcaster)
// #[update(name = "icrc72_confirm_messages")]
// #[candid_method(update)]
pub async fn confirm_messages(notification_ids: Vec<Nat>) -> ConfirmationResult {
    // TODO confirm messages

    ConfirmationResult::AllAccepted
}

/// Handle messages (Subscriber)
// #[update(name = "icrc72_handle_notification")]
// #[candid_method(update)]
pub async fn handle_notification(notification: EventNotification) {
    ic_cdk::println!("Received notification: {:?}", notification);

    // TODO handle notification
}

/// Get Statistics (Query)
pub fn get_subscriptions_info(
    namespace: Option<String>,
    prev: Option<Nat>,
    take: Option<u64>,
    stats_filter: Option<Vec<ICRC16Map>>,
) -> Vec<SubscriptionInfo> {
    let mut subs_vec = SUBSCRIPTIONS.with(|subs| subs.borrow().values().cloned().collect::<Vec<_>>());

    if let Some(ns) = namespace {
        subs_vec.retain(|sub| sub.namespace == ns);
    }

    if let Some(prev_id) = prev {
        if let Some(pos) = subs_vec.iter().position(|sub| sub.subscription_id == prev_id) {
            if pos + 1 < subs_vec.len() {
                subs_vec = subs_vec.split_off(pos + 1);
            } else {
                subs_vec.clear();
            }
        } else {
            // empty vector in case nothing found
            subs_vec.clear();
        }
    }

    if let Some(take_number) = take {
        if subs_vec.len() > take_number as usize {
            subs_vec.truncate(take_number as usize);
        }
    }

    subs_vec
}



