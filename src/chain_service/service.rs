use crate::get_state_value;

use super::config::ChainConfig;
use super::monitoring::start_monitoring_internal;
use candid::Principal;
use ic_cdk_timers::TimerId;
use std::cell::RefCell;
use std::sync::Arc;
use candid::Nat;

pub struct ChainService {
    pub config: ChainConfig,
    pub evm_rpc_canister: Principal,
    pub last_processed_block: RefCell<Nat>,
    pub timer_id: RefCell<Option<TimerId>>,
}

impl ChainService {
    pub fn new(config: ChainConfig) -> Self {
        let evm_rpc_canister = get_state_value!(evm_rpc_canister);
        let last_processed_block = RefCell::new(Nat::from(0u32));
        let timer_id = RefCell::new(None);

        ChainService {
            config,
            evm_rpc_canister,
            last_processed_block,
            timer_id,
        }
    }

    pub fn start_monitoring(self: Arc<Self>, interval: std::time::Duration) {
        start_monitoring_internal(self, interval);
    }
}
