use candid::Nat;
use evm_logs_types::{Event, EventNotification, SendNotificationError, SendNotificationResult};
use ic_cdk::{self, api::call::call};

use super::utils::event_matches_filter;
use crate::{
  FILTERS_MANAGER, NEXT_NOTIFICATION_ID, constants::*, get_state_value, internals::misc::timestamp_nanos,
  log_with_metrics, types::balances::Balances,
};

/// Approximate size of a response payload(just Ok response)
const OK_RESP_SIZE: u64 = 32;

fn estimate_cycles_for_event_notification(event_size: usize) -> u64 {
  // Estimated request size (event notification structure)
  let request_size = event_size as u64; // Size in bytes of the event notification payload
  let response_size = OK_RESP_SIZE;

  // Compute cycles based on transmission costs
  let cycles_for_request = request_size * CYCLES_PER_BYTE_SEND;
  let cycles_for_response = response_size * CYCLES_PER_BYTE_RECEIVE;

  // Total estimated cycles including the base call cost
  BASE_CALL_CYCLES + cycles_for_request + cycles_for_response
}

pub async fn publish_events(events: Vec<Event>) {
  for event in events {
    // all errors are being handled there individually for each event
    distribute_event(event).await;
  }
}

/// Distribute event to corresponding subscribers and handle sending errors
async fn distribute_event(event: Event) {
  // Get all subscriptions for the event's chain_id
  let subscriptions = crate::STATE.with(|state| {
    let subs = state.borrow();
    subs
      .subscriptions
      .values()
      .filter(|sub| sub.chain_id == event.chain_id)
      .cloned()
      .collect::<Vec<_>>()
  });
  // This amount is a minimum required for subscriber to have, otherwise event won't be sent
  // Estimate the cycles required per event notification
  let event_size = std::mem::size_of::<EventNotification>(); // Estimate the size of EventNotification in bytes
  let estimated_cycles_for_event = estimate_cycles_for_event_notification(event_size);

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
        timestamp: timestamp_nanos(),
        chain_id: event.chain_id,
        source: ic_cdk::api::id(),
        filter: None,
        log_entry: event.log_entry.clone(),
      };

      // Check if the subscriber has sufficient balance, otherwise - remove the subscription filter
      if !Balances::is_sufficient(subscriber_principal, Nat::from(estimated_cycles_for_event)).unwrap() {
        log_with_metrics!(
          "Insufficient balance for subscriber, unsubscribe: {}",
          subscriber_principal
        );

        // remove from subscriptions state
        crate::STATE.with(|subs| {
          subs.borrow_mut().subscriptions.remove(&sub.subscription_id);
        });

        FILTERS_MANAGER.with(|filter_manager| {
          let mut filter_manager = filter_manager.borrow_mut();
          filter_manager.remove_filter(sub.chain_id, &sub.filter);
        });

        continue;
      }

      // Send the notification to the subscriber via proxy canister
      let call_result: Result<(SendNotificationResult,), _> = call(
        get_state_value!(proxy_canister),
        "send_notification",
        (sub.subscriber_principal, notification.clone()),
      )
      .await
      .map_err(|e| format!("Failed to send notification: {:?}", e));

      match call_result {
        Ok((send_result,)) => match send_result {
          SendNotificationResult::Ok => {
            // if notification was successfully sent - charge this subscriber

            if Balances::is_sufficient(subscriber_principal, Nat::from(estimated_cycles_for_event)).unwrap() {
              Balances::reduce(&subscriber_principal, Nat::from(estimated_cycles_for_event)).unwrap();
            }

            log_with_metrics!(
              "Notification sent successfully. ID: {}, Charged: {}",
              notification_id,
              estimated_cycles_for_event
            );
          }
          SendNotificationResult::Err(error) => {
            // Handle application-level error
            match error {
              SendNotificationError::FailedToSend => {
                log_with_metrics!("Failed to send notification to subscriber.");
              }
              SendNotificationError::InvalidSubscriber => {
                log_with_metrics!("Invalid subscriber principal provided.");
              }
            }
          }
        },
        Err(transport_error) => {
          // Handle transport or call-level error
          log_with_metrics!("Error calling send_notification: {}", transport_error);
        }
      }
    }
  }
}
