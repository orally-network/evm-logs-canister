use std::vec::Vec;

use candid::{Principal, candid_method};
use evm_logs_types::{EventNotification, SendNotificationError, SendNotificationResult};
use ic_cdk::api::call::call;
use ic_cdk_macros::{init, query, update};

pub mod utils;

#[init]
async fn init() {
  log!("Proxy canister initialized");
}

#[update(name = "send_notification")]
#[candid_method(update)]
async fn send_notification(subscriber: Principal, notification: EventNotification) -> SendNotificationResult {
  // Send the notification to the subscriber
  let call_result: Result<(), String> = call(subscriber, "handle_notification", (notification.clone(),))
    .await
    .map_err(|e| format!("Transport or call error: {:?}", e));

  match call_result {
    Ok(_) => SendNotificationResult::Ok,
    Err(err_msg) => {
      log!("Error sending notification: {}", err_msg);
      SendNotificationResult::Err(SendNotificationError::FailedToSend)
    }
  }
}

#[query]
fn get_candid_pointer() -> String {
  __export_service()
}

candid::export_service!();
