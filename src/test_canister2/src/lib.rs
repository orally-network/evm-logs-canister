use ic_cdk::api::call::call;
use candid::{Principal, CandidType};
use ic_cdk_macros::{update, query, init};
use serde::Deserialize;
use std::cell::RefCell;

use evm_logs_types::{SubscriptionRegistration, RegisterSubscriptionResult, EventNotification, ICRC16Map, ICRC16Value};

thread_local! {
    static NOTIFICATIONS: RefCell<Vec<EventNotification>> = RefCell::new(Vec::new());
}

#[init]
async fn init() {
    ic_cdk::println!("Test_canister2 initialized");
}

#[update]
async fn call_icrc72_register_subscription(canister_id: Principal, registrations: Vec<SubscriptionRegistration>) {
    ic_cdk::println!("Calling icrc72_register_subscription for namespaces:");
    for reg in &registrations {
        ic_cdk::println!(" - {:?}", reg.namespace);
    }

    let result: Result<(Vec<RegisterSubscriptionResult>,), _> = call(
        canister_id,
        "icrc72_register_subscription",
        (registrations,),
    )
    .await;

    match result {
        Ok((response,)) => {
            ic_cdk::println!("Success: {:?}", response);
        }
        Err(e) => {
            ic_cdk::println!("Error calling canister: {:?}", e);
        }
    }
}

#[update]
async fn icrc72_handle_notification(notification: EventNotification) {
    ic_cdk::println!("Received notification for event ID: {:?}", notification.event_id);
    ic_cdk::println!("Notification details: {:?}", notification);

    // Зберігання отриманого сповіщення
    NOTIFICATIONS.with(|notifs| {
        notifs.borrow_mut().push(notification);
    });
}

#[query]
fn get_notifications() -> Vec<EventNotification> {
    NOTIFICATIONS.with(|notifs| notifs.borrow().clone())
}
