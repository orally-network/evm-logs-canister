use super::config::ChainConfig;
use super::monitoring::start_monitoring_internal;
use ic_cdk_timers::TimerId;
use std::cell::RefCell;
use std::rc::Rc;
use candid::Nat;

pub struct ChainService {
    pub config: ChainConfig,
    pub last_processed_block: RefCell<Nat>,
    pub timer_id: RefCell<Option<TimerId>>,
}

impl ChainService {
    pub fn new(config: ChainConfig) -> Self {
        let last_processed_block = RefCell::new(Nat::from(0u32));
        let timer_id = RefCell::new(None);

        ChainService {
            config,
            last_processed_block,
            timer_id,
        }
    }

    pub fn start_monitoring(self: Rc<Self>, interval: std::time::Duration) {
        start_monitoring_internal(self, interval);
    }
}
