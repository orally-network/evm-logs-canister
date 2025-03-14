use std::{rc::Rc, time::Duration};

use ic_cdk;
use ic_cdk_timers::set_timer_interval;

use super::{events_processor::process_and_publish_events, logs_fetcher::fetch_logs, service::ChainService};
use crate::{log_with_metrics, subscription_manager::queries, utils::get_latest_block_number};

pub fn start_monitoring_internal(service: Rc<ChainService>, interval: Duration) {
  let service_clone = Rc::clone(&service);

  let timer_id = set_timer_interval(interval, move || {
    let service_inner = Rc::clone(&service_clone);
    ic_cdk::spawn(async move {
      service_inner.logs_fetching_and_processing_task().await;
    });
  });

  *service.timer_id.borrow_mut() = Some(timer_id);
}

impl ChainService {
  pub async fn logs_fetching_and_processing_task(&self) {
    let (addresses, topics) = queries::get_active_addresses_and_topics(self.config.chain_id);

    if addresses.is_empty() && topics.is_none() {
      log_with_metrics!(
        "Chain {:?} : No active filters to monitor. No fetching",
        self.config.chain_id
      );
      return;
    }

    let last_processed_block = self.last_processed_block.borrow().clone();

    if last_processed_block == 0u32 {
      // Initialize last_processed_block
      match get_latest_block_number(self.config.rpc_providers.clone()).await {
        Ok(latest_block_number) => {
          *self.last_processed_block.borrow_mut() = latest_block_number.clone();
          log_with_metrics!(
            "Initialized last block number to {} for Chain ID {:?}",
            latest_block_number,
            self.config.chain_id
          );
          return;
        }
        Err(err) => {
          log_with_metrics!(
            "Failed to initialize last block number Chain ID {:?}: {}",
            self.config.chain_id,
            err,
          );
          return;
        }
      }
    }
    let from_block = last_processed_block.clone() + 1u32;

    log_with_metrics!(
      "Chain {:?}: Fetching logs from block {} to latest",
      self.config.chain_id,
      from_block
    );

    match fetch_logs(&self.config, from_block.clone(), Some(addresses), topics).await {
      Ok(logs) => {
        if !logs.is_empty() {
          let max_block_number = logs
            .iter()
            .filter_map(|log| log.block_number.as_ref())
            .map(|bn| candid::Nat::from(bn.clone()))
            .max()
            .unwrap_or(last_processed_block);

          *self.last_processed_block.borrow_mut() = max_block_number;
          log_with_metrics!(
            "Last processed block new value: {}",
            *self.last_processed_block.borrow()
          );

          process_and_publish_events(self, logs).await;
        } else {
          *self.last_processed_block.borrow_mut() = from_block.clone();
          log_with_metrics!(
            "{:?}: No new logs found. Advancing to block {}",
            self.config.chain_id,
            from_block
          );
        }
      }
      Err(e) => {
        log_with_metrics!("Error during logs extraction for {:?}: {}", self.config.chain_id, e);
      }
    }
  }
}
