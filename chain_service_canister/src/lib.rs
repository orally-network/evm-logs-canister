pub mod state;
pub mod types;
pub mod api;
pub mod monitoring;
pub mod metrics;
pub mod guards;
pub mod cycle_tracking;
pub mod http_types;
pub mod logs_fetcher;
pub mod events_processor;
pub mod subscriptions;

use candid::{Nat, Principal};
use ic_cdk_macros::{init, post_upgrade, query, update};
use crate::http_types::{HttpRequest, HttpResponse};

use crate::state::{init_state, read_state, mutate_state};
use crate::types::*;
use crate::api::*;
use crate::guards::*;

#[init]
fn init(config: ChainConfig) {
    init_state(config);
}

#[post_upgrade]
fn post_upgrade() {
    // State is preserved in stable memory
}

 // User Operations
#[update(guard = "is_orchestrator")]
fn top_up_balance(user: Principal) -> Result<TopUpBalanceResult, String> {
    api::top_up_balance(user)
}

#[update(guard = "is_orchestrator")]
fn get_balance(user: Principal) -> Nat {
    api::get_balance(user)
}

#[update(guard = "is_orchestrator")]
async fn register_subscription(
    registration: evm_logs_types::SubscriptionRegistration,
) -> Result<RegisterSubscriptionResult, String> {
    api::register_subscription(registration).await
}

#[update(guard = "is_orchestrator")]
async fn batch_register_subscriptions(
    registrations: Vec<evm_logs_types::SubscriptionRegistration>,
) -> Result<Vec<RegisterSubscriptionResult>, String> {
    api::batch_register_subscriptions(registrations).await
}

#[update(guard = "is_orchestrator")]
fn unsubscribe(subscription_id: Nat, user: Principal) -> Result<UnsubscribeResult, String> {
    api::unsubscribe(subscription_id, user)
}

#[update(guard = "is_orchestrator")]
fn batch_unsubscribe(subscription_ids: Vec<Nat>, user: Principal) -> Result<Vec<UnsubscribeResult>, String> {
    api::batch_unsubscribe(subscription_ids, user)
}

#[update(guard = "is_orchestrator")]
fn pause_subscription(subscription_id: Nat, user: Principal) -> Result<(), String> {
    api::pause_subscription(subscription_id, user)
}

#[update(guard = "is_orchestrator")]
fn resume_subscription(subscription_id: Nat, user: Principal) -> Result<(), String> {
    api::resume_subscription(subscription_id, user)
}

 // Query Operations
#[update(guard = "is_orchestrator")]
fn get_user_subscriptions(user: Principal) -> Vec<SubscriptionInfo> {
    api::get_user_subscriptions(user)
}

#[query(guard = "is_orchestrator")]
fn get_user_subscriptions_with_status(user: Principal) -> Vec<SubscriptionInfo> {
    api::get_user_subscriptions_with_status(user)
}

#[query(guard = "is_orchestrator")]
fn get_subscription_status(subscription_id: Nat) -> Result<SubscriptionStatus, String> {
    api::get_subscription_status(subscription_id)
}

#[query(guard = "is_orchestrator")]
fn get_cycle_usage_stats() -> CycleUsageStats {
    api::get_cycle_usage_stats()
}

#[query(guard = "is_orchestrator")]
fn get_monitoring_status() -> MonitoringStatus {
    api::get_monitoring_status()
}

#[query(guard = "is_orchestrator")]
fn get_health_status() -> HealthStatus {
    api::get_health_status()
}

// Orchestrator Communication
#[update(guard = "is_orchestrator")]
fn set_orchestrator(orchestrator: Principal) -> Result<(), String> {
    api::set_orchestrator(orchestrator)
}

#[update(guard = "is_orchestrator")]
fn update_config(config: ChainConfig) -> Result<(), String> {
    api::update_config(config)
}

#[update(guard = "is_orchestrator")]
fn start_monitoring() -> Result<(), String> {
    monitoring::start_monitoring()
}

#[update(guard = "is_orchestrator")]
fn stop_monitoring() -> Result<(), String> {
    monitoring::stop_monitoring()
}

#[update(guard = "is_orchestrator")]
fn update_proxy_canister_id(proxy_canister_id: Option<Principal>) -> Result<(), String> {
    api::update_proxy_canister_id(proxy_canister_id)
}

#[update(guard = "is_orchestrator")]
fn update_evm_rpc_canister_id(evm_rpc_canister_id: Option<Principal>) -> Result<(), String> {
    api::update_evm_rpc_canister_id(evm_rpc_canister_id)
}

#[update(guard = "is_orchestrator")]
fn update_rpc_service_config(rpc_service: RpcServiceConfig) -> Result<(), String> {
    api::update_rpc_service_config(rpc_service)
}



// HTTP Endpoints
#[query(hidden = true)]
fn http_request(req: HttpRequest) -> HttpResponse {
    metrics::http_request(req)
}

ic_cdk::export_candid!();
