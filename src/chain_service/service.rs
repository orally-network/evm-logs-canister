use std::{cell::RefCell, rc::Rc};

use candid::Nat;
use ic_cdk_timers::TimerId;

use super::{config::ChainConfig, monitoring::start_monitoring_internal};
use crate::log;

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
    log!("Starting monitoring for chain ID {}", self.config.chain_id);
    start_monitoring_internal(self, interval);
  }

  pub fn stop_monitoring(&self) {
    log!("Stopping monitoring for chain ID {}", self.config.chain_id);
    let timer_id = self.timer_id.borrow_mut().take();
    if let Some(timer_id) = timer_id {
      ic_cdk_timers::clear_timer(timer_id);
    }
  }
}
