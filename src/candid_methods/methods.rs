use candid::{candid_method, Principal};
use ic_cdk_macros::*;
use candid::Nat;
use evm_logs_types::*;
use metrics::cycles_count;

use crate::subscription_manager;
use ic_cdk::caller;
use crate::log;
use crate::types::balances::Balances;

// register subscription by specified filter (addresses and topics)
#[update(name = "subscribe")]
#[candid_method(update)]
pub async fn subscribe(
    registration: SubscriptionRegistration,
    principal: Option<Principal>,
) -> RegisterSubscriptionResult {
    let received_cycles = ic_cdk::api::call::msg_cycles_available();
    let principal_to_top_up = principal.unwrap_or_else(caller);

    log!("Received cycles: {:?}, for principal: {:?}", received_cycles, principal_to_top_up.to_text());

    if let Err(err) = Balances::top_up(principal_to_top_up, Nat::from(received_cycles)) {
        log!("Failed to top up balance: {}", err);
        return RegisterSubscriptionResult::Err(RegisterSubscriptionError::InsufficientFunds);
    }

    subscription_manager::register_subscription(registration).await
}

// unsubscribe from subscription with specified ID
#[update(name = "unsubscribe")]
#[candid_method(update)]
pub async fn unsubscribe(subscription_id: Nat) -> UnsubscribeResult {
    subscription_manager::unsubscribe(caller(), subscription_id)
}

// get all subscriptions assigned to the user
#[query(name = "get_user_subscriptions")]
#[candid_method(query)]
pub fn get_user_subscriptions() -> Vec<SubscriptionInfo> {
    subscription_manager::queries::get_user_subscriptions(caller())
}

// generally for testing purpose

// get all evm-logs-canister filters info
#[query(name = "get_active_filters")]
#[candid_method(query)]
pub fn get_active_filters() -> Vec<evm_logs_types::Filter> {
    subscription_manager::queries::get_active_filters()
}

// get all evm-logs-canister subscriptions info
#[query(name = "get_subscriptions")]
#[candid_method(query)]
#[cycles_count]
pub fn get_subscriptions(
    namespace: Option<u32>,
    from_id: Option<Nat>,
    filters: Option<Vec<Filter>>,
) -> Vec<SubscriptionInfo> {
    subscription_manager::queries::get_subscriptions_info(namespace, from_id, filters)
}

#[update(name = "top_up_balance")]
#[candid_method(update)]
pub fn top_up_balance(principal: Option<Principal>) -> TopUpBalanceResult {
    let received_cycles = ic_cdk::api::call::msg_cycles_available();
    let principal_to_top_up = principal.unwrap_or_else(caller);
    
    log!("Received cycles: {:?}, for principal: {:?}", received_cycles, principal_to_top_up.to_text());

    match Balances::top_up(principal_to_top_up, Nat::from(received_cycles)) {
        Ok(_) => TopUpBalanceResult::Ok,
        Err(err) => {
            log!("Failed to top up balance: {}", err);
            TopUpBalanceResult::Err(TopUpBalanceError::GenericError)
        }
    }
}

#[query(name = "get_balance")]
#[candid_method(query)]
#[cycles_count]
pub fn get_balance() -> Nat {
    let caller = caller();
    log!("get balance, caller: {:?}", caller.to_text());
    Balances::get_balance(&caller).unwrap()
}

// only testing purpose
#[update(name = "publish_events")]
#[candid_method(update)]
pub async fn icrc72_publish(events: Vec<Event>) -> Vec<Option<Result<Vec<Nat>, PublishError>>> {
    subscription_manager::events_publisher::publish_events(events).await
}