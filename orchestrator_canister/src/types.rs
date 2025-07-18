use candid::{CandidType, Deserialize, Nat, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use serde::Serialize;
use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct OrchestratorInitArg {
    pub admin: Principal,
    pub version: String,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct OrchestratorState {
    pub admin: Principal,
    pub chain_services: HashMap<u32, ChainServiceInfo>,
    pub user_registry: HashMap<Principal, UserInfo>,
    pub chain_service_wasm: Option<Vec<u8>>,
    pub previous_wasms: VecDeque<(String, Vec<u8>)>,
    pub version: String,
}

impl Default for OrchestratorState {
    fn default() -> Self {
        Self {
            admin: Principal::anonymous(),
            chain_services: HashMap::new(),
            user_registry: HashMap::new(),
            chain_service_wasm: None,
            previous_wasms: VecDeque::new(),
            version: "0.1.0".to_string(),
        }
    }
}

impl Storable for OrchestratorState {
    fn to_bytes(&self) -> Cow<[u8]> {
        let bytes = bincode::serialize(self).expect("Failed to serialize OrchestratorState");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(&bytes).expect("Failed to deserialize OrchestratorState")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ChainServiceInfo {
    pub canister_id: Principal,
    pub chain_id: u32,
    pub chain_name: String,
    pub version: String,
    pub status: ChainServiceStatus,
    pub deployment_timestamp: u64,
    pub last_upgrade_timestamp: Option<u64>,
}

impl Storable for ChainServiceInfo {
    fn to_bytes(&self) -> Cow<[u8]> {
        let bytes = bincode::serialize(self).expect("Failed to serialize ChainServiceInfo");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(&bytes).expect("Failed to deserialize ChainServiceInfo")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum ChainServiceStatus {
    Active,
    Paused,
    Upgrading,
    Failed,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct UserInfo {
    pub principal: Principal,
    pub subscribed_chains: Vec<u32>,
    pub registration_timestamp: u64,
    pub last_activity_timestamp: u64,
}

impl Storable for UserInfo {
    fn to_bytes(&self) -> Cow<[u8]> {
        let bytes = bincode::serialize(self).expect("Failed to serialize UserInfo");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(&bytes).expect("Failed to deserialize UserInfo")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ChainServiceConfig {
    pub chain_id: u32,
    pub chain_name: String,
    pub rpc_url: String,
    pub block_interval_seconds: u64,
    pub max_response_bytes: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct SubscriptionResult {
    pub subscription_id: Nat,
    pub chain_service_canister_id: Principal,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ChainSubscriptionRequest {
    pub chain_id: u32,
    pub filter: evm_logs_types::Filter,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct BatchSubscriptionResult {
    pub successful: Vec<SubscriptionResult>,
    pub failed: Vec<(ChainSubscriptionRequest, String)>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct UpgradeResult {
    pub chain_id: u32,
    pub canister_id: Principal,
    pub success: bool,
    pub error_message: Option<String>,
    pub timestamp: u64,
    pub from_version: String,
    pub to_version: String,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct SystemStatus {
    pub total_chain_services: u32,
    pub active_chain_services: u32,
    pub total_subscriptions: u64,
    pub orchestrator_version: String,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct OrchestratorInfo {
    pub admin: Principal,
    pub chain_services: Vec<ChainServiceInfo>,
    pub version: String,
    pub total_users: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct SubscriptionInfo {
    pub subscription_id: Nat,
    pub subscriber_principal: Principal,
    pub chain_id: u32,
    pub filter: evm_logs_types::Filter,
    pub status: SubscriptionStatus,
    pub created_at: u64,
    pub last_updated: u64,
    pub cycles_consumed: u64,
    pub events_received: u64,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum SubscriptionStatus {
    Active,
    InsufficientBalance { since: u64 },
    ChainServiceOffline { since: u64 },
    ProcessingError { error: String, since: u64 },
    PausedByUser { since: u64 },
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct OrchestratorMetrics {
    pub total_chain_services: u32,
    pub active_chain_services: u32,
    pub paused_chain_services: u32,
    pub failed_chain_services: u32,
    pub total_subscriptions: u64,
    pub total_users: u64,
    pub cycle_balance: u128,
    pub successful_upgrades: u64,
    pub failed_upgrades: u64,
    pub current_wasm_version: String,
}
