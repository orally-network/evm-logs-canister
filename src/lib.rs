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
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;
use crate::types::state::State;
use evm_logs_types::*;

use crate::log_filters::filter_manager::FilterManager;
use crate::utils::generate_chain_configs;

use candid::Principal;
use std::collections::HashMap;

use evm_logs_types::{Event, SubscriptionInfo};

thread_local! {
    pub static STATE: RefCell<State> = RefCell::default();

    pub static SUBSCRIPTIONS: RefCell<HashMap<Nat, SubscriptionInfo>> = RefCell::new(HashMap::new());
    pub static SUBSCRIBERS: RefCell<HashMap<Principal, Vec<Nat>>> = RefCell::new(HashMap::new());

    pub static NEXT_SUBSCRIPTION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    pub static NEXT_NOTIFICATION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));

    pub static TOPICS_MANAGER: RefCell<FilterManager> = RefCell::new(FilterManager::new());
}

#[init]
async fn init(config: types::config::Config) {
    subscription_manager::init();
    crate::types::state::init(config);

    let monitoring_interval = Duration::from_secs(15);

    let chain_configs = generate_chain_configs();

    chain_configs
        .into_iter()
        .for_each(|config| init_chain_service(config, monitoring_interval));

    log!("EVM logs monitoring is started");
}

fn init_chain_service(config: ChainConfig, monitoring_interval: Duration) {
    let service = Arc::new(ChainService::new(config));
    service.clone().start_monitoring(monitoring_interval);
}


#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();
