// lib.rs

mod chain_service;
mod log_filters;
mod subscription_manager;
mod utils;
mod types;

use candid::candid_method;
use ic_cdk_macros::*;

use candid::Nat;
use chain_service::{service::ChainService, ChainConfig};
use ic_cdk_macros::query;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;
use crate::types::state::State;
use evm_logs_types::*;

use evm_rpc_canister_types::{EthMainnetService, L2MainnetService, RpcApi, RpcServices};

thread_local! {
    static CHAIN_SERVICES: RefCell<Vec<Arc<ChainService>>> = const { RefCell::new(Vec::new()) };
    pub static STATE: RefCell<State> = RefCell::default();
}

#[init]
async fn init(config: types::config::Config) {
    subscription_manager::init();
    crate::types::state::init(config);

    let monitoring_interval = Duration::from_secs(20); // TODO mode to state/config

    let chain_configs = vec![
        ChainConfig {
            chain_name: ChainName::Ethereum,
            rpc_providers: RpcServices::EthMainnet(Some(vec![EthMainnetService::PublicNode])),
            evm_rpc_canister: get_state_value!(evm_rpc_canister),
        },
        ChainConfig {
            chain_name: ChainName::Base,
            rpc_providers: RpcServices::BaseMainnet(Some(vec![L2MainnetService::PublicNode])),
            evm_rpc_canister: get_state_value!(evm_rpc_canister),
        },
        ChainConfig {
            chain_name: ChainName::Optimism,
            rpc_providers: RpcServices::OptimismMainnet(Some(vec![L2MainnetService::PublicNode])),
            evm_rpc_canister: get_state_value!(evm_rpc_canister),
        },
        ChainConfig {
            chain_name: ChainName::Polygon,
            rpc_providers: RpcServices::Custom {
                chainId: 137,
                services: vec![RpcApi {
                    url: "https://polygon-rpc.com".to_string(),
                    headers: None,
                }],
            },
            evm_rpc_canister: get_state_value!(evm_rpc_canister),
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

    log!("EVM logs monitoring is started");
}

fn init_chain_service(config: ChainConfig, monitoring_interval: Duration) -> Arc<ChainService> {
    let service = Arc::new(ChainService::new(config));
    service.clone().start_monitoring(monitoring_interval);
    service
}

// Candid methods

// register subscription by specified filter(adresses and topics)
#[update(name = "subscribe")]
#[candid_method(update)]
async fn subscribe(
    registration: SubscriptionRegistration,
) -> RegisterSubscriptionResult {
    subscription_manager::register_subscription(registration).await
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
    subscription_manager::queries::get_user_subscriptions(ic_cdk::caller())
}

// generally for testing purpose

// get all evm-logs-canister filters info.
#[query(name = "get_active_filters")]
#[candid_method(query)]
fn get_active_filters() -> Vec<evm_logs_types::Filter> {
    subscription_manager::queries::get_active_filters()
}

// get all evm-logs-canister addresses and topics which are being monitored. Must be unique
// #[query(name = "get_active_addresses_and_topics")]
// #[candid_method(query)]
// fn get_active_addresses_and_topics() -> (Vec<String>, Option<Vec<Vec<String>>>) {
//     subscription_manager::queries::get_active_addresses_and_topics()
// }

// get all evm-logs-canister subscriptions info
#[query(name = "get_subscriptions")]
#[candid_method(query)]
fn get_subscriptions(
    namespace: Option<String>,
    from_id: Option<Nat>,
    filters: Option<Vec<Filter>>,
) -> Vec<SubscriptionInfo> {
    subscription_manager::queries::get_subscriptions_info(namespace, from_id, filters)
}

// only testing purpose
#[update(name = "publish_events")]
#[candid_method(update)]
async fn icrc72_publish(events: Vec<Event>) -> Vec<Option<Result<Vec<Nat>, PublishError>>> {
    subscription_manager::events_publisher::publish_events(events).await
}

#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();
