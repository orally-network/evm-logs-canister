use candid::{Nat, Principal};
use evm_logs_types::LogEntry;
use evm_rpc_client::{EvmRpcClient, IcRuntime};
use evm_rpc_types::{
    GetLogsArgs, BlockTag, RpcServices, EthMainnetService, EthSepoliaService, 
    MultiRpcResult, RpcResult, RpcError, ConsensusStrategy, RpcConfig,
};
use ic_canister_log::{log, Sink};
use futures::future::join_all;
use std::collections::HashMap;

use crate::state::{read_state, mutate_state};
use crate::types::RpcServiceConfig;

const BASE_STRUCT_SIZE: usize = 8;
const MAX_RETRIES: usize = 2;
const MIN_ATTACHED_CYCLES: u128 = 10_000_000_000; // 10B cycles
const ETH_ADDRESS_SIZE: usize = 20;
const ETH_TOPIC_SIZE: usize = 32;
const CYCLES_PER_BYTE_SEND: u64 = 1000;
const CYCLES_PER_BYTE_RECEIVE: u64 = 2000;
const BASE_CALL_CYCLES: u64 = 1_000_000;

// Logger implementation for EVM RPC client
#[derive(Debug, Clone)]
pub struct ChainServiceLogger;

impl Sink for ChainServiceLogger {
    fn append(&self, entry: ic_canister_log::LogEntry) {
        ic_cdk::println!("EVM_RPC {}:{} {}", entry.file, entry.line, entry.message);
    }
}

fn estimate_log_entry_size(logs: &[LogEntry]) -> usize {
    logs.iter().map(|log| {
        // Estimate size based on log entry components
        ETH_ADDRESS_SIZE + // address
        log.topics.len() * ETH_TOPIC_SIZE + // topics
        256 + // estimated data size (since we can't get exact length easily)
        32 // other fields (block_number, tx_hash, etc.)
    }).sum()
}

fn estimate_cycles_used(
    logs_received: &[LogEntry],
    addresses_count: usize,
    topics_count: Option<&Vec<Vec<String>>>,
) -> u64 {
    // Estimate request size
    let request_size_bytes = BASE_STRUCT_SIZE
        + (ETH_ADDRESS_SIZE * addresses_count) // Address bytes
        + topics_count.map_or(0, |t| t.iter().map(|x| ETH_TOPIC_SIZE * x.len()).sum()); // Topics bytes

    // Estimate response size based on received logs
    let response_size_bytes = estimate_log_entry_size(logs_received) as u64;

    // Compute cycles for sending request and receiving response
    let cycles_for_request = request_size_bytes as u64 * CYCLES_PER_BYTE_SEND;
    let cycles_for_response = response_size_bytes * CYCLES_PER_BYTE_RECEIVE;

    // Total cycles usage including base call cost and multiple RPC queries
    BASE_CALL_CYCLES + cycles_for_request + cycles_for_response + (cycles_for_request + cycles_for_response)
}


fn calculate_request_chunk_size(events_per_interval: u32, addresses_len: u32) -> usize {
    // Simple chunking strategy - can be improved
    let base_chunk_size = 50;
    if addresses_len > 100 {
        base_chunk_size / 2
    } else {
        base_chunk_size
    }
}

fn rpc_service_config_to_rpc_services(config: &RpcServiceConfig) -> Result<RpcServices, String> {
    match config {
        RpcServiceConfig::EthMainnet { providers } => {
            let services = match providers {
                Some(provider_names) => {
                    let mut services = Vec::new();
                    for provider in provider_names {
                        match provider.as_str() {
                            "Alchemy" => services.push(EthMainnetService::Alchemy),
                            "Ankr" => services.push(EthMainnetService::Ankr),
                            "Cloudflare" => services.push(EthMainnetService::Cloudflare),
                            "PublicNode" => services.push(EthMainnetService::PublicNode),
                            "BlockPi" => services.push(EthMainnetService::BlockPi),
                            _ => return Err(format!("Unknown EthMainnet provider: {}", provider)),
                        }
                    }
                    Some(services)
                },
                None => None,
            };
            Ok(RpcServices::EthMainnet(services))
        },
        RpcServiceConfig::EthSepolia { providers } => {
            let services = match providers {
                Some(provider_names) => {
                    let mut services = Vec::new();
                    for provider in provider_names {
                        match provider.as_str() {
                            "Alchemy" => services.push(EthSepoliaService::Alchemy),
                            "Ankr" => services.push(EthSepoliaService::Ankr),
                            "BlockPi" => services.push(EthSepoliaService::BlockPi),
                            "PublicNode" => services.push(EthSepoliaService::PublicNode),
                            _ => return Err(format!("Unknown EthSepolia provider: {}", provider)),
                        }
                    }
                    Some(services)
                },
                None => None,
            };
            Ok(RpcServices::EthSepolia(services))
        },
        RpcServiceConfig::ArbitrumOne { providers: _ } => {
            // For now, use default Arbitrum providers
            Ok(RpcServices::ArbitrumOne(None))
        },
        RpcServiceConfig::BaseMainnet { providers: _ } => {
            // For now, use default Base providers
            Ok(RpcServices::BaseMainnet(None))
        },
        RpcServiceConfig::OptimismMainnet { providers: _ } => {
            // For now, use default Optimism providers
            Ok(RpcServices::OptimismMainnet(None))
        },
        RpcServiceConfig::Custom { rpc_url, chain_id } => {
            // For custom RPC, we'll use EthMainnet as a fallback
            // In a real implementation, you might want to add support for custom RPC services
            ic_cdk::println!("Custom RPC not fully supported yet, using EthMainnet as fallback. URL: {}, Chain ID: {}", rpc_url, chain_id);
            Ok(RpcServices::EthMainnet(None))
        },
    }
}

pub async fn fetch_logs(
    from_block: Nat,
    addresses: Option<Vec<String>>,
    topics: Option<Vec<Vec<String>>>,
) -> Result<Vec<LogEntry>, String> {
    let addresses = addresses.unwrap_or_default();

    if addresses.is_empty() {
        return eth_get_logs_call_with_retry(from_block.clone(), None, topics.clone()).await;
    }

    // Get events per interval estimate from state
    let events_per_interval = read_state(|_state| {
        // For now, use a default value - this should be configurable
        100u32
    });

    let chunk_size = calculate_request_chunk_size(events_per_interval, addresses.len() as u32);
    let chunks_iter = addresses.chunks(chunk_size);
    let mut futures = vec![];

    for chunk in chunks_iter {
        let chunk_vec = chunk.to_vec();
        let topics_clone = topics.clone();
        let from_block = from_block.clone();

        let fut = async move {
            eth_get_logs_call_with_retry(from_block.clone(), Some(chunk_vec), topics_clone).await
        };
        futures.push(fut);
    }

    let results = join_all(futures).await;

    let mut merged_logs = Vec::new();
    for res in results {
        match res {
            Ok(logs) => merged_logs.extend(logs),
            Err(e) => return Err(e),
        }
    }

    Ok(merged_logs)
}

async fn eth_get_logs_call_with_retry(
    from_block: Nat,
    addresses: Option<Vec<String>>,
    topics: Option<Vec<Vec<String>>>,
) -> Result<Vec<LogEntry>, String> {
    let addresses = addresses.unwrap_or_default();

    // Get configuration from state
    let (evm_rpc_canister_id, rpc_service_config) = read_state(|state| {
        let evm_rpc_canister_id = state.config.evm_rpc_canister_id.unwrap_or_else(|| {
            // Default EVM RPC canister ID on IC mainnet
            Principal::from_text("7hfb6-caaaa-aaaar-qadga-cai").unwrap()
        });
        (evm_rpc_canister_id, state.config.rpc_service.clone())
    });

    // Convert RPC service config to RpcServices
    let rpc_services = rpc_service_config_to_rpc_services(&rpc_service_config)?;

    // Create EVM RPC client
    let evm_client = EvmRpcClient::builder(IcRuntime {}, ChainServiceLogger)
        .with_providers(rpc_services)
        .with_evm_canister_id(evm_rpc_canister_id)
        .with_min_attached_cycles(MIN_ATTACHED_CYCLES)
        .with_max_num_retries(MAX_RETRIES as u32)
        .build();

    // Convert block number to BlockTag
    let from_block_tag = if from_block == Nat::from(0u32) {
        BlockTag::Latest
    } else {
        // Convert Nat to u64 for BlockTag::Number
        let block_num: u64 = from_block.0.to_u64_digits().first().copied().unwrap_or(0);
        BlockTag::Number(block_num.into())
    };

    // Convert string addresses to Hex20 format expected by EVM RPC
    let hex_addresses: Vec<evm_rpc_types::Hex20> = addresses.iter()
        .filter_map(|addr_str| {
            if addr_str.starts_with("0x") && addr_str.len() == 42 {
                use std::str::FromStr;
                evm_rpc_types::Hex20::from_str(addr_str).ok()
            } else {
                None
            }
        })
        .collect();

    // Convert string topics to Hex32 format expected by EVM RPC
    let hex_topics: Option<Vec<Vec<evm_rpc_types::Hex32>>> = topics.map(|topic_vecs| {
        topic_vecs.iter()
            .map(|topic_vec| {
                topic_vec.iter()
                    .filter_map(|topic_str| {
                        if topic_str.starts_with("0x") && topic_str.len() == 66 {
                            use std::str::FromStr;
                            evm_rpc_types::Hex32::from_str(topic_str).ok()
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .collect()
    });

    // Prepare arguments for the RPC call
    let get_logs_args = GetLogsArgs {
        from_block: Some(from_block_tag),
        to_block: Some(BlockTag::Latest),
        addresses: hex_addresses,
        topics: hex_topics,
    };

    // Retry logic with EVM RPC client
    for attempt in 1..=MAX_RETRIES {
        ic_cdk::println!("calling eth_getLogs, attempt {}", attempt);
        
        match evm_client.eth_get_logs(get_logs_args.clone()).await {
            MultiRpcResult::Consistent(result) => {
                return match result {
                    RpcResult::Ok(logs) => {
                        ic_cdk::println!("Successfully fetched {} logs", logs.len());
                        Ok(logs)
                    },
                    RpcResult::Err(err) => {
                        let error_msg = format!("RPC error: {:?}", err);
                        ic_cdk::println!("{}", error_msg);
                        if attempt == MAX_RETRIES {
                            Err(error_msg)
                        } else {
                            continue;
                        }
                    }
                };
            }
            MultiRpcResult::Inconsistent(results) => {
                ic_cdk::println!("Inconsistent RPC results: {:?}", results);
                if attempt == MAX_RETRIES {
                    return Err("RPC providers gave inconsistent results".to_string());
                }
            }
        }
    }
    
    Err("Failed to get logs after retries.".to_string())
}
