use candid::{candid_method, Principal};
use ic_cdk::api::call::call;
use std::vec::Vec;
use evm_logs_types::EventNotification;
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
) -> Result<(), String> {
    ic_cdk::println!("Calling handle_notification");
    // Send the notification to the subscriber
    // TODO propagate error to evm-logs-canister? handle error correctly
    let result: Result<(), String> = call(
        subscriber,
        "handle_notification",
        (notification.clone(),),
    )
    .await
    .map_err(|e| format!("Failed to send notification: {:?}", e));


    Ok(())
}

#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();

