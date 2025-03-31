use std::{rc::Rc, time::Duration};

use candid::Nat;
use canister_utils::debug_log;
use ic_cdk::storage;

use crate::{
  CHAIN_SERVICES, FILTERS_MANAGER, NEXT_NOTIFICATION_ID, NEXT_SUBSCRIPTION_ID, STATE,
  chain_service::{ChainConfig, service::ChainService},
  internals::misc::DEFAULT_MONITORING_TIME,
  log_filters::filter_manager::FilterManager,
  types::state::State,
};

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
  let state = STATE.with(|state| state.borrow().clone());
  let next_subscription_id = NEXT_SUBSCRIPTION_ID.with(|id| id.borrow().clone());
  let next_notification_id = NEXT_NOTIFICATION_ID.with(|id| id.borrow().clone());
  let topics_manager = FILTERS_MANAGER.with(|manager| manager.borrow().clone());

  let chain_configs: Vec<ChainConfig> = CHAIN_SERVICES.with(|chain_services| {
    chain_services
      .borrow()
      .iter()
      .map(|service| service.config.clone())
      .collect()
  });

  storage::stable_save((
    state,
    next_subscription_id,
    next_notification_id,
    topics_manager,
    chain_configs,
  ))
  .expect("error during pre_upgrade state saving");

  debug_log!("pre_upgrade: State saved successfully.");
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
  let (
        saved_state,
        saved_next_subscription_id,
        saved_next_notification_id,
        saved_topics_manager,
        saved_chain_configs,
    ): (State, Nat, Nat, FilterManager, Vec<ChainConfig>) =
        storage::stable_restore().expect("Failed to restore state after upgrade");

  STATE.with(|state| {
    *state.borrow_mut() = saved_state;
  });

  NEXT_SUBSCRIPTION_ID.with(|id| {
    *id.borrow_mut() = saved_next_subscription_id;
  });

  NEXT_NOTIFICATION_ID.with(|id| {
    *id.borrow_mut() = saved_next_notification_id;
  });

  FILTERS_MANAGER.with(|manager| {
    *manager.borrow_mut() = saved_topics_manager;
  });

  let monitoring_interval = Duration::from_secs(DEFAULT_MONITORING_TIME); // TODO

  let restored_services: Vec<Rc<ChainService>> = saved_chain_configs
    .into_iter()
    .map(|config| {
      let service = Rc::new(ChainService::new(config));
      service.clone().start_monitoring(monitoring_interval);
      service
    })
    .collect();

  CHAIN_SERVICES.with(|chain_services| {
    *chain_services.borrow_mut() = restored_services;
  });

  debug_log!("post_upgrade: State restored successfully.");
}
