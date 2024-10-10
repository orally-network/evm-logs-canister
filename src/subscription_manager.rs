use candid::{CandidType, Principal, Nat};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade};
use std::cell::RefCell;
use std::collections::HashMap;

use evm_logs_types::{
    PublicationInfo, SubscriptionInfo, Event, PublicationRegistration,
    RegisterPublicationResult, SubscriptionRegistration, RegisterSubscriptionResult,
    PublishError, EventNotification, ConfirmationResult, ICRC16Map, ICRC16Value,
};
use serde::{Serialize, Deserialize};

use crate::utils::current_timestamp;

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

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct Filter {
    pub addresses: Vec<String>,
    pub topics: Option<Vec<Vec<String>>>,
}

// #[init]/
pub fn init() {
    ic_cdk::println!("SubscriptionManager initialized");
}

#[pre_upgrade]
pub fn pre_upgrade() {
    let publications = PUBLICATIONS.with(|pubs| pubs.borrow().clone());
    let subscriptions = SUBSCRIPTIONS.with(|subs| subs.borrow().clone());
    let events = EVENTS.with(|evs| evs.borrow().clone());

    ic_cdk::storage::stable_save((publications, subscriptions, events))
        .expect("Failed to save stable state");
}

#[post_upgrade]
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

pub async fn register_publication(
    registrations: Vec<PublicationRegistration>,
) -> Vec<RegisterPublicationResult> {
    let caller = ic_cdk::caller();
    let mut results = Vec::new();

    for reg in registrations {
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

fn parse_filter_from_config(config: &Vec<ICRC16Map>) -> Option<Filter> {
    for map in config {
        if let ICRC16Value::Text(key_str) = &map.key {
            if key_str == "icrc72:subscription:filter" {
                if let ICRC16Value::Text(filter_str) = &map.value {
                    return parse_filter_string(filter_str);
                }
            }
        }
    }
    None
}

fn parse_filter_string(filter_str: &str) -> Option<Filter> {
    let mut addresses = Vec::new();
    let mut topics = Vec::new();

    let parts: Vec<&str> = filter_str.split("&&").collect();

    for part in parts {
        let part = part.trim();
        if part.starts_with("address ==") {
            let address = part["address ==".len()..].trim().to_string();
            addresses.push(address);
        } else if part.starts_with("topic ==") {
            let topic = part["topic ==".len()..].trim().trim_matches('\'').to_string();
            topics.push(vec![topic]);
        }
    }

    if addresses.is_empty() {
        return None; 
    }

    Some(Filter {
        addresses,
        topics: if topics.is_empty() { None } else { Some(topics) },
    })
}


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

        let filter = parse_filter_from_config(&reg.config);

        // Serialize filter to string
        let filter_str = filter.clone().map(|f| serde_json::to_string(&f).unwrap());

        let subscription_info = SubscriptionInfo {
            subscription_id: sub_id.clone(),
            subscriber_principal: caller,
            namespace: reg.namespace.clone(),
            config: reg.config.clone(),
            filter: filter_str.clone(),
            skip: None,
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
            "Subscription registered: ID={}, Namespace={}, Filter: {:?}",
            sub_id,
            reg.namespace,
            filter
        );

        results.push(RegisterSubscriptionResult::Ok(sub_id));
    }

    results
}

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

pub async fn confirm_messages(notification_ids: Vec<Nat>) -> ConfirmationResult {
    // TODO confirm messages
    let _ = notification_ids; // To silence unused variable warning

    ConfirmationResult::AllAccepted
}

pub async fn handle_notification(notification: EventNotification) {
    ic_cdk::println!("Received notification: {:?}", notification);

    // TODO handle notification
}

pub fn get_subscriptions_info(
    namespace: Option<String>,
    prev: Option<Nat>,
    take: Option<u64>,
    stats_filter: Option<Vec<ICRC16Map>>,
) -> Vec<SubscriptionInfo> {
    let mut subs_vec =
        SUBSCRIPTIONS.with(|subs| subs.borrow().values().cloned().collect::<Vec<_>>());

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
            subs_vec.clear();
        }
    }

    if let Some(take_number) = take {
        if subs_vec.len() > take_number as usize {
            subs_vec.truncate(take_number as usize);
        }
    }

    let _ = stats_filter; // To silence unused variable warning

    subs_vec
}

/// Get Active Filters (Used by ChainService)
pub fn get_active_filters() -> Vec<Filter> {
    SUBSCRIPTIONS.with(|subs| {
        subs.borrow()
            .values()
            .filter_map(|sub| {
                sub.filter.as_ref().and_then(|filter_str| {
                    serde_json::from_str::<Filter>(filter_str).ok()
                })
            })
            .collect()
    })
}
