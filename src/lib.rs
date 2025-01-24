// lib.rs

mod chain_service;
mod log_filters;
mod subscription_manager;
mod utils;
mod types;
mod candid_methods;
mod constants;

use ic_cdk_macros::*;

use candid::Nat;
use chain_service::{service::ChainService, ChainConfig};
use ic_cdk_macros::query;
use utils::get_rpc_providers_for_chain;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;
use crate::types::state::State;
use evm_logs_types::*;

use crate::log_filters::filter_manager::FilterManager;
use candid::Principal;
use std::collections::HashMap;

use evm_logs_types::{Event, SubscriptionInfo};
use constants::*;

thread_local! {
    pub static STATE: RefCell<State> = RefCell::default();

    pub static CHAIN_SERVICES: RefCell<Vec<Arc<ChainService>>> = const { RefCell::new(Vec::new()) };
    
    pub static SUBSCRIPTIONS: RefCell<HashMap<Nat, SubscriptionInfo>> = RefCell::new(HashMap::new());
    pub static SUBSCRIBERS: RefCell<HashMap<Principal, Vec<Nat>>> = RefCell::new(HashMap::new());
    pub static EVENTS: RefCell<HashMap<Nat, Event>> = RefCell::new(HashMap::new());

    pub static NEXT_SUBSCRIPTION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    pub static NEXT_EVENT_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    pub static NEXT_NOTIFICATION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));

    pub static TOPICS_MANAGER: RefCell<FilterManager> = RefCell::new(FilterManager::new());

}

#[init]
async fn init(config: types::config::Config) {
    subscription_manager::init();
    crate::types::state::init(config);

    let monitoring_interval = Duration::from_secs(15);

    let chain_configs = vec![
        ChainConfig {
            chain_id: ETHEREUM_CHAIN_ID,
            rpc_providers: get_rpc_providers_for_chain(ETHEREUM_CHAIN_ID),
            evm_rpc_canister: get_state_value!(evm_rpc_canister),
        },
        ChainConfig {
            chain_id: BASE_CHAIN_ID,
            rpc_providers: get_rpc_providers_for_chain(BASE_CHAIN_ID),
            evm_rpc_canister: get_state_value!(evm_rpc_canister),
        },
        ChainConfig {
            chain_id: OPTIMISM_CHAIN_ID,
            rpc_providers: get_rpc_providers_for_chain(OPTIMISM_CHAIN_ID),
            evm_rpc_canister: get_state_value!(evm_rpc_canister),
        },
        ChainConfig {
            chain_id: POLYGON_CHAIN_ID,
            rpc_providers: get_rpc_providers_for_chain(POLYGON_CHAIN_ID),
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


#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();
