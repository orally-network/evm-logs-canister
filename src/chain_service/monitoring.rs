use crate::{
    subscription_manager::queries,
    utils::get_latest_block_number,
    log,
};
use ic_cdk;
use ic_cdk_timers::set_timer_interval;
use std::sync::Arc;

use super::events_processor::process_and_publish_events;
use super::logs_fetcher::fetch_logs;
use super::service::ChainService;
use std::time::Duration;

pub fn start_monitoring_internal(service: Arc<ChainService>, interval: Duration) {
    let service_clone = Arc::clone(&service);

    let timer_id = set_timer_interval(interval, move || {
        let service_inner = Arc::clone(&service_clone);
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
            log!(
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
                    log!(
                        "Initialized last_processed_block to {} for {:?}",
                        latest_block_number,
                        self.config.chain_id
                    );
                    return;
                }
                Err(err) => {
                    log!(
                        "Failed to initialize last_processed_block for {:?}: {}",
                        self.config.chain_id,
                        err,
                    );
                    return;
                }
            }
        }

        let from_block = last_processed_block.clone() + 1u32;

        log!(
            "Chain {:?}: Fetching logs from block {} to latest",
            self.config.chain_id,
            from_block
        );

        match fetch_logs(
            self.evm_rpc_canister,
            &self.config.rpc_providers,
            from_block.clone(),
            Some(addresses),
            topics,
        )
        .await
        {
            Ok(logs) => {
                if !logs.is_empty() {
                    let max_block_number = logs
                        .iter()
                        .filter_map(|log| log.block_number.as_ref())
                        .map(|bn| candid::Nat::from(bn.clone())) 
                        .max()
                        .unwrap_or(last_processed_block);

                    *self.last_processed_block.borrow_mut() = max_block_number;
                    log!(
                        "Last processed block new value: {}",
                        *self.last_processed_block.borrow()
                    );

                    process_and_publish_events(self, logs).await;

                } 
                else {
                    *self.last_processed_block.borrow_mut() = from_block.clone();
                    log!(
                        "{:?}: No new logs found. Advancing to block {}",
                        self.config.chain_id,
                        from_block
                    );
                }
            }
            Err(e) => {
                log!(
                    "Error during logs extraction for {:?}: {}",
                    self.config.chain_id,
                    e
                );
            }
        }
    }
}
