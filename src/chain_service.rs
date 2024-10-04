use crate::subscription_manager;
use evm_logs_types::{
    PublicationRegistration, Event, RegisterPublicationResult, ICRC16Value,
};
use candid::{Nat, Principal};
use ic_cdk::api::time;
use ic_cdk_timers::TimerId;
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;
use evm_rpc_canister_types::{
    BlockTag, GetLogsArgs, GetLogsResult, MultiGetLogsResult, RpcServices, EvmRpcCanister, LogEntry,
};

use crate::utils::get_latest_block_number;

#[derive(Clone)]
pub struct ChainConfig {
    pub chain_name: String,
    pub rpc_providers: RpcServices,
    pub evm_rpc_canister: Principal,
    pub addresses: Vec<String>,
    pub topics: Option<Vec<Vec<String>>>,
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
        to_block: u64,
        addresses: Option<Vec<String>>,
        topics: Option<Vec<Vec<String>>>,
    ) -> Result<Vec<LogEntry>, String> {
        let get_logs_args = GetLogsArgs {
            fromBlock: Some(BlockTag::Number(Nat::from(from_block))),
            toBlock: Some(BlockTag::Number(Nat::from(to_block))),
            addresses: addresses.unwrap_or_default(),
            topics,
        };

        let cycles = 10_000_000_000;
        let (result,) = self
            .evm_rpc
            .eth_get_logs(self.config.rpc_providers.clone(), None, get_logs_args, cycles)
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
        match get_latest_block_number(&self.evm_rpc, self.config.rpc_providers.clone()).await {
            Ok(latest_block_number) => {
                let mut last_processed_block = *self.last_processed_block.borrow();
                ic_cdk::println!(
                    "Last processed block number for {} : {:?}",
                    self.config.chain_name,
                    self.last_processed_block
                );
                ic_cdk::println!(
                    "Last actual block number for {} : {}",
                    self.config.chain_name,
                    latest_block_number
                );

                if last_processed_block == 0 {
                    last_processed_block = latest_block_number;
                    *self.last_processed_block.borrow_mut() = latest_block_number;
                    return; 
                }

                if last_processed_block >= latest_block_number {
                    return;
                }

                let from_block = last_processed_block + 1;
                let to_block = latest_block_number;

                ic_cdk::println!(
                    "{} : Getting logs FROM {} block number TO {} block number",
                    self.config.chain_name,
                    from_block,
                    to_block
                );

                match self
                    .fetch_logs(
                        from_block,
                        to_block,
                        Some(self.config.addresses.clone()),
                        self.config.topics.clone(),
                    )
                    .await
                {
                    Ok(logs) => {
                        *self.last_processed_block.borrow_mut() = latest_block_number;

                        if !logs.is_empty() {
                            *self.last_checked_time.borrow_mut() = time() / 1_000_000;

                            let log_strings = self.convert_logs_to_strings(&logs);

                            self.print_logs(&log_strings);

                            if let Err(e) = self.process_events(log_strings).await {
                                ic_cdk::println!("Error processing events: {}", e);
                            }
                        } else {
                            ic_cdk::println!("No new logs found.");
                        }
                    }
                    Err(e) => {
                        ic_cdk::println!("Error during logs extraction: {}", e);
                    }
                }
            }
            Err(e) => {
                ic_cdk::println!("Error fetching latest block number: {}", e);
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
                    log_entry.blockNumber,
                    log_entry.topics,
                    log_entry.data
                )
            })
            .collect()
    }

    async fn process_events(&self, log_strings: Vec<String>) -> Result<(), String> {
        let registration = PublicationRegistration {
            namespace: format!(
                "com.example.myapp.events.{}",
                self.config.chain_name
            ),
            config: vec![],
            memo: None,
        };

        let registration_result = subscription_manager::register_publication(vec![registration]).await;

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
                namespace: format!(
                    "com.example.myapp.events.{}",
                    self.config.chain_name
                ),
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
                    ic_cdk::println!(
                        "Event was not published (no result available)."
                    );
                }
            }
        }
        Ok(())
    }
}
