use candid::{Nat, Principal};
use evm_logs_types::{Filter, SubscriptionInfo};
use evm_rpc_types::{Hex20, Hex32};

use crate::FILTERS_MANAGER;

pub fn get_subscriptions_info(
  chain_id: Option<u32>,
  from_id: Option<Nat>,
  filters: Option<Vec<Filter>>,
) -> Vec<SubscriptionInfo> {
  let mut subs_vec = crate::STATE.with(|state| state.borrow().subscriptions.values().cloned().collect::<Vec<_>>());

  if let Some(ns) = chain_id {
    subs_vec.retain(|sub| sub.chain_id == ns);
  }

  if let Some(prev_id) = from_id {
    if let Some(pos) = subs_vec.iter().position(|sub| sub.subscription_id == prev_id) {
      if pos + 1 < subs_vec.len() {
        subs_vec = subs_vec.split_off(pos + 1);
      } else {
        subs_vec.clear();
      }
    } else {
      subs_vec.clear();
    }
  }

  let _ = filters;

  subs_vec
}

pub fn get_active_filters() -> Vec<Filter> {
  crate::STATE.with(|state| {
    state
      .borrow()
      .subscriptions
      .values()
      .map(|sub| sub.filter.clone())
      .collect()
  })
}

// Get unique addresses and topics to pass to eth_getLogs.
pub fn get_active_addresses_and_topics(chain_id: u32) -> (Vec<Hex20>, Option<Vec<Vec<Hex32>>>) {
  FILTERS_MANAGER.with(|manager| {
    let manager = manager.borrow();
    manager.get_active_addresses_and_topics(chain_id)
  })
}

pub fn get_user_subscriptions(caller: Principal) -> Vec<SubscriptionInfo> {
  let subscription_ids = crate::STATE.with(|state| {
    state
      .borrow()
      .subscribers
      .get(&caller)
      .cloned()
      .unwrap_or_else(Vec::new)
  });
  crate::STATE.with(|state| {
    subscription_ids
      .iter()
      .filter_map(|id| state.borrow().subscriptions.get(id).cloned())
      .collect()
  })
}
