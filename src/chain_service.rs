use crate::subscription_manager;
use evm_logs_types::{
    PublicationRegistration, Event, RegisterPublicationResult, ICRC16Value,
};
use candid::{CandidType, Deserialize, Nat, Principal};
use ic_cdk::api::call::{call, call_with_payment128};
use ic_cdk::api::time;
use ic_cdk_timers::TimerId;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use std::sync::Arc;
use evm_rpc_canister_types::{
    BlockTag, EthMainnetService, GetLogsArgs, GetLogsResult, MultiGetLogsResult, RpcServices, EvmRpcCanister,
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
    timer_id: RefCell<Option<TimerId>>,
}

impl ChainService {
    pub fn new(config: ChainConfig) -> Self {
        let evm_rpc = EvmRpcCanister(config.evm_rpc_canister);
        let last_checked_time = RefCell::new(time() / 1_000_000);
        let timer_id = RefCell::new(None);

        ChainService {
            config,
            evm_rpc,
            last_checked_time,
            timer_id,
        }
    }

    pub async fn fetch_logs(
        &self,
        from_block: u64,
        to_block: u64,
        address: Option<String>,
    ) -> Result<Vec<String>, String> {
        let get_logs_args = GetLogsArgs {
            fromBlock: Some(BlockTag::Number(Nat::from(from_block))),
            toBlock: Some(BlockTag::Number(Nat::from(to_block))),
            addresses: address.into_iter().collect(),
            topics: None,
        };

        let cycles = 10_000_000_000;
        let (result,) = self
            .evm_rpc
            .eth_get_logs(self.config.rpc_providers.clone(), None, get_logs_args, cycles)
            .await
            .expect("Call failed");

        match result {
            MultiGetLogsResult::Consistent(r) => match r {
                GetLogsResult::Ok(block) => {
                    let log_strings: Vec<String> = block
                        .into_iter()
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
                        .collect();
                    Ok(log_strings)
                }
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
        match self
            .fetch_logs(
                20798658,
                20798660,
                Some("0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".to_string()),
            )
            .await
        {
            Ok(logs) => {
                if !logs.is_empty() {
                    *self.last_checked_time.borrow_mut() = time() / 1_000_000;

                    for log in &logs {
                        ic_cdk::println!("Log: {}", log);
                    }

                    // Prepare publication registration
                    let registration = PublicationRegistration {
                        namespace: format!("com.example.myapp.events.{}", self.config.chain_name),
                        config: vec![],
                        memo: None,
                    };

                    // Register the publication
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
                                return;
                            }
                        }
                    }

                    // Prepare and publish events
                    let events: Vec<Event> = logs
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
                                ic_cdk::println!("Event was not published (no result available).");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                ic_cdk::println!("Error during logs extraction: {}", e);
            }
        }
    }
}
