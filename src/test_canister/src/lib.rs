use ic_cdk::api::call::call;
use candid::Principal;
use ic_cdk_macros::{update, query, init};
use std::cell::RefCell;

use evm_logs_types::{SubscriptionRegistration, RegisterSubscriptionResult, EventNotification};

thread_local! {
    static NOTIFICATIONS: RefCell<Vec<EventNotification>> = RefCell::new(Vec::new());
}

#[init]
async fn init() {
    ic_cdk::println!("Test_canister initialized");
}

#[update]
async fn call_icrc72_register_subscription() {
    // TODO set canister_id during init method
    let canister_id = Principal::from_text("bkyz2-fmaaa-aaaaa-qaaaq-cai").unwrap();

    let namespaces = vec![
        "com.example.myapp.events.Ethereum",
        "com.example.myapp.events.Optimism",
        "com.example.myapp.events.Base",
    ];

    let registrations: Vec<SubscriptionRegistration> = namespaces
        .into_iter()
        .map(|namespace| SubscriptionRegistration {
            namespace: namespace.to_string(),
            config: vec![],
            memo: None,
        })
        .collect();

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

    // Store the received notification
    NOTIFICATIONS.with(|notifs| {
        notifs.borrow_mut().push(notification);
    });
}

#[query]
fn get_notifications() -> Vec<EventNotification> {
    NOTIFICATIONS.with(|notifs| notifs.borrow().clone())
}


