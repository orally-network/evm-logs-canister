use std::collections::HashMap;
use std::str::FromStr;

use candid::{Principal, CandidType, Nat};
use serde::{Deserialize, Serialize};
use super::{
    config::Config,
    balances::Balances,
};
use evm_logs_types::SubscriptionInfo;
use crate::STATE;

#[derive(Clone, CandidType, Serialize, Deserialize, Debug)]
pub struct State {
    pub evm_rpc_canister: Principal,
    pub proxy_canister: Principal,
    pub estimate_events_num: u32,
    pub subscriptions: HashMap<Nat, SubscriptionInfo>,
    pub subscribers: HashMap<Principal, Vec<Nat>>,
    pub user_balances: Balances,
    pub max_response_bytes: u32,
    pub test: u32,
}


pub fn init(config: Config) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.evm_rpc_canister = config.evm_rpc_canister;
        state.proxy_canister = config.proxy_canister;
        state.estimate_events_num = config.estimate_events_num;
        state.max_response_bytes = config.max_response_bytes;
    });
}


impl Default for State {
    fn default() -> Self {
        Self {
            evm_rpc_canister: Principal::from_str("aaaaa-aa").expect("Invalid principal"),
            proxy_canister: Principal::from_str("aaaaa-aa").expect("Invalid principal"),
            estimate_events_num: 5,
            subscriptions: HashMap::new(),
            subscribers: HashMap::new(),
            user_balances: Balances::default(),
            max_response_bytes: 1_000_000,
            test: 0,
        }
    }
}