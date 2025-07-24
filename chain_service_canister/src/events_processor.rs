use candid::{Nat, Principal};
use evm_logs_types::{Event, EventNotification, LogEntry, SendNotificationResult, SendNotificationError};
use ic_cdk::api::call::call;

use crate::state::{read_state, mutate_state};
use crate::types::SubscriptionStatus;

const ETH_ADDRESS_SIZE: usize = 20;
const ETH_TOPIC_SIZE: usize = 32;
const CYCLES_PER_BYTE_SEND: u64 = 1000;
const CYCLES_PER_BYTE_RECEIVE: u64 = 2000;
const BASE_CALL_CYCLES: u64 = 1_000_000;
const OK_RESP_SIZE: u64 = 32;

fn timestamp_millis() -> u64 {
    ic_cdk::api::time() / 1_000_000
}

fn timestamp_nanos() -> u64 {
    ic_cdk::api::time()
}

fn event_matches_filter(event: &Event, filter: &evm_logs_types::Filter) -> bool {
    // Check address match
    if let Some(filter_address) = &filter.address {
        if &event.log_entry.address != filter_address {
            return false;
        }
    }

    // Check topics match
    if let Some(filter_topics) = &filter.topics {
        let event_topics = &event.log_entry.topics;

        if event_topics.len() < filter_topics.len() {
            return false;
        }

        return filter_topics.iter().enumerate().all(|(i, filter_topic_set)| {
            event_topics
                .get(i)
                .is_some_and(|event_topic| filter_topic_set.contains(event_topic))
        });
    }

    true
}

fn estimate_cycles_for_event_notification(event_size: usize) -> u64 {
    // Estimated request size (event notification structure)
    let request_size = event_size as u64;
    let response_size = OK_RESP_SIZE;

    // Compute cycles based on transmission costs
    let cycles_for_request = request_size * CYCLES_PER_BYTE_SEND;
    let cycles_for_response = response_size * CYCLES_PER_BYTE_RECEIVE;

    // Total estimated cycles including the base call cost
    BASE_CALL_CYCLES + cycles_for_request + cycles_for_response
}

pub async fn process_and_publish_events(logs: Vec<LogEntry>) {
    let chain_id = read_state(|state| state.config.chain_id);

    let events: Vec<Event> = logs
        .iter()
        .enumerate()
        .map(|(index, log)| Event {
            id: Nat::from(index as u64 + 1),
            timestamp: timestamp_millis(),
            chain_id,
            log_entry: log.clone(),
        })
        .collect();

    for event in events {
        distribute_event(event).await;
    }
}

async fn distribute_event(event: Event) {
    // Get all subscriptions for the event's chain_id
    let subscriptions = read_state(|state| {
        state.subscriptions
            .values()
            .filter(|sub| sub.chain_id == event.chain_id)
            .cloned()
            .collect::<Vec<_>>()
    });

    // Estimate the cycles required per event notification
    let event_size = std::mem::size_of::<EventNotification>();
    let estimated_cycles_for_event = estimate_cycles_for_event_notification(event_size);

    // Check each subscription and send a notification if the event matches the filter
    for sub in subscriptions {
        let filter = &sub.filter;
        if event_matches_filter(&event, filter) {
            let subscriber_principal = sub.subscriber_principal;

            // Check if the subscriber has sufficient balance
            let has_sufficient_balance = read_state(|state| {
                let current_balance = state.user_balances.get(&subscriber_principal).cloned().unwrap_or(Nat::from(0u32));
                current_balance >= Nat::from(estimated_cycles_for_event)
            });

            if !has_sufficient_balance {
                ic_cdk::println!(
                    "Insufficient balance for subscriber, unsubscribe: {}",
                    subscriber_principal
                );

                // Remove subscription due to insufficient balance
                mutate_state(|state| {
                    if let Some(subscription) = state.subscriptions.get_mut(&sub.subscription_id) {
                        subscription.status = SubscriptionStatus::InsufficientBalance {
                            since: ic_cdk::api::time(),
                        };
                    }
                });

                continue;
            }

            // Generate a unique notification ID
            let notification_id = mutate_state(|state| {
                let current_id = state.next_subscription_id.clone();
                state.next_subscription_id += 1u32;
                current_id
            });

            let notification = EventNotification {
                sub_id: sub.subscription_id.clone(),
                event_id: notification_id.clone(),
                timestamp: timestamp_nanos(),
                chain_id: event.chain_id,
                source: ic_cdk::api::id(),
                filter: None,
                log_entry: event.log_entry.clone(),
            };

            // Get proxy canister ID from configuration
            let proxy_canister = match read_state(|state| state.config.proxy_canister_id) {
                Some(canister_id) => canister_id,
                None => {
                    ic_cdk::println!("Proxy canister not configured, skipping notification");
                    continue;
                }
            };

            // Send the notification to the subscriber via proxy canister
            let call_result: Result<(SendNotificationResult,), _> = call(
                proxy_canister,
                "send_notification",
                (sub.subscriber_principal, notification.clone()),
            )
            .await
            .map_err(|e| format!("Failed to send notification: {:?}", e));

            match call_result {
                Ok((send_result,)) => match send_result {
                    SendNotificationResult::Ok => {
                        // If notification was successfully sent - charge this subscriber
                        mutate_state(|state| {
                            let current_balance = state.user_balances.get(&subscriber_principal).cloned().unwrap_or(Nat::from(0u32));
                            if current_balance >= Nat::from(estimated_cycles_for_event) {
                                let new_balance = current_balance - Nat::from(estimated_cycles_for_event);
                                state.user_balances.insert(subscriber_principal, new_balance);
                            }
                        });

                        ic_cdk::println!(
                            "Notification sent successfully. ID: {}, Charged: {}",
                            notification_id,
                            estimated_cycles_for_event
                        );
                    }
                    SendNotificationResult::Err(error) => {
                        // Handle application-level error
                        match error {
                            SendNotificationError::FailedToSend => {
                                ic_cdk::println!("Failed to send notification to subscriber.");
                            }
                            SendNotificationError::InvalidSubscriber => {
                                ic_cdk::println!("Invalid subscriber principal provided.");
                            }
                        }
                    }
                },
                Err(transport_error) => {
                    // Handle transport or call-level error
                    ic_cdk::println!("Error calling send_notification: {}", transport_error);
                }
            }
        }
    }
}
