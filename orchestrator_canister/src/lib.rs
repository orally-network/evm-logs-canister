pub mod state;
pub mod types;
pub mod api;
pub mod management;
pub mod metrics;
pub mod guards;
pub mod http_types;

use candid::{Nat, Principal};
use ic_cdk_macros::{init, post_upgrade, query, update};

use crate::state::{init_state, read_state, mutate_state};
use crate::types::*;
use crate::api::*;
use crate::guards::*;
use crate::http_types::{HttpRequest, HttpResponse};

#[init]
fn init(arg: OrchestratorInitArg) {
    init_state(arg);
}

#[post_upgrade]
fn post_upgrade() {
    // State is preserved in stable memory
}

// Admin Methods
#[update]
async fn deploy_chain_service(
    chain_id: u32,
    chain_name: String,
    config: ChainServiceConfig,
) -> Result<Principal, String> {
    is_controller().await?;
    management::deploy_chain_service(chain_id, chain_name, config).await
}

#[update]
async fn upgrade_chain_service(chain_id: u32) -> Result<(), String> {
    is_controller().await?;
    management::upgrade_chain_service(chain_id).await
}

#[update]
async fn upgrade_all_chain_services() -> Result<Vec<UpgradeResult>, String> {
    is_controller().await?;
    management::upgrade_all_chain_services().await
}

#[update]
async fn update_chain_service_wasm(wasm: Vec<u8>, version: String) -> Result<(), String> {
    is_controller().await?;
    management::update_chain_service_wasm(wasm, version)
}

#[update]
async fn pause_chain_service(chain_id: u32) -> Result<(), String> {
    is_controller().await?;
    management::pause_chain_service(chain_id).await
}

#[update]
async fn resume_chain_service(chain_id: u32) -> Result<(), String> {
    is_controller().await?;
    management::resume_chain_service(chain_id).await
}

#[update]
async fn rollback_chain_service(chain_id: u32, version: String) -> Result<(), String> {
    is_controller().await?;
    management::rollback_chain_service(chain_id, version).await
}

// User Methods
#[update]
fn register_user() -> Result<(), String> {
    api::register_user()
}

#[query]
fn get_available_chain_services() -> Vec<ChainServiceInfo> {
    api::get_available_chain_services()
}

#[update]
async fn subscribe_to_chain(chain_id: u32, filter: evm_logs_types::Filter) -> Result<SubscriptionResult, String> {
    api::subscribe_to_chain(chain_id, filter).await
}

#[update]
async fn batch_subscribe_to_chains(
    subscriptions: Vec<ChainSubscriptionRequest>,
) -> Result<BatchSubscriptionResult, String> {
    api::batch_subscribe_to_chains(subscriptions).await
}

#[query]
fn get_all_subscriptions() -> Vec<SubscriptionInfo> {
    api::get_all_subscriptions()
}

#[query]
fn get_system_status() -> SystemStatus {
    api::get_system_status()
}

// Service Discovery
#[query]
fn get_chain_service_canister_id(chain_id: u32) -> Result<Principal, String> {
    api::get_chain_service_canister_id(chain_id)
}

#[query]
fn get_orchestrator_info() -> OrchestratorInfo {
    api::get_orchestrator_info()
}

// HTTP Endpoints
#[query(hidden = true)]
fn http_request(req: HttpRequest) -> HttpResponse {
    metrics::http_request(req)
}

ic_cdk::export_candid!();
