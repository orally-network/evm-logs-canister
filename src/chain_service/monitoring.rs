use std::sync::Arc;
use ic_cdk_timers::set_timer_interval;
use crate::utils::get_latest_block_number;
use crate::subscription_manager::queries;
use ic_cdk;

use super::service::ChainService;
use super::logs_fetcher::fetch_logs;
use super::events_processor::process_events;
use super::utils::{print_logs, convert_log_to_string};
use std::time::Duration;
use num_traits::ToPrimitive;

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
        let (addresses, topics) = queries::get_active_addresses_and_topics();

        if addresses.is_empty() && topics.is_none() {
            ic_cdk::println!("{:?} : No active filters to monitor. No fetching", self.config.chain_name);
            return;
        }

        let last_processed_block = *self.last_processed_block.borrow();

        if last_processed_block == 0 {
            // Initialize last_processed_block
            match get_latest_block_number(
                &self.evm_rpc,
                self.config.rpc_providers.clone(),
            ).await {
                Ok(latest_block_number) => {
                    *self.last_processed_block.borrow_mut() = latest_block_number;
                    ic_cdk::println!(
                        "Initialized last_processed_block to {} for {:?}",
                        latest_block_number,
                        self.config.chain_name
                    );
                    return;
                },
                Err(err) => {
                    ic_cdk::println!(
                        "Failed to initialize last_processed_block for {:?}: {}",
                        self.config.chain_name,
                        err,
                    );
                    return;
                },
            }
        }

        let from_block = last_processed_block + 1;

        ic_cdk::println!(
            "{:?}: Fetching logs from block {} to latest",
            self.config.chain_name,
            from_block
        );

        match fetch_logs(&self.evm_rpc, &self.config.rpc_providers, from_block, Some(addresses), topics).await {
            Ok(logs) => {
                if !logs.is_empty() {
                    let max_block_number = logs
                        .iter()
                        .filter_map(|log| log.blockNumber.as_ref())
                        .map(|bn| bn.0.to_u64().unwrap_or_default())
                        .max()
                        .unwrap_or(last_processed_block);

                    *self.last_processed_block.borrow_mut() = max_block_number;
                    ic_cdk::println!("Last processed block new value: {}", *self.last_processed_block.borrow());

                    let log_strings: Vec<String> = logs.iter().map(|log| convert_log_to_string(&self.config.chain_name, log)).collect();
                    print_logs(&log_strings);

                    if let Err(e) = process_events(self, logs).await {
                        ic_cdk::println!("Error processing events: {}", e);
                    }
                } else {
                    *self.last_processed_block.borrow_mut() = from_block;
                    ic_cdk::println!(
                        "{:?}: No new logs found. Advancing to block {}",
                        self.config.chain_name,
                        from_block
                    );
                }
            },
            Err(e) => {
                ic_cdk::println!(
                    "Error during logs extraction for {:?}: {}",
                    self.config.chain_name,
                    e
                );
            }
        }
    }
}
