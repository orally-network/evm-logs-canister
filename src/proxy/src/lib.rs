use candid::{candid_method, Principal};
use ic_cdk::api::call::call;
use std::vec::Vec;
use evm_logs_types::{EventNotification, SendNotificationResult, SendNotificationError};
use ic_cdk_macros::{query, update, init};

#[init]
async fn init() {
    ic_cdk::println!("Proxy canister initialized");
}

#[update(name = "send_notification")]
#[candid_method(update)]
async fn send_notification(
    subscriber: Principal,
    notification: EventNotification,
) -> SendNotificationResult {
    ic_cdk::println!("Calling handle_notification");
    
    // Send the notification to the subscriber
    let call_result: Result<(), String> = call(
        subscriber,
        "handle_notification",
        (notification.clone(),),
    )
    .await
    .map_err(|e| format!("Transport or call error: {:?}", e));

    match call_result {
        Ok(_) => {
            SendNotificationResult::Ok
        }
        Err(err_msg) => {
            ic_cdk::println!("Error sending notification: {}", err_msg);
            SendNotificationResult::Err(SendNotificationError::FailedToSend)
        }
    }
}

#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();

