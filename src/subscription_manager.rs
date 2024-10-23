use candid::{Principal, Nat};
use std::{cell::RefCell, collections::HashSet};
use std::collections::HashMap;
use crate::topic_manager::TopicManager;
use crate::utils::event_matches_filter;

use evm_logs_types::{
    SubscriptionInfo, Event, SubscriptionRegistration, RegisterSubscriptionResult, RegisterSubscriptionError,
    EventNotification, PublishError, Filter, UnsubscribeResult, ICRC16Value
};

use crate::utils::current_timestamp;

thread_local! {
    static SUBSCRIPTIONS: RefCell<HashMap<Nat, SubscriptionInfo>> = RefCell::new(HashMap::new());
    static SUBSCRIBERS: RefCell<HashMap<Principal, Vec<Nat>>> = RefCell::new(HashMap::new());
    static EVENTS: RefCell<HashMap<Nat, Event>> = RefCell::new(HashMap::new());

    static NEXT_SUBSCRIPTION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    static NEXT_EVENT_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    static NEXT_NOTIFICATION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));

    static ADDRESSES: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
    
    static TOPICS_MANAGER: RefCell<TopicManager> = RefCell::new(TopicManager::new());


}

pub fn init() {
    ic_cdk::println!("SubscriptionManager initialized");
}

pub async fn register_subscription(
    registrations: Vec<SubscriptionRegistration>,
) -> Vec<RegisterSubscriptionResult> {
    let caller = ic_cdk::caller();
    let mut results = Vec::new();

    for reg in registrations {
        let filters= reg.filters.clone();

        let is_subscription_exist = SUBSCRIBERS.with(|subs| {
            subs.borrow()
                .get(&caller)
                .and_then(|sub_ids| {
                    sub_ids.iter().find_map(|sub_id| {
                        SUBSCRIPTIONS.with(|subs| {
                            subs.borrow()
                                .get(sub_id)
                                .filter(|sub_info| sub_info.filters == filters)
                                .cloned()
                        })
                    })
                })
        });

        if let Some(_) = is_subscription_exist {
            ic_cdk::println!(
                "Subscription already exists for caller {} with the same filters",
                caller
            );
            results.push(RegisterSubscriptionResult::Err(RegisterSubscriptionError::SameFilterExists));
            continue;
        }


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
            filters: filters.clone(),
            skip: None,
            stats: vec![],
        };


        ADDRESSES.with(|addr_map| {
            let mut addr_count_map = addr_map.borrow_mut();
            for filter in &filters {
                for address in &filter.addresses {
                    *addr_count_map.entry(address.clone()).or_insert(0) += 1;
                }
            }
        });

        TOPICS_MANAGER.with(|manager| {
            let mut manager = manager.borrow_mut();
            for filter in &filters {
                manager.add_filter(&filter.topics);
            }
            ic_cdk::println!("TOPICS MANAGER: {:?}", manager.subscriptions_accept_any_topic_at_position);
        });
        
        
        
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
            reg.namespace,
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
        // Generate a unique event ID
        let event_id = NEXT_EVENT_ID.with(|id| {
            let mut id = id.borrow_mut();
            let current_id = id.clone();
            *id += Nat::from(1u32);
            current_id
        });

        // Update event data with the new event ID and current timestamp
        event.id = event_id.clone();
        event.timestamp = current_timestamp();

        // Store the event in the EVENTS collection
        EVENTS.with(|evs| {
            evs.borrow_mut().insert(event_id.clone(), event.clone());
        });

        // Distribute the event to subscribers
        distribute_event(event).await;

        // Append the result of the event publication
        results.push(Some(Ok(vec![event_id])));
    }

    results
}

// Function to distribute the event to subscribers
async fn distribute_event(event: Event) {
    // Get all subscriptions for the event's namespace
    let subscriptions = SUBSCRIPTIONS.with(|subs| {
        subs.borrow()
            .values()
            .filter(|sub| sub.namespace == event.namespace)
            .cloned()
            .collect::<Vec<_>>()
    });

    // Check each subscription and send a notification if the event matches the filter
    for sub in subscriptions {
        let filters= &sub.filters;
        for filter in filters {
            // Check if the event matches the subscriber's filter
            if event_matches_filter(&event, filter) {
                // Generate a unique notification ID
                let notification_id = NEXT_NOTIFICATION_ID.with(|id| {
                    let mut id = id.borrow_mut();
                    let current_id = id.clone();
                    *id += Nat::from(1u32);
                    current_id
                });

                // Create the notification to send
                let notification = EventNotification {
                    id: notification_id.clone(),
                    event_id: event.id.clone(),
                    event_prev_id: event.prev_id.clone(),
                    timestamp: current_timestamp(),
                    namespace: event.namespace.clone(),
                    data: event.data.clone(),
                    tx_hash: event.tx_hash.clone(),
                    headers: event.headers.clone(),
                    source: ic_cdk::api::id(),
                    filter: None, // We don't need to store the filter in the notification
                };

                // Send the notification to the subscriber
                let result: Result<(), String> = ic_cdk::api::call::call(
                    sub.subscriber_principal, // Use the subscriber's Principal
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
    }
}

pub fn get_subscriptions_info(
    namespace: Option<String>,
    from_id: Option<Nat>,
    filters: Option<Vec<Filter>>,
) -> Vec<SubscriptionInfo> {
    let mut subs_vec =
        SUBSCRIPTIONS.with(|subs| subs.borrow().values().cloned().collect::<Vec<_>>());

    if let Some(ns) = namespace {
        subs_vec.retain(|sub| sub.namespace == ns);
    }

    if let Some(prev_id) = from_id {
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

    let _ = filters; // To silence unused variable warning

    subs_vec
}


pub fn get_active_filters() -> Vec<Filter> {
    SUBSCRIPTIONS.with(|subs| {
        subs.borrow()
            .values()
            .flat_map(|sub| sub.filters.clone().into_iter()) 
            .collect()
    })
}


// Get unique addresses and topics to pass to eth_getLogs.
pub fn get_active_addresses_and_topics() -> (Vec<String>, Option<Vec<Vec<String>>>) {
    let addresses: Vec<String> = ADDRESSES.with(|addr| {
        addr.borrow().keys().cloned().collect()
    });

    let topics = TOPICS_MANAGER.with(|manager| {
        manager.borrow().get_combined_topics()
    });

    (addresses, topics)
}



pub fn get_user_subscriptions(caller: Principal) -> Vec<SubscriptionInfo> {

    let subscription_ids = SUBSCRIBERS.with(|subs| {
        subs.borrow()
            .get(&caller)
            .cloned()
            .unwrap_or_else(|| vec![]) 
    });

    SUBSCRIPTIONS.with(|subs| {
        subscription_ids
            .iter()
            .filter_map(|id| subs.borrow().get(id).cloned()) 
            .collect()
    })
}

pub fn unsubscribe(caller: Principal, subscription_id: Nat) -> UnsubscribeResult {
    let subscription_removed = SUBSCRIPTIONS.with(|subs| {
        let mut subs = subs.borrow_mut();
        subs.remove(&subscription_id)
    });

    if let Some(subscription_info) = subscription_removed {
        let filters = subscription_info.filters;

        ADDRESSES.with(|addr_map| {
            let mut addr_count_map = addr_map.borrow_mut();
            for filter in &filters {
                for address in &filter.addresses {
                    if let Some(count) = addr_count_map.get_mut(address) {
                        *count -= 1;
                        if *count == 0 {
                            addr_count_map.remove(address);  
                        }
                    }
                }
            }
        });

        TOPICS_MANAGER.with(|manager| {
            let mut manager = manager.borrow_mut();
            for filter in &filters {
                manager.remove_filter(&filter.topics);
            }
        });

        SUBSCRIBERS.with(|subs| {
            let mut subs = subs.borrow_mut();
            if let Some(sub_list) = subs.get_mut(&caller) {
                sub_list.retain(|id| *id != subscription_id);
                if sub_list.is_empty() {
                    subs.remove(&caller);
                }
            }
        });

        UnsubscribeResult::Ok()
    } else {
        UnsubscribeResult::Err(format!("Subscription with ID {} not found", subscription_id))
    }
}






