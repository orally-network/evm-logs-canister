use std::str::FromStr;

use super::state::{EVENTS, NEXT_EVENT_ID, NEXT_NOTIFICATION_ID, SUBSCRIPTIONS};
use crate::utils::{current_timestamp, event_matches_filter};
use candid::{Nat, Principal};
use evm_logs_types::{Event, EventNotification, PublishError, SendNotificationResult, SendNotificationError};
use ic_cdk;
use ic_cdk::api::call::call;

// TODO rework return type
pub async fn publish_events(events: Vec<Event>) -> Vec<Option<Result<Vec<Nat>, PublishError>>> {
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

        EVENTS.with(|evs| {
            evs.borrow_mut().insert(event_id.clone(), event.clone());
        });

        distribute_event(event).await;

        results.push(Some(Ok(vec![event_id])));
    }

    results
}

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
        let filter = &sub.filter;
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
                sub_id: sub.subscription_id.clone(),
                event_id: event.id.clone(),
                event_prev_id: event.prev_id.clone(),
                timestamp: current_timestamp(),
                namespace: event.namespace.clone(),
                data: event.data.clone(),
                tx_hash: event.tx_hash.clone(),
                headers: event.headers.clone(),
                topics: event.topics.clone().unwrap_or_default(),
                source: ic_cdk::api::id(),
                filter: None,
            };

            // Send the notification to the subscriber via proxy canister
            let call_result: Result<(SendNotificationResult,), _>= call(
                Principal::from_str("be2us-64aaa-aaaaa-qaabq-cai").unwrap(), // TODO: Replace with actual canister ID
                "send_notification",
                (sub.subscriber_principal, notification.clone()),
            )
            .await
            .map_err(|e| format!("Failed to send notification: {:?}", e));

            match call_result {
                Ok((send_result,)) => match send_result {
                    SendNotificationResult::Ok => {
                        ic_cdk::println!("Notification sent successfully. ID: {}", notification_id);
                    }
                    SendNotificationResult::Err(error) => {
                        // Handle application-level error
                        match error {
                            SendNotificationError::FailedToSend => {
                                ic_cdk::println!("Failed to send notification.");
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
