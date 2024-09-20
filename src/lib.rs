// #[macro_use]
// extern crate num_derive;

mod types;
mod utils;
mod subscription_manager;
mod chain_service; 

use ic_cdk_macros::*;
use candid::candid_method;
use crate::types::*;

use candid::Nat;
use ic_cdk_macros::query;
use chain_service::ChainService;
use std::cell::RefCell;


thread_local! {
    static CHAIN_SERVICE: RefCell<Option<ChainService>> = RefCell::new(None);
}

// canister init and update

#[init]
fn init() {
    subscription_manager::init();

    CHAIN_SERVICE.with(|cs| {
        *cs.borrow_mut() = Some(ChainService::new("https://rpc-url".to_string()));
    });
}

#[pre_upgrade]
fn pre_upgrade() {
    subscription_manager::pre_upgrade();
}

#[post_upgrade]
fn post_upgrade() {
    subscription_manager::post_upgrade();
    CHAIN_SERVICE.with(|cs| {
        *cs.borrow_mut() = Some(ChainService::new("https://rpc-url".to_string()));
    });
}

// Orchestrator export 

#[update]
#[candid_method(update)]
async fn call_register_publication(
    registrations: Vec<PublicationRegistration>,
) -> Vec<RegisterPublicationResult> {
    subscription_manager::register_publication(registrations).await
}

#[update(name = "icrc72_register_subscription")]
#[candid_method(update)]
async fn call_register_subscription(
    registrations: Vec<SubscriptionRegistration>,
) -> Vec<RegisterSubscriptionResult> {
    subscription_manager::register_subscription(registrations).await
}

// Broadcaster export

#[update(name = "icrc72_publish")]
#[candid_method(update)]
async fn icrc72_publish(
    events: Vec<Event>,
) -> Vec<Option<Result<Vec<Nat>, PublishError>>> {
    subscription_manager::publish_events(events).await
}

#[update]
#[candid_method(update)]
async fn call_confirm_messages(
    notification_ids: Vec<Nat>,
) -> ConfirmationResult {
    subscription_manager::confirm_messages(notification_ids).await
}


// Subscriber export

#[update(name = "icrc72_handle_notification")]
#[candid_method(update)]
async fn icrc72_handle_notification(
    notification: EventNotification,
) {
    subscription_manager::handle_notification(notification).await
}

// Query methods export

#[query(name = "icrc72_get_subscriptions")]
#[candid_method(query)]
fn call_get_subscriptions(
    namespace: Option<String>,
    prev: Option<Nat>,
    take: Option<u64>,
    stats_filter: Option<Vec<ICRC16Map>>,
) -> Vec<SubscriptionInfo> {
    subscription_manager::get_subscriptions_info(namespace, prev, take, stats_filter)
}

// ChainService: get EVM logs
#[update]
#[candid_method(update)]
async fn get_chain_events() -> Vec<String> {
    let chain_service = ChainService::new("bd3sg-teaaa-aaaaa-qaaba-cai".to_string());
    let logs_result = chain_service.fetch_logs(20697988, 20697990, Some("0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe".to_string())).await;

    match logs_result {
        Ok(logs) => logs,
        Err(err) => vec![format!("{}", err)],
    }
}


// Candid interface export

#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();