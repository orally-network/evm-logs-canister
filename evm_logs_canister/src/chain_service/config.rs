use candid::{CandidType, Principal};
use evm_rpc_types::{RpcConfig, RpcServices};
use serde::Deserialize;

#[derive(Clone, CandidType, Deserialize)]
pub struct ChainConfig {
  pub chain_id: u32,
  pub rpc_providers: RpcServices,
  pub evm_rpc_canister: Principal,
  pub rpc_config: Option<RpcConfig>, // for ConsensusStrategy
  pub monitoring_interval: u64,      // in seconds
}

impl ChainConfig {
  pub fn rpc_providers_len(&self) -> usize {
    match &self.rpc_providers {
      RpcServices::Custom { services, .. } => services.len(),
      RpcServices::EthMainnet(Some(services)) => services.len(),
      RpcServices::EthSepolia(Some(services)) => services.len(),
      RpcServices::ArbitrumOne(Some(services)) => services.len(),
      RpcServices::BaseMainnet(Some(services)) => services.len(),
      RpcServices::OptimismMainnet(Some(services)) => services.len(),
      _ => 0, // Covers None cases or variants without Vec fields
    }
  }
}
