use candid::{CandidType, Deserialize, Nat, Principal};
use serde_bytes::ByteBuf;

// Test constants - reuse from chain service tests to avoid CheckSequenceNotMatch errors
pub const ORCHESTRATOR_PRINCIPAL: &str = "mqygn-kiaaa-aaaar-qaadq-cai";
pub const USER_PRINCIPAL: &str = "mxzaz-hqaaa-aaaar-qaada-cai";
pub const EVM_RPC_PRINCIPAL: &str = "7hfb6-caaaa-aaaar-qadga-cai";
pub const CHAIN_SERVICE_CANISTER_ID: &str = "lxzze-o7777-77777-aaaaa-cai";

// Import types from actual canisters instead of duplicating them
pub use chain_service_canister::types::{
    ChainConfig, RpcServiceConfig, SubscriptionInfo, 
    SubscriptionStatus, CycleUsageStats, MonitoringStatus,
    HealthStatus, TopUpBalanceResult, RegisterSubscriptionResult,
    UnsubscribeResult
};

pub use orchestrator_canister::types::{
    OrchestratorInitArg, ChainServiceConfig, ChainServiceInfo,
    ChainServiceStatus
};

// HTTP types for metrics endpoint
#[derive(CandidType, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: ByteBuf,
}

#[derive(CandidType, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: ByteBuf,
}

// Helper function to convert ChainServiceConfig to ChainConfig
pub fn chain_service_config_to_chain_config(config: ChainServiceConfig) -> ChainConfig {
    ChainConfig {
        chain_id: config.chain_id,
        chain_name: config.chain_name,
        rpc_url: config.rpc_url.clone(),
        block_interval_seconds: config.block_interval_seconds,
        max_response_bytes: config.max_response_bytes,
        proxy_canister_id: None,
        evm_rpc_canister_id: Some(Principal::from_text(EVM_RPC_PRINCIPAL).unwrap()),
        rpc_service: match config.chain_id {
            1 => RpcServiceConfig::EthMainnet { providers: None },
            11155111 => RpcServiceConfig::EthSepolia { providers: None },
            42161 => RpcServiceConfig::ArbitrumOne { providers: None },
            8453 => RpcServiceConfig::BaseMainnet { providers: None },
            10 => RpcServiceConfig::OptimismMainnet { providers: None },
            _ => RpcServiceConfig::Custom { 
                rpc_url: config.rpc_url.clone(),
                chain_id: config.chain_id as u64,
            },
        },
    }
}
