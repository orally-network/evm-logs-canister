use candid::{CandidType, Deserialize, Nat, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use ic_cdk_timers::TimerId;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ChainConfig {
    pub chain_id: u32,
    pub chain_name: String,
    pub rpc_url: String,
    pub block_interval_seconds: u64,
    pub max_response_bytes: u64,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ChainServiceState {
    pub config: ChainConfig,
    pub orchestrator: Option<Principal>,
    pub last_processed_block: Nat,
    pub subscriptions: HashMap<Nat, SubscriptionInfo>,
    pub user_balances: HashMap<Principal, Nat>,
    pub timer_id: Option<String>, // String representation for stable storage
    pub cycle_usage_stats: CycleUsageStats,
    pub version: String,
    pub next_subscription_id: Nat,
}

impl Default for ChainServiceState {
    fn default() -> Self {
        Self {
            config: ChainConfig {
                chain_id: 0,
                chain_name: "Unknown".to_string(),
                rpc_url: "".to_string(),
                block_interval_seconds: 60,
                max_response_bytes: 10000,
            },
            orchestrator: None,
            last_processed_block: Nat::from(0u32),
            subscriptions: HashMap::new(),
            user_balances: HashMap::new(),
            timer_id: None,
            cycle_usage_stats: CycleUsageStats::default(),
            version: "0.1.0".to_string(),
            next_subscription_id: Nat::from(1u32),
        }
    }
}

impl Storable for ChainServiceState {
    fn to_bytes(&self) -> Cow<[u8]> {
        let bytes = bincode::serialize(self).expect("Failed to serialize ChainServiceState");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(&bytes).expect("Failed to deserialize ChainServiceState")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
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

impl Storable for SubscriptionInfo {
    fn to_bytes(&self) -> Cow<[u8]> {
        let bytes = bincode::serialize(self).expect("Failed to serialize SubscriptionInfo");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(&bytes).expect("Failed to deserialize SubscriptionInfo")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum SubscriptionStatus {
    Active,
    InsufficientBalance { since: u64 },
    ChainServiceOffline { since: u64 },
    ProcessingError { error: String, since: u64 },
    PausedByUser { since: u64 },
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct CycleUsageStats {
    pub total_cycles_used: u64,
    pub last_execution_cycles: u64,
    pub average_cycles_per_block: u64,
    pub cycles_per_log_entry: u64,
    pub last_updated: u64,
    pub execution_count: u64,
}

impl Default for CycleUsageStats {
    fn default() -> Self {
        Self {
            total_cycles_used: 0,
            last_execution_cycles: 0,
            average_cycles_per_block: 0,
            cycles_per_log_entry: 0,
            last_updated: 0,
            execution_count: 0,
        }
    }
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct MonitoringStatus {
    pub is_monitoring: bool,
    pub last_processed_block: Nat,
    pub timer_id: Option<String>,
    pub monitoring_interval_seconds: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct HealthStatus {
    pub chain_id: u32,
    pub is_healthy: bool,
    pub last_successful_fetch: Option<u64>,
    pub consecutive_failures: u32,
    pub error_message: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct TopUpBalanceResult {
    pub new_balance: Nat,
    pub cycles_received: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct RegisterSubscriptionResult {
    pub subscription_id: Nat,
    pub estimated_cycles_per_day: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct UnsubscribeResult {
    pub refunded_cycles: Nat,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ChainServiceMetrics {
    pub chain_id: u32,
    pub chain_name: String,
    pub subscription_count: u64,
    pub active_subscription_count: u64,
    pub cycle_usage_stats: CycleUsageStats,
    pub block_processing_stats: BlockProcessingStats,
    pub error_stats: ErrorStats,
    pub user_balance_stats: UserBalanceStats,
    pub cycle_balance: u128,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct BlockProcessingStats {
    pub last_processed_block: Nat,
    pub blocks_processed_total: u64,
    pub average_processing_time_ms: u64,
    pub logs_fetched_total: u64,
    pub events_published_total: u64,
}

impl Default for BlockProcessingStats {
    fn default() -> Self {
        Self {
            last_processed_block: Nat::from(0u32),
            blocks_processed_total: 0,
            average_processing_time_ms: 0,
            logs_fetched_total: 0,
            events_published_total: 0,
        }
    }
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ErrorStats {
    pub rpc_errors_total: u64,
    pub processing_errors_total: u64,
    pub notification_errors_total: u64,
    pub consecutive_failures: u32,
    pub last_error_timestamp: Option<u64>,
}

impl Default for ErrorStats {
    fn default() -> Self {
        Self {
            rpc_errors_total: 0,
            processing_errors_total: 0,
            notification_errors_total: 0,
            consecutive_failures: 0,
            last_error_timestamp: None,
        }
    }
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct UserBalanceStats {
    pub total_deposited_cycles: u128,
    pub total_consumed_cycles: u128,
    pub average_user_balance: u64,
    pub users_with_insufficient_balance: u32,
}

impl Default for UserBalanceStats {
    fn default() -> Self {
        Self {
            total_deposited_cycles: 0,
            total_consumed_cycles: 0,
            average_user_balance: 0,
            users_with_insufficient_balance: 0,
        }
    }
}

// Runtime timer storage (not persisted in stable memory)
thread_local! {
    pub static TIMER_ID: std::cell::RefCell<Option<TimerId>> = std::cell::RefCell::new(None);
}
