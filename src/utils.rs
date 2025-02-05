use candid::Nat;
use ic_cdk::api::call::call_with_payment128;
use ic_cdk::api::time;
use metrics::cycles_count;
use std::cell::RefCell;
use evm_rpc_types::{Block, BlockTag, ConsensusStrategy, MultiRpcResult, RpcApi, RpcConfig, RpcResult, RpcServices};
use crate::chain_service::ChainConfig;
use crate::constants::*;

#[macro_export]
macro_rules! get_state_value {
    ($field:ident) => {{
        $crate::STATE.with(|state| state.borrow().$field.clone())
    }};
}

#[macro_export]
macro_rules! update_state {
    ($field:ident, $value:expr) => {{
        $crate::STATE.with(|state| {
            state.borrow_mut().$field = $value;
        })
    }};
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        ic_cdk::println!($($arg)*);
    }};
}

thread_local! {
    static SUB_ID_COUNTER: RefCell<Nat> = RefCell::new(Nat::from(0u32));
}

pub fn current_timestamp() -> u64 {
    time()
}

#[cycles_count]
pub async fn get_latest_block_number(
    rpc_providers: RpcServices,
) -> Result<Nat, String> {
    let cycles = 10_000_000_000;

    let block_tag = BlockTag::Latest;

    let rpc_config = RpcConfig {
        response_size_estimate: None,
        response_consensus: Some(ConsensusStrategy::Threshold { 
            total: Some(3), 
            min: 1
        }),
    };
    let evm_rpc_canister = get_state_value!(evm_rpc_canister);
    log!("calling eth_getBlockByNumber!");
    let (result,): (MultiRpcResult<Block>,) = 
        call_with_payment128(
            evm_rpc_canister, 
            "eth_getBlockByNumber", 
            (rpc_providers, rpc_config, block_tag), 
            cycles,
        )
        .await
        .map_err(|e| format!("Call failed: {:?}", e))?;

    match result {
        MultiRpcResult::Consistent(res) => match res {
            RpcResult::Ok(block) => {
                let block_number = block
                    .number;
                let block_number: Nat = block_number.into();
                Ok(block_number)
            }
            RpcResult::Err(err) => Err(format!("RPC error: {:?}", err)),
        },
        MultiRpcResult::Inconsistent(_) => {
            Err("RPC providers gave inconsistent results".to_string())
        }
    }
}

pub fn generate_chain_configs() -> Vec<ChainConfig> {
    
    let evm_rpc_canister = get_state_value!(evm_rpc_canister);

    vec![
        ChainConfig {
            chain_id: ETHEREUM_CHAIN_ID,
            rpc_providers: get_rpc_providers_for_chain(ETHEREUM_CHAIN_ID),
            evm_rpc_canister,
            rpc_config: Some(RpcConfig {
                response_size_estimate: None,
                response_consensus: Some(ConsensusStrategy::Threshold {
                    total: Some(4),
                    min: 2,
                }),
            }),
        },
        ChainConfig {
            chain_id: BASE_CHAIN_ID,
            rpc_providers: get_rpc_providers_for_chain(BASE_CHAIN_ID),
            evm_rpc_canister,
            rpc_config: Some(RpcConfig {
                response_size_estimate: None,
                response_consensus: Some(ConsensusStrategy::Threshold {
                    total: Some(4),
                    min: 2,
                }),
            }),
        },
        ChainConfig {
            chain_id: OPTIMISM_CHAIN_ID,
            rpc_providers: get_rpc_providers_for_chain(OPTIMISM_CHAIN_ID),
            evm_rpc_canister,
            rpc_config: Some(RpcConfig {
                response_size_estimate: None,
                response_consensus: Some(ConsensusStrategy::Threshold {
                    total: Some(4),
                    min: 2,
                }),
            }),
        },
        ChainConfig {
            chain_id: POLYGON_CHAIN_ID,
            rpc_providers: get_rpc_providers_for_chain(POLYGON_CHAIN_ID),
            evm_rpc_canister,
            rpc_config: Some(RpcConfig {
                response_size_estimate: None,
                response_consensus: Some(ConsensusStrategy::Threshold {
                    total: Some(3),
                    min: 2,
                }),
            }),
        },
        ChainConfig {
            chain_id: ARBITRUM_CHAIN_ID,
            rpc_providers: get_rpc_providers_for_chain(ARBITRUM_CHAIN_ID),
            evm_rpc_canister,
            rpc_config: Some(RpcConfig {
                response_size_estimate: None,
                response_consensus: Some(ConsensusStrategy::Threshold {
                    total: Some(3),
                    min: 2,
                }),
            }),
        },
        ChainConfig {
            chain_id: BSC_CHAIN_ID,
            rpc_providers: get_rpc_providers_for_chain(BSC_CHAIN_ID),
            evm_rpc_canister,
            rpc_config: Some(RpcConfig {
                response_size_estimate: None,
                response_consensus: Some(ConsensusStrategy::Threshold {
                    total: Some(3),
                    min: 2,
                }),
            }),
        },
    ]
}


pub fn get_rpc_providers_for_chain(chain: u32) -> RpcServices {
    match chain {
        1 => {
            RpcServices::EthMainnet(None)
        }
        8453 => {
            RpcServices::BaseMainnet(None)
        }
        10 => {
            RpcServices::OptimismMainnet(None)
        }
        42161 => {
            RpcServices::ArbitrumOne(None)
        },
        137 => {
            RpcServices::Custom {
                chain_id: 137,
                services: vec![
                    RpcApi {
                        url: "https://polygon-rpc.com".to_string(),
                        headers: None,
                    },
                    RpcApi {
                        url: "https://polygon.llamarpc.com".to_string(),
                        headers: None,
                    },
                    RpcApi {
                        url: "https://rpc.ankr.com/polygon".to_string(),
                        headers: None,
                    },
                ],
            }
        },
        56 => {
            RpcServices::Custom {
                chain_id: 56,
                services: vec![
                    RpcApi {
                        url: "https://binance.llamarpc.com".to_string(),
                        headers: None,
                    },
                    RpcApi {
                        url: "https://rpc.ankr.com/bsc".to_string(),
                        headers: None,
                    },
                    RpcApi {
                        url: "https://bscrpc.com".to_string(),
                        headers: None,
                    },
                ],
            }
        }
        _ => unreachable!(),
    }
}
