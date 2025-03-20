use std::cell::RefCell;

use candid::Nat;
use evm_rpc_types::{Block, BlockTag, ConsensusStrategy, MultiRpcResult, RpcApi, RpcConfig, RpcResult, RpcServices};
use ic_cdk::api::{call::call_with_payment128, time};

use crate::{
  chain_service::ChainConfig,
  constants::{
    ARBITRUM_CHAIN_ID, BASE_CHAIN_ID, BSC_CHAIN_ID, ETHEREUM_CHAIN_ID, EVM_EVENT_SIZE_BYTES, OPTIMISM_CHAIN_ID,
    POLYGON_CHAIN_ID,
  },
  get_state_value,
};

thread_local! {
    static SUB_ID_COUNTER: RefCell<Nat> = RefCell::new(Nat::from(0u32));
}

pub const DEFAULT_MONITORING_TIME: u64 = 15;

pub fn timestamp_nanos() -> u64 {
  time()
}

pub fn timestamp_millis() -> u64 {
  timestamp_nanos() / 1_000_000
}

pub async fn get_latest_block_number(rpc_providers: RpcServices) -> Result<Nat, String> {
  let cycles = 10_000_000_000; // TODO

  let block_tag = BlockTag::Latest;

  let rpc_config = RpcConfig {
    response_size_estimate: None,
    response_consensus: Some(ConsensusStrategy::Threshold { total: Some(3), min: 1 }),
  };
  let evm_rpc_canister = get_state_value!(evm_rpc_canister);

  let (result,): (MultiRpcResult<Block>,) = call_with_payment128(
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
        let block_number = block.number;
        let block_number: Nat = block_number.into();
        Ok(block_number)
      }
      RpcResult::Err(err) => Err(format!("RPC error: {:?}", err)),
    },
    MultiRpcResult::Inconsistent(_) => Err("RPC providers gave inconsistent results".to_string()),
  }
}

pub fn generate_chain_configs() -> Vec<ChainConfig> {
  let evm_rpc_canister = get_state_value!(evm_rpc_canister);
  let estimate_events_num = get_state_value!(estimate_events_num);
  let response_size_estimate = (estimate_events_num * EVM_EVENT_SIZE_BYTES) as u64;

  vec![
    ChainConfig {
      chain_id: ETHEREUM_CHAIN_ID,
      rpc_providers: get_rpc_providers_for_chain(ETHEREUM_CHAIN_ID),
      evm_rpc_canister,
      rpc_config: Some(RpcConfig {
        response_size_estimate: Some(response_size_estimate),
        response_consensus: Some(ConsensusStrategy::Threshold { total: Some(4), min: 1 }),
      }),
      monitoring_interval: DEFAULT_MONITORING_TIME,
    },
    ChainConfig {
      chain_id: BASE_CHAIN_ID,
      rpc_providers: get_rpc_providers_for_chain(BASE_CHAIN_ID),
      evm_rpc_canister,
      rpc_config: Some(RpcConfig {
        response_size_estimate: Some(response_size_estimate),
        response_consensus: Some(ConsensusStrategy::Threshold { total: Some(4), min: 1 }),
      }),
      monitoring_interval: DEFAULT_MONITORING_TIME,
    },
    ChainConfig {
      chain_id: OPTIMISM_CHAIN_ID,
      rpc_providers: get_rpc_providers_for_chain(OPTIMISM_CHAIN_ID),
      evm_rpc_canister,
      rpc_config: Some(RpcConfig {
        response_size_estimate: Some(response_size_estimate),
        response_consensus: Some(ConsensusStrategy::Threshold { total: Some(4), min: 1 }),
      }),
      monitoring_interval: DEFAULT_MONITORING_TIME,
    },
    ChainConfig {
      chain_id: POLYGON_CHAIN_ID,
      rpc_providers: get_rpc_providers_for_chain(POLYGON_CHAIN_ID),
      evm_rpc_canister,
      rpc_config: Some(RpcConfig {
        response_size_estimate: Some(response_size_estimate),
        response_consensus: Some(ConsensusStrategy::Threshold { total: Some(3), min: 1 }),
      }),
      monitoring_interval: DEFAULT_MONITORING_TIME,
    },
    ChainConfig {
      chain_id: ARBITRUM_CHAIN_ID,
      rpc_providers: get_rpc_providers_for_chain(ARBITRUM_CHAIN_ID),
      evm_rpc_canister,
      rpc_config: Some(RpcConfig {
        response_size_estimate: Some(response_size_estimate),
        response_consensus: Some(ConsensusStrategy::Threshold { total: Some(3), min: 1 }),
      }),
      monitoring_interval: DEFAULT_MONITORING_TIME,
    },
    ChainConfig {
      chain_id: BSC_CHAIN_ID,
      rpc_providers: get_rpc_providers_for_chain(BSC_CHAIN_ID),
      evm_rpc_canister,
      rpc_config: Some(RpcConfig {
        response_size_estimate: Some(response_size_estimate),
        response_consensus: Some(ConsensusStrategy::Threshold { total: Some(3), min: 1 }),
      }),
      monitoring_interval: DEFAULT_MONITORING_TIME,
    },
  ]
}

pub fn get_rpc_providers_for_chain(chain: u32) -> RpcServices {
  match chain {
    ETHEREUM_CHAIN_ID => RpcServices::EthMainnet(None),
    BASE_CHAIN_ID => RpcServices::BaseMainnet(None),
    OPTIMISM_CHAIN_ID => RpcServices::OptimismMainnet(None),
    ARBITRUM_CHAIN_ID => RpcServices::ArbitrumOne(None),
    POLYGON_CHAIN_ID => RpcServices::Custom {
      chain_id: POLYGON_CHAIN_ID as u64,
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
    },
    BSC_CHAIN_ID => RpcServices::Custom {
      chain_id: BSC_CHAIN_ID as u64,
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
    },
    _ => unreachable!(),
  }
}
