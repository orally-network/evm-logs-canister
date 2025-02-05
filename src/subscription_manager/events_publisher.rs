
use crate::types::balances::Balances;
use crate::NEXT_NOTIFICATION_ID;
use crate::{
    utils::current_timestamp,
    log, get_state_value
};
use super::utils::event_matches_filter;
use candid::Nat;
use evm_logs_types::{Event, EventNotification, SendNotificationResult, SendNotificationError};
use ic_cdk;
use ic_cdk::api::call::call;

pub async fn publish_events(events: Vec<Event>) {
    for event in events {
        // all errors are being handled there individually for each event
        distribute_event(event).await;
    }
}

// distibute event to corresponding subscribers and handle sending errors
async fn distribute_event(event: Event) {
    let balance_before = ic_cdk::api::canister_balance();

    // Get all subscriptions for the event's chain_id
    let subscriptions = crate::STATE.with(|state| {
        let subs = state.borrow();
        subs.subscriptions
            .values()
            .filter(|sub| sub.chain_id == event.chain_id)
            .cloned()
            .collect::<Vec<_>>()
    });
    // this amount is a minimum required for subscriber to have, otherwise event won't be send
    let estimate_cycles_for_event_send = Nat::from(1_000_000u32);

    // Check each subscription and send a notification if the event matches the filter
    for sub in subscriptions {
        let filter = &sub.filter;
        if event_matches_filter(&event, filter) {
            
            let subscriber_principal = sub.subscriber_principal;

            // Generate a unique notification ID
            let notification_id = NEXT_NOTIFICATION_ID.with(|id| {
                let mut id = id.borrow_mut();
                let current_id = id.clone();
                *id += Nat::from(1u32);
                current_id
            });

            let notification = EventNotification {
                sub_id: sub.subscription_id.clone(),
                event_id: notification_id.clone(),
                event_prev_id: event.prev_id.clone(),
                timestamp: current_timestamp(),
                chain_id: event.chain_id,
                data: event.data.clone(),
                tx_hash: event.tx_hash.clone(),
                headers: event.headers.clone(),
                address: event.address.clone(),
                topics: event.topics.clone().unwrap_or_default(),
                source: ic_cdk::api::id(),
                filter: None,
            };

            // Check if the subscriber has sufficient balance
            if !Balances::is_sufficient(subscriber_principal, estimate_cycles_for_event_send.clone()).unwrap() {
                log!("Insufficient balance for subscriber: {}", subscriber_principal);
                continue;
            }
            
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
                        // if notification was succesfully sent - charge this subscriber 
                        let balance_after = ic_cdk::api::canister_balance();
                        let cycles_spent = balance_before - balance_after;

                        if Balances::is_sufficient(subscriber_principal, Nat::from(cycles_spent)).unwrap() {
                            Balances::reduce(&subscriber_principal, Nat::from(cycles_spent)).unwrap();
                        }
                        
                        log!("Notification sent successfully. ID: {}, Charged: {}", notification_id, cycles_spent);
                    }
                    SendNotificationResult::Err(error) => {
                        // Handle application-level error
                        match error {
                            SendNotificationError::FailedToSend => {
                                log!("Failed to send notification to subscriber.");
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
