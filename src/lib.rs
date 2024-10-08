// lib.rs

mod utils;
mod subscription_manager;
mod chain_service;

use ic_cdk_macros::*;
use candid::candid_method;

use candid::{Nat, Principal};
use ic_cdk_macros::query;
use chain_service::{ChainService, ChainConfig};
use std::cell::RefCell;
use std::time::Duration;
use std::sync::Arc;

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
            rpc_providers: RpcServices::EthMainnet(Some(vec![EthMainnetService::Cloudflare])),
            evm_rpc_canister: Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap(),
        },
        ChainMonitoringParams {
            chain_name: "Base".to_string(),
            rpc_providers: RpcServices::BaseMainnet(Some(vec![L2MainnetService::PublicNode])),
            evm_rpc_canister: Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap(),
        },
        ChainMonitoringParams {
            chain_name: "Optimism".to_string(),
            rpc_providers: RpcServices::OptimismMainnet(Some(vec![L2MainnetService::PublicNode])),
            evm_rpc_canister: Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap(),
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

struct ChainMonitoringParams {
    chain_name: String,
    rpc_providers: RpcServices,
    evm_rpc_canister: Principal,
}

fn init_chain_service(params: ChainMonitoringParams, monitoring_interval: Duration) -> Arc<ChainService> {
    let config = ChainConfig {
        chain_name: params.chain_name,
        rpc_providers: params.rpc_providers,
        evm_rpc_canister: params.evm_rpc_canister,
    };

    let service = Arc::new(ChainService::new(config));
    service.clone().start_monitoring(monitoring_interval);
    service
}

// Orchestrator

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

// Broadcaster

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

// Subscriber

#[update(name = "icrc72_handle_notification")]
#[candid_method(update)]
async fn icrc72_handle_notification(
    notification: EventNotification,
) {
    subscription_manager::handle_notification(notification).await
}

// Query methods

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

// Candid interface export

#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();
