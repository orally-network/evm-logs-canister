use super::config::ChainConfig;
use super::monitoring::start_monitoring_internal;
use evm_rpc_canister_types::EvmRpcCanister;
use std::cell::RefCell;
use std::sync::Arc;
use ic_cdk_timers::TimerId;

pub struct ChainService {
    pub config: ChainConfig,
    pub evm_rpc: EvmRpcCanister,
    pub last_processed_block: RefCell<u64>,
    pub timer_id: RefCell<Option<TimerId>>,
}

impl ChainService {
    pub fn new(config: ChainConfig) -> Self {
        let evm_rpc = EvmRpcCanister(config.evm_rpc_canister);
        let last_processed_block = RefCell::new(0);
        let timer_id = RefCell::new(None);

        ChainService {
            config,
            evm_rpc,
            last_processed_block,
            timer_id,
        }
    }

    pub fn start_monitoring(self: Arc<Self>, interval: std::time::Duration) {
        start_monitoring_internal(self, interval);
    }
}
