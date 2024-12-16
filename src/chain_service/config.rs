use candid::Principal;
use evm_logs_types::ChainName;
use evm_rpc_canister_types::RpcServices;

#[derive(Clone)]
pub struct ChainConfig {
    pub chain_name: ChainName,
    pub rpc_providers: RpcServices,
    pub evm_rpc_canister: Principal,
}
