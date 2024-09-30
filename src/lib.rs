// #[macro_use]
// extern crate num_derive;
mod utils;
mod subscription_manager;
mod chain_service;

use ic_cdk_macros::*;
use candid::candid_method;

use candid::Nat;
use ic_cdk_macros::query;
use chain_service::{ChainService, ChainConfig};
use std::cell::RefCell;
use std::time::Duration;
use std::sync::Arc;
use candid::Principal;

use evm_logs_types::*;

use evm_rpc_canister_types::{
    BlockTag, EthMainnetService, L2MainnetService, GetLogsArgs, EvmRpcCanister, GetBlockByNumberResult, GetLogsResult, HttpOutcallError, MultiGetBlockByNumberResult, MultiGetLogsResult, RejectionCode, RpcError, RpcServices, EVM_RPC
};


thread_local! {
    static CHAIN_SERVICES: RefCell<Vec<Arc<ChainService>>> = RefCell::new(Vec::new());
}

// canister init and update

#[init]
async fn init() {
    subscription_manager::init();

    let ethereum_config = ChainConfig {
        chain_name: "Ethereum".to_string(),
        rpc_providers: evm_rpc_canister_types::RpcServices::EthMainnet(Some(vec![EthMainnetService::Alchemy])),
        evm_rpc_canister: Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap(),
    };
    let base_config = ChainConfig {
        chain_name: "Base".to_string(),
        rpc_providers: evm_rpc_canister_types::RpcServices::BaseMainnet(Some(vec![L2MainnetService::PublicNode])),
        evm_rpc_canister: Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap(),
    };
    let optimism_config = ChainConfig {
        chain_name: "Optimism".to_string(),
        rpc_providers: evm_rpc_canister_types::RpcServices::OptimismMainnet(Some(vec![L2MainnetService::PublicNode])),
        evm_rpc_canister: Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap(),
    };

    let ethereum_service = Arc::new(ChainService::new(ethereum_config));
    let base_service = Arc::new(ChainService::new(base_config));
    let optimism_service = Arc::new(ChainService::new(optimism_config));

    ethereum_service.clone().start_monitoring(Duration::from_secs(60));
    base_service.clone().start_monitoring(Duration::from_secs(60));
    optimism_service.clone().start_monitoring(Duration::from_secs(60));

    CHAIN_SERVICES.with(|services| {
        let mut services = services.borrow_mut();
        services.push(ethereum_service);
        services.push(base_service);
        services.push(optimism_service);
    });

    ic_cdk::println!("EVM logs monitoring is started");
}


// #[pre_upgrade]
// fn pre_upgrade() {
//     subscription_manager::pre_upgrade();
// }

// #[post_upgrade]
// fn post_upgrade() {
//     subscription_manager::post_upgrade();
//     CHAIN_SERVICE.with(|cs| {
//         *cs.borrow_mut() = Some(ChainService::new("https://rpc-url".to_string()));
//     });
// }

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
// #[update]
// #[candid_method(update)]
// async fn get_chain_events() -> String {
//     // let chain_service = ChainService::new("bd3sg-teaaa-aaaaa-qaaba-cai".to_string());
//     // chain_service.start_monitoring(Duration::from_secs(40));

//     // "EVM logs monitoring is started".to_string()

//     let ethereum_config = ChainConfig { 
//         chain_name: "Ethereum".to_string(), 
//         rpc_providers: evm_rpc_canister_types::RpcServices::EthMainnet(()), 
//         evm_rpc_canister: evm_rpc_canister_types::RpcServices
//     };
//     let base_config = ChainConfig { /* ... */ };
//     let optimism_config = ChainConfig { /* ... */ };

//     // Create services for each chain
//     let ethereum_service = ChainService::new(ethereum_config);
//     let base_service = ChainService::new(base_config);
//     let optimism_service = ChainService::new(optimism_config);

//     // Start monitoring
//     ethereum_service.start_monitoring(Duration::from_secs(60));
//     base_service.start_monitoring(Duration::from_secs(60));
//     optimism_service.start_monitoring(Duration::from_secs(60));

//     "EVM logs monitoring is started".to_string()
// }


// Candid interface export

#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();