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
    EthMainnetService, L2MainnetService, RpcServices,
};


thread_local! {
    static CHAIN_SERVICES: RefCell<Vec<Arc<ChainService>>> = RefCell::new(Vec::new());
}

#[init]
async fn init() {
    subscription_manager::init();

    let monitoring_interval = Duration::from_secs(40);

    let chain_configs = vec![
        ChainMonitoringParams {
            chain_name: "Ethereum".to_string(),
            rpc_providers: RpcServices::EthMainnet(Some(vec![EthMainnetService::Alchemy])),
            addresses: vec!["0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".to_string()],
            topics: None,
        },
        ChainMonitoringParams {
            chain_name: "Base".to_string(),
            rpc_providers: RpcServices::BaseMainnet(Some(vec![L2MainnetService::PublicNode])),
            addresses: vec!["0xdC2ccCdE78754D5eC82Ea2CaACB917E1F1437568".to_string()],
            topics: None,
        },
        ChainMonitoringParams {
            chain_name: "Optimism".to_string(),
            rpc_providers: RpcServices::OptimismMainnet(Some(vec![L2MainnetService::PublicNode])),
            addresses: vec!["0xC110E7FAA95680c79937CCACa3d1caB7902bE25e".to_string()],
            topics: None,
        },
    ];

    let services: Vec<Arc<ChainService>> = chain_configs
        .into_iter()
        .map(|params| init_chain_service(params, monitoring_interval))
        .collect();

    CHAIN_SERVICES.with(|services_ref| {
        let mut services_cell = services_ref.borrow_mut();
        services_cell.extend(services);
    });

    ic_cdk::println!("EVM logs monitoring is started");
}


// this struct describes parameters used to execute logs monitoring
struct ChainMonitoringParams {
    chain_name: String,
    rpc_providers: RpcServices,
    addresses: Vec<String>,
    topics: Option<Vec<Vec<String>>>,
}

fn init_chain_service(params: ChainMonitoringParams, monitoring_interval: Duration) -> Arc<ChainService> {
    let config = ChainConfig {
        chain_name: params.chain_name,
        rpc_providers: params.rpc_providers,
        evm_rpc_canister: Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap(),
        addresses: params.addresses,
        topics: params.topics,
    };

    let service = Arc::new(ChainService::new(config));
    service.clone().start_monitoring(monitoring_interval);
    service
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