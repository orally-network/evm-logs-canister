use crate::subscription_manager;
use crate::subscription_manager::Filter;
use crate::utils::get_latest_block_number;
use evm_logs_types::{
    PublicationRegistration, Event, RegisterPublicationResult, ICRC16Value,
};
use num_traits::ToPrimitive;
use candid::{Nat, Principal};
use ic_cdk::api::time;
use ic_cdk_timers::TimerId;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;
use evm_rpc_canister_types::{
    BlockTag, GetLogsArgs, GetLogsResult, MultiGetLogsResult, RpcServices, EvmRpcCanister, LogEntry,
};

#[derive(Clone)]
pub struct ChainConfig {
    pub chain_name: String,
    pub rpc_providers: RpcServices,
    pub evm_rpc_canister: Principal,
}

pub struct ChainService {
    config: ChainConfig,
    evm_rpc: EvmRpcCanister,
    last_checked_time: RefCell<u64>,
    last_processed_block: RefCell<u64>,
    timer_id: RefCell<Option<TimerId>>,
}

impl ChainService {
    pub fn new(config: ChainConfig) -> Self {
        let evm_rpc = EvmRpcCanister(config.evm_rpc_canister);
        let last_checked_time = RefCell::new(time() / 1_000_000);
        let last_processed_block = RefCell::new(0);
        let timer_id = RefCell::new(None);

        ChainService {
            config,
            evm_rpc,
            last_checked_time,
            last_processed_block,
            timer_id,
        }
    }

    pub async fn fetch_logs(
        &self,
        from_block: u64,
        addresses: Option<Vec<String>>,
        topics: Option<Vec<Vec<String>>>,
    ) -> Result<Vec<LogEntry>, String> {
        let get_logs_args = GetLogsArgs {
            fromBlock: Some(BlockTag::Number(Nat::from(from_block))),
            toBlock: Some(BlockTag::Latest),
            addresses: addresses.unwrap_or_default(),
            topics,
        };

        let cycles = 10_000_000_000;
        let (result,) = self
            .evm_rpc
            .eth_get_logs(
                self.config.rpc_providers.clone(),
                None,
                get_logs_args,
                cycles,
            )
            .await
            .map_err(|e| format!("Call failed: {:?}", e))?;

        match result {
            MultiGetLogsResult::Consistent(r) => match r {
                GetLogsResult::Ok(logs) => Ok(logs),
                GetLogsResult::Err(err) => Err(format!("{:?}", err)),
            },
            MultiGetLogsResult::Inconsistent(_) => {
                Err("RPC providers gave inconsistent results".to_string())
            }
        }
    }

    pub fn start_monitoring(self: Arc<Self>, interval: Duration) {
        let self_clone = Arc::clone(&self);
        let timer_id = ic_cdk_timers::set_timer_interval(interval, move || {
            let self_clone_inner = Arc::clone(&self_clone);
            ic_cdk::spawn(async move {
                self_clone_inner.fetch_logs_and_update_time().await;
            });
        });

        *self.timer_id.borrow_mut() = Some(timer_id);
    }

    async fn fetch_logs_and_update_time(&self) {

        // Get filters from subscriptions
        let filters = subscription_manager::get_active_filters();

        if filters.is_empty() {
            ic_cdk::println!("{} : No active filters to monitor. No fetching", self.config.chain_name);
            return;
        }


        let mut last_processed_block = *self.last_processed_block.borrow();

        if last_processed_block == 0 {
            // Initialize last_processed_block to the latest block number
            match get_latest_block_number(
                &self.evm_rpc,
                self.config.rpc_providers.clone(),
            )
            .await
            {
                Ok(latest_block_number) => {
                    last_processed_block = latest_block_number;
                    *self.last_processed_block.borrow_mut() = latest_block_number;
                    ic_cdk::println!(
                        "Initialized last_processed_block to {} for {}",
                        latest_block_number,
                        self.config.chain_name
                    );
                },
                Err(err) => {
                    ic_cdk::println!(
                        "Failed to initialize last_processed_block for {}: {}",
                        self.config.chain_name,
                        err,
                    );
                    return;
                },
            }
        }

        let from_block = last_processed_block + 1;

        ic_cdk::println!(
            "{}: Fetching logs from block {} to latest",
            self.config.chain_name,
            from_block
        );

        // Combine filters into addresses and topics
        let (addresses, topics) = self.combine_filters(filters);

        // Fetch logs with combined filters
        match self
            .fetch_logs(from_block, Some(addresses), topics)
            .await
        {
            Ok(logs) => {
                if !logs.is_empty() {
                    // Find the maximum block number from the logs
                    let max_block_number = logs
                        .iter()
                        .filter_map(|log| log.blockNumber.as_ref())
                        .filter_map(|block_number| block_number.0.to_u64())
                        .max()
                        .unwrap_or(last_processed_block);


                    *self.last_processed_block.borrow_mut() = max_block_number;

                    ic_cdk::println!("Last processed block new value: {}", *self.last_processed_block.borrow());

                    *self.last_checked_time.borrow_mut() = time() / 1_000_000;

                    let log_strings = self.convert_logs_to_strings(&logs);

                    self.print_logs(&log_strings);

                    if let Err(e) = self.process_events(log_strings).await {
                        ic_cdk::println!("Error processing events: {}", e);
                    }
                } else {
                    // No logs found; increment last_processed_block by 1
                    *self.last_processed_block.borrow_mut() = from_block;
                    ic_cdk::println!(
                        "{}: No new logs found. Advancing to block {}",
                        self.config.chain_name,
                        from_block
                    );
                }
            }
            Err(e) => {
                ic_cdk::println!(
                    "Error during logs extraction for {}: {}",
                    self.config.chain_name,
                    e
                );
            }
        }
    }

    fn print_logs(&self, logs: &Vec<String>) {
        for log in logs {
            ic_cdk::println!("Log: {:?}", log);
        }
    }

    fn convert_logs_to_strings(&self, logs: &Vec<LogEntry>) -> Vec<String> {
        logs.iter()
            .map(|log_entry| {
                format!(
                    "Chain: {}, Address: {}, TxHash: {:?}, Block: {:?}, Topics: {:?}, Data: {}",
                    self.config.chain_name,
                    log_entry.address,
                    log_entry.transactionHash,
                    log_entry
                        .blockNumber
                        .as_ref()
                        .map(|n| n.0.clone())
                        .unwrap_or_default(),
                    log_entry.topics,
                    log_entry.data
                )
            })
            .collect()
    }

    async fn process_events(&self, log_strings: Vec<String>) -> Result<(), String> {
        let registration = PublicationRegistration {
            namespace: format!("com.example.myapp.events.{}", self.config.chain_name),
            config: vec![],
            memo: None,
        };

        let registration_result =
            subscription_manager::register_publication(vec![registration]).await;

        for result in registration_result {
            match result {
                RegisterPublicationResult::Ok(pub_id) => {
                    ic_cdk::println!(
                        "Successfully registered publication with ID: {:?}",
                        pub_id
                    );
                }
                _ => {
                    ic_cdk::println!("Failed to register publication.");
                    return Err("Failed to register publication.".to_string());
                }
            }
        }

        let events: Vec<Event> = log_strings
            .iter()
            .enumerate()
            .map(|(index, log)| Event {
                id: Nat::from(index as u64 + 1),
                prev_id: None,
                timestamp: time() / 1_000_000,
                namespace: format!("com.example.myapp.events.{}", self.config.chain_name),
                data: ICRC16Value::Text(log.clone()),
                headers: None,
            })
            .collect();

        let publish_result = subscription_manager::publish_events(events).await;

        for opt_result in publish_result {
            match opt_result {
                Some(Ok(event_ids)) => {
                    ic_cdk::println!(
                        "Event published and sent to subscribers with Event IDs: {:?}",
                        event_ids
                    );
                }
                Some(Err(error)) => {
                    ic_cdk::println!("Failed to publish or send event: {:?}", error);
                }
                None => {
                    ic_cdk::println!("Event was not published (no result available).");
                }
            }
        }
        Ok(())
    }

    fn combine_filters(&self, filters: Vec<Filter>) -> (Vec<String>, Option<Vec<Vec<String>>>) {
        let mut addresses = Vec::new();
        let mut topics = Vec::new();

        for filter in filters {
            addresses.extend(filter.addresses);
            if let Some(filter_topics) = filter.topics {
                topics.extend(filter_topics);
            }
        }

        let topics = if topics.is_empty() { None } else { Some(topics) };
        (addresses, topics)
    }
}
