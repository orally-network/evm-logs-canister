use candid::Principal;
use evm_logs_types::{
    EventNotification, RegisterSubscriptionResult, SubscriptionRegistration, UnsubscribeResult,
};
use ic_cdk::api::call::call;
use ic_cdk_macros::{init, query, update};
use std::cell::RefCell;

thread_local! {
    static NOTIFICATIONS: RefCell<Vec<EventNotification>> = RefCell::new(Vec::new());
}

#[init]
async fn init() {
    ic_cdk::println!("Test_canister2 initialized");
}

#[update]
async fn register_subscription(
    canister_id: Principal,
    registrations: Vec<SubscriptionRegistration>,
) {
    ic_cdk::println!("Calling register_subscription for namespaces:");
    for reg in &registrations {
        ic_cdk::println!(" - {:?}", reg.namespace);
    }

    let result: Result<(Vec<RegisterSubscriptionResult>,), _> =
        call(canister_id, "register_subscription", (registrations,)).await;

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
    ic_cdk::println!(
        "Received notification for event ID: {:?}",
        notification.event_id
    );
    ic_cdk::println!("Notification details: {:?}", notification);

    NOTIFICATIONS.with(|notifs| {
        notifs.borrow_mut().push(notification);
    });
}

#[query]
fn get_notifications() -> Vec<EventNotification> {
    NOTIFICATIONS.with(|notifs| notifs.borrow().clone())
}

#[update]
async fn unsubscribe(canister_id: Principal, subscription_id: candid::Nat) {
    ic_cdk::println!(
        "Calling unsubscribe for subscription ID: {:?}",
        subscription_id
    );

    let result: Result<(evm_logs_types::UnsubscribeResult,), _> =
        call(canister_id, "unsubscribe", (subscription_id.clone(),)).await;

    match result {
        Ok((response,)) => match response {
            UnsubscribeResult::Ok() => {
                ic_cdk::println!("Successfully unsubscribed from {:?}", subscription_id)
            }
            UnsubscribeResult::Err(err) => ic_cdk::println!("Error unsubscribing: {:?}", err),
        },
        Err(e) => {
            ic_cdk::println!("Error calling canister: {:?}", e);
        }
    }
}

#[update]
async fn get_subscriptions(canister_id: Principal) -> Vec<evm_logs_types::SubscriptionInfo> {
    ic_cdk::println!("Calling get_subscriptions");

    let result: Result<(Vec<evm_logs_types::SubscriptionInfo>,), _> =
        call(canister_id, "get_user_subscriptions", ()).await;

    match result {
        Ok((subscriptions,)) => {
            ic_cdk::println!("Successfully fetched subscriptions: {:?}", subscriptions);
            subscriptions
        }
        Err(e) => {
            ic_cdk::println!("Error fetching subscriptions: {:?}", e);
            vec![]
        }
    }
}

#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();
