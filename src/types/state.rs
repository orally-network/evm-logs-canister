use std::str::FromStr;

use candid::CandidType;
use candid::Principal;
use serde::{Deserialize, Serialize};
use super::{
    config::{Config, EventsPerInterval},
    balances::Balances,
};

use crate::STATE;

#[derive(Clone, CandidType, Serialize, Deserialize, Debug)]
pub struct State {
    pub evm_rpc_canister: Principal,
    pub proxy_canister: Principal,
    pub rpc_wrapper: String,
    pub events_per_interval: EventsPerInterval,
    pub user_balances: Balances,
    pub test: u32,
}


pub fn init(config: Config) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.evm_rpc_canister = config.evm_rpc_canister;
        state.proxy_canister = config.proxy_canister;
        state.rpc_wrapper = config.rpc_wrapper.clone();
        state.events_per_interval = config.events_per_interval.clone();
    });
}


impl Default for State {
    fn default() -> Self {
        Self {
            evm_rpc_canister: Principal::from_str("aaaaa-aa").expect("Invalid principal"),
            proxy_canister: Principal::from_str("aaaaa-aa").expect("Invalid principal"),
            rpc_wrapper: "".to_string(),
            events_per_interval: EventsPerInterval{interval: 20, events_num: 5},
            user_balances: Balances::default(),
            test: 0,
        }
    }
}