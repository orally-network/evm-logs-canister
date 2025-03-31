use ic_cdk_macros::init;

use crate::{log_with_metrics, subscription_manager, types};

#[init]
async fn init(config: types::config::Config) {
  subscription_manager::subscription::init();
  types::state::init(config);
  log_with_metrics!("EVM logs canister initialized.");
}
