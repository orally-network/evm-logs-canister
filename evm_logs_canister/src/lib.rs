mod chain_service;
mod constants;
mod internals;
mod lifecycle;
mod log_filters;
mod subscription_manager;
mod types;

use std::{cell::RefCell, rc::Rc};

use candid::{Nat, Principal};
use chain_service::service::ChainService;
use evm_logs_types::*;
use ic_utils::api_type::{GetInformationRequest, GetInformationResponse, UpdateInformationRequest};

use crate::{log_filters::filter_manager::FilterManager, types::state::State};

thread_local! {
    pub static STATE: RefCell<State> = RefCell::default();
    pub static NEXT_SUBSCRIPTION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    pub static NEXT_NOTIFICATION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    pub static FILTERS_MANAGER: RefCell<FilterManager> = RefCell::new(FilterManager::default());
    pub static CHAIN_SERVICES: RefCell<Vec<Rc<ChainService>>> = const {RefCell::new(Vec::new())};
}

ic_cdk::export_candid!();
