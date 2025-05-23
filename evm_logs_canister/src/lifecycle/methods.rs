use candid::{Nat, Principal, candid_method};
use evm_logs_types::*;
use ic_cdk::caller;
use ic_cdk_macros::*;

use crate::{log_with_metrics, subscription_manager, types::balances::Balances};

/// Register subscription by specified filter (addresses and topics)
#[update(name = "subscribe")]
#[candid_method(update)]
pub async fn subscribe(registration: SubscriptionRegistration) -> RegisterSubscriptionResult {
  let received_cycles = ic_cdk::api::call::msg_cycles_available();

  log_with_metrics!(
    "Received cycles: {:?}, for principal: {:?}",
    received_cycles,
    registration.canister_to_top_up.to_text()
  );

  if let Err(err) = Balances::top_up(registration.canister_to_top_up, Nat::from(received_cycles)) {
    log_with_metrics!("Failed to top up balance: {}", err);
    return RegisterSubscriptionResult::Err(RegisterSubscriptionError::InsufficientFunds);
  }

  subscription_manager::subscription::register_subscription(registration).await
}

/// Unsubscribe from subscription with specified ID
#[update(name = "unsubscribe")]
#[candid_method(update)]
pub async fn unsubscribe(subscription_id: Nat) -> UnsubscribeResult {
  subscription_manager::subscription::unsubscribe(caller(), subscription_id)
}

/// Get all subscriptions assigned to the user (takes caller as a parameter implicitly)
#[query(name = "get_user_subscriptions")]
#[candid_method(query)]
pub fn get_user_subscriptions() -> Vec<SubscriptionInfo> {
  subscription_manager::queries::get_user_subscriptions(caller())
}

/// Get all evm-logs-canister filters info [generally for testing purpose]
#[query(name = "get_active_filters")]
#[candid_method(query)]
pub fn get_active_filters() -> Vec<evm_logs_types::Filter> {
  subscription_manager::queries::get_active_filters()
}

// Get all evm-logs-canister subscriptions info
#[query(name = "get_subscriptions")]
#[candid_method(query)]
pub fn get_subscriptions(
  namespace: Option<u32>,
  from_id: Option<Nat>,
  filters: Option<Vec<Filter>>,
) -> Vec<SubscriptionInfo> {
  subscription_manager::queries::get_subscriptions_info(namespace, from_id, filters)
}

/// Top up balance of specific user that is subscribed on some events
#[update(name = "top_up_balance")]
#[candid_method(update)]
pub fn top_up_balance(canister_to_top_up: Principal) -> TopUpBalanceResult {
  let received_cycles = ic_cdk::api::call::msg_cycles_available();

  log_with_metrics!(
    "Received cycles: {:?}, for principal: {:?}",
    received_cycles,
    canister_to_top_up.to_text()
  );

  match Balances::top_up(canister_to_top_up, Nat::from(received_cycles)) {
    Ok(_) => TopUpBalanceResult::Ok,
    Err(err) => {
      log_with_metrics!("Failed to top up balance: {}", err);
      TopUpBalanceResult::Err(TopUpBalanceError::GenericError)
    }
  }
}

/// Get balance of the specified user by its principal
#[query(name = "get_balance")]
#[candid_method(query)]
pub fn get_balance(canister_id: Principal) -> Nat {
  log_with_metrics!("get balance for canister: {:?}", canister_id.to_text());
  Balances::get_balance(&canister_id).unwrap()
}

/// Method for sending events to the Broadcaster [Created only testing purpose]
/// Used IRC72 proposal.
#[update(name = "publish_events")]
#[candid_method(update)]
pub async fn icrc72_publish(events: Vec<Event>) {
  subscription_manager::events_publisher::publish_events(events).await
}
