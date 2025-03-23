use std::rc::Rc;

use candid::{Nat, Principal};
use evm_logs_types::{
  RegisterSubscriptionError, RegisterSubscriptionResult, SubscriptionInfo, SubscriptionRegistration, UnsubscribeResult,
};

use crate::{
  CHAIN_SERVICES, FILTERS_MANAGER, NEXT_SUBSCRIPTION_ID, chain_service::service::ChainService, get_state_value,
  internals::misc::generate_chain_configs, log_with_metrics,
};

pub fn init() {
  log_with_metrics!("SubscriptionManager initialized");
}

pub async fn register_subscription(registration: SubscriptionRegistration) -> RegisterSubscriptionResult {
  let subscriber_principal = registration.canister_to_top_up;
  let filter = registration.filter.clone();

  let subscribers = get_state_value!(subscribers);
  let subscriptions = get_state_value!(subscriptions);

  let is_subscription_exist = subscribers.get(&subscriber_principal).and_then(|sub_ids| {
    sub_ids.iter().find_map(|sub_id| {
      subscriptions
        .get(sub_id)
        .filter(|sub_info| sub_info.filter == filter)
        .cloned()
    })
  });

  if is_subscription_exist.is_some() {
    log_with_metrics!(
      "Subscription already exists for caller {} with the same filter",
      subscriber_principal
    );
    return RegisterSubscriptionResult::Err(RegisterSubscriptionError::SameFilterExists);
  }

  let chain_id = registration.chain_id;

  let sub_id = NEXT_SUBSCRIPTION_ID.with(|id| {
    let mut id = id.borrow_mut();
    let current_id = id.clone();
    *id += Nat::from(1u32);
    current_id
  });

  let subscription_info = SubscriptionInfo {
    subscription_id: sub_id.clone(),
    subscriber_principal,
    chain_id,
    filter: filter.clone(),
    stats: vec![],
  };

  // add to subscriptions
  crate::STATE.with(|subs| {
    subs
      .borrow_mut()
      .subscriptions
      .insert(sub_id.clone(), subscription_info);
  });

  // add to subscribers
  crate::STATE.with(|subs| {
    subs
      .borrow_mut()
      .subscribers
      .entry(subscriber_principal)
      .or_default()
      .push(sub_id.clone());
  });

  FILTERS_MANAGER.with(|manager| {
    let mut manager = manager.borrow_mut();
    manager.add_filter(chain_id, &filter);
  });

  log_with_metrics!(
    "Subscription registered: ID={}, Namespace={}, Filter = {:?}",
    sub_id,
    registration.chain_id,
    filter,
  );

  // start timer corresponding to the subscription if it's the first subscription for the chain
  // check if there are any subscriptions for the chain
  let subscriptions_amount = crate::STATE.with(|subs| {
    subs
      .borrow()
      .subscriptions
      .values()
      .filter(|sub_info| sub_info.chain_id == chain_id)
      .count()
  });

  if subscriptions_amount == 1 {
    let chain_configs = generate_chain_configs();
    let chain_config = chain_configs
      .iter()
      .find(|config| config.chain_id == chain_id)
      .expect("Chain config not found. Add it to the chain configs generation");

    let service = Rc::new(ChainService::new(chain_config.clone()));
    let monitoring_interval = std::time::Duration::from_secs(chain_config.monitoring_interval_sec);
    service.clone().start_monitoring(monitoring_interval);

    CHAIN_SERVICES.with(|chain_services| {
      chain_services.borrow_mut().push(service);
    });
  }

  RegisterSubscriptionResult::Ok(sub_id)
}

pub fn unsubscribe(caller: Principal, subscription_id: Nat) -> UnsubscribeResult {
  // remove subscription from the state
  let removed_subscription = crate::STATE.with(|subs| subs.borrow_mut().subscriptions.remove(&subscription_id));

  if let Some(subscription_info) = removed_subscription {
    let filter = subscription_info.filter;
    let chain_id = subscription_info.chain_id;

    // remove subscription filter from the filter manager
    FILTERS_MANAGER.with(|manager| {
      let mut manager = manager.borrow_mut();
      manager.remove_filter(chain_id, &filter);
    });

    // update subscribers state
    crate::STATE.with(|subs| {
      let mut subs = subs.borrow_mut();
      if let Some(sub_list) = subs.subscribers.get_mut(&caller) {
        sub_list.retain(|id| *id != subscription_id);
        if sub_list.is_empty() {
          subs.subscribers.remove(&caller);
        }
      }
    });

    // Stop timer for specific chain ID if there are no more subscriptions for it
    let has_subscriptions = crate::STATE.with(|subs| {
      subs
        .borrow()
        .subscriptions
        .values()
        .any(|sub_info| sub_info.chain_id == chain_id)
    });

    if !has_subscriptions {
      CHAIN_SERVICES.with(|chain_services| {
        let chain_services = chain_services.borrow_mut();
        // call stop_monitoring for the chain service, but don't remove it
        if let Some(service) = chain_services
          .iter()
          .find(|service| service.config.chain_id == chain_id)
        {
          service.stop_monitoring()
        }
      });
    }

    UnsubscribeResult::Ok()
  } else {
    UnsubscribeResult::Err(format!("Subscription with ID {} not found", subscription_id))
  }
}

#[cfg(test)]
mod tests {
  use std::str::FromStr;

  use evm_logs_types::{Filter, RegisterSubscriptionResult, SubscriptionRegistration};
  use evm_rpc_types::{Hex20, Hex32};

  use super::*;
  #[test]
  fn test_unsubscribe_nonexistent() {
    // Using tokio runtime explicitly because of tokio::test error. TODO fix
    tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(async {
        let result = unsubscribe(Principal::anonymous(), Nat::from(9999u32));
        assert!(matches!(result, UnsubscribeResult::Err(_)));
      })
  }
}
