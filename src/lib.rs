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
        ChainConfig {
            chain_name: "Ethereum".to_string(),
            rpc_providers: RpcServices::EthMainnet(Some(vec![EthMainnetService::Alchemy])),
            evm_rpc_canister: Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap(),
        },
        ChainConfig {
            chain_name: "Base".to_string(),
            rpc_providers: RpcServices::BaseMainnet(Some(vec![L2MainnetService::PublicNode])),
            evm_rpc_canister: Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap(),
        },
        ChainConfig {
            chain_name: "Optimism".to_string(),
            rpc_providers: RpcServices::OptimismMainnet(Some(vec![L2MainnetService::PublicNode])),
            evm_rpc_canister: Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap(),
        },
    ];

    let services: Vec<Arc<ChainService>> = chain_configs
        .into_iter()
        .map(|config| init_chain_service(config, monitoring_interval))
        .collect();

    CHAIN_SERVICES.with(|services_ref| {
        let mut services_cell = services_ref.borrow_mut();
        services_cell.extend(services);
    });

    ic_cdk::println!("EVM logs monitoring is started");
}

fn init_chain_service(config: ChainConfig, monitoring_interval: Duration) -> Arc<ChainService> {
    let service = Arc::new(ChainService::new(config));
    service.clone().start_monitoring(monitoring_interval);
    service
}


// Candid methods

// register subscription by specified filter(adresses and topics)
#[update(name = "register_subscription")]
#[candid_method(update)]
async fn register_subscription(
    registrations: Vec<SubscriptionRegistration>,
) -> Vec<RegisterSubscriptionResult> {
    subscription_manager::register_subscription(registrations).await
}

// unsubscribe from subcription with specified ID
#[update(name = "unsubscribe")]
#[candid_method(update)]
async fn unsubscribe(subscription_id: Nat) -> UnsubscribeResult {
    subscription_manager::unsubscribe(ic_cdk::caller(), subscription_id)
}

// get all subscriptions assigned to the user
#[query(name = "get_user_subscriptions")]
#[candid_method(query)]
fn get_user_subscriptions() -> Vec<SubscriptionInfo> {
    subscription_manager::get_user_subscriptions(ic_cdk::caller())
}

// generally for testing purpose

// get all evm-logs-canister filters info. 
#[query(name = "get_active_filters")]
#[candid_method(query)]
fn get_active_filters() -> Vec<evm_logs_types::Filter> {
    subscription_manager::get_active_filters()
}

// get all evm-logs-canister addresses and topics which are being monitored. Must be unique
#[query(name = "get_active_addresses_and_topics")]
#[candid_method(query)]
fn get_active_addresses_and_topics() -> (Vec<String>, Option<Vec<Vec<String>>>) {
    subscription_manager::get_active_addresses_and_topics()
}

// get all evm-logs-canister subscriptions info
#[query(name = "get_subscriptions")]
#[candid_method(query)]
fn get_subscriptions(
    namespace: Option<String>,
    from_id: Option<Nat>,
    filters: Option<Vec<Filter>>,
) -> Vec<SubscriptionInfo> {
    subscription_manager::get_subscriptions_info(namespace, from_id, filters)
}

#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();
