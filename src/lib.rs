mod chain_service;
mod log_filters;
mod subscription_manager;
mod types;
mod utils;
mod constants;
mod candid_methods;

use std::{
    cell::RefCell,
    rc::Rc,
    time::Duration,
};
use ic_cdk::storage;
use candid::{Nat, Principal};
use chain_service::{service::ChainService, ChainConfig};
use evm_logs_types::*;
use ic_cdk_macros::*;

use crate::{
    log_filters::filter_manager::FilterManager,
    types::state::{State, init as init_state},
    utils::utils::generate_chain_configs,
};

use ic_cdk_macros::query;

thread_local! {
    pub static STATE: RefCell<State> = RefCell::default();

    pub static NEXT_SUBSCRIPTION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    pub static NEXT_NOTIFICATION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));

    pub static TOPICS_MANAGER: RefCell<FilterManager> = RefCell::new(FilterManager::default());

    pub static CHAIN_SERVICES: RefCell<Vec<Rc<ChainService>>> = RefCell::new(Vec::new());
}

#[init]
async fn init(config: types::config::Config) {
    subscription_manager::init();
    init_state(config);

    let monitoring_interval = Duration::from_secs(15);

    let chain_configs = generate_chain_configs();

    let services: Vec<Rc<ChainService>> = chain_configs
        .into_iter()
        .map(|config| {
            let service = Rc::new(ChainService::new(config));
            service.clone().start_monitoring(monitoring_interval);
            service
        })
        .collect();

    CHAIN_SERVICES.with(|chain_services| {
        *chain_services.borrow_mut() = services;
    });

    log!("EVM logs monitoring is started");
}


#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|state| state.borrow().clone());
    let next_subscription_id = NEXT_SUBSCRIPTION_ID.with(|id| id.borrow().clone());
    let next_notification_id = NEXT_NOTIFICATION_ID.with(|id| id.borrow().clone());
    let topics_manager = TOPICS_MANAGER.with(|manager| manager.borrow().clone());

    let chain_configs: Vec<ChainConfig> = CHAIN_SERVICES.with(|chain_services| {
        chain_services.borrow().iter().map(|service| service.config.clone()).collect()
    });

    storage::stable_save((
        state,
        next_subscription_id,
        next_notification_id,
        topics_manager,
        chain_configs,
    ))
    .expect("error during pre_upgrade state saving");

    ic_cdk::println!("pre_upgrade: State saved successfully.");
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let (saved_state, saved_next_subscription_id, saved_next_notification_id, saved_topics_manager, saved_chain_configs): 
        (State, Nat, Nat, FilterManager, Vec<ChainConfig>) = storage::stable_restore().expect("Failed to restore state after upgrade");

    STATE.with(|state| {
        *state.borrow_mut() = saved_state;
    });

    NEXT_SUBSCRIPTION_ID.with(|id| {
        *id.borrow_mut() = saved_next_subscription_id;
    });

    NEXT_NOTIFICATION_ID.with(|id| {
        *id.borrow_mut() = saved_next_notification_id;
    });

    TOPICS_MANAGER.with(|manager| {
        *manager.borrow_mut() = saved_topics_manager;
    });

    let monitoring_interval = Duration::from_secs(15); // TODO

    let restored_services: Vec<Rc<ChainService>> = saved_chain_configs
        .into_iter()
        .map(|config| {
            let service = Rc::new(ChainService::new(config));
            service.clone().start_monitoring(monitoring_interval);
            service
        })
        .collect();

    CHAIN_SERVICES.with(|chain_services| {
        *chain_services.borrow_mut() = restored_services;
    });

    ic_cdk::println!("post_upgrade: State restored successfully.");
}   


#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();
