
use crate::types::balances::Balances;
use crate::{EVENTS, NEXT_EVENT_ID, NEXT_NOTIFICATION_ID, SUBSCRIPTIONS};
use crate::{
    utils::{current_timestamp, event_matches_filter},
    log, get_state_value
};

use candid::Nat;
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
            .filter(|sub| sub.chain_id == event.chain_id)
            .cloned()
            .collect::<Vec<_>>()
    });
    let cycles_for_event_send = Nat::from(10_000u32); // TODO

    // Check each subscription and send a notification if the event matches the filter
    for sub in subscriptions {
        let filter = &sub.filter;
        // Check if the event matches the subscriber's filter
        if event_matches_filter(&event, filter) {
            
            let subscriber_principal = sub.subscriber_principal;
            
            if Balances::is_sufficient(subscriber_principal, cycles_for_event_send.clone()).unwrap() {
                Balances::reduce(&subscriber_principal, cycles_for_event_send.clone()).unwrap();
            }

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
                chain_id: event.chain_id.clone(),
                data: event.data.clone(),
                tx_hash: event.tx_hash.clone(),
                headers: event.headers.clone(),
                topics: event.topics.clone().unwrap_or_default(),
                source: ic_cdk::api::id(),
                filter: None,
            };

            // Send the notification to the subscriber via proxy canister
            let call_result: Result<(SendNotificationResult,), _>= call(
                get_state_value!(proxy_canister),
                "send_notification",
                (sub.subscriber_principal, notification.clone()),
            )
            .await
            .map_err(|e| format!("Failed to send notification: {:?}", e));

            match call_result {
                Ok((send_result,)) => match send_result {
                    SendNotificationResult::Ok => {
                        log!("Notification sent successfully. ID: {}", notification_id);
                    }
                    SendNotificationResult::Err(error) => {
                        // Handle application-level error
                        match error {
                            SendNotificationError::FailedToSend => {
                                log!("Failed to send notification.");
                            }
                            SendNotificationError::InvalidSubscriber => {
                                log!("Invalid subscriber principal provided.");
                            }
                        }
                    }
                },
                Err(transport_error) => {
                    // Handle transport or call-level error
                    log!("Error calling send_notification: {}", transport_error);
                }
            }
        }
    }
}
