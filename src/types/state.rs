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
    pub user_balances: Balances,
    pub test: u32,
}


pub fn init(config: Config) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.evm_rpc_canister = config.evm_rpc_canister;
        state.proxy_canister = config.proxy_canister;
        state.estimate_events_num = config.estimate_events_num;
    });
}


impl Default for State {
    fn default() -> Self {
        Self {
            evm_rpc_canister: Principal::from_str("aaaaa-aa").expect("Invalid principal"),
            proxy_canister: Principal::from_str("aaaaa-aa").expect("Invalid principal"),
            estimate_events_num: 5,
            subscriptions: HashMap::new(),
            user_balances: Balances::default(),
            test: 0,
        }
    }
}