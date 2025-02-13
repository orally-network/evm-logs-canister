use candid::Principal;
use evm_rpc_types::{RpcServices, RpcConfig};

#[derive(Clone)]
pub struct ChainConfig {
    pub chain_id: u32,
    pub rpc_providers: RpcServices,
    pub evm_rpc_canister: Principal,
    pub rpc_config: Option<RpcConfig>, // for ConsensusStrategy 
}
