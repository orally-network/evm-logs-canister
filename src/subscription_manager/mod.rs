use candid::Nat;
use candid::Principal;
use evm_logs_types::{
    RegisterSubscriptionError, RegisterSubscriptionResult, SubscriptionInfo,
    SubscriptionRegistration, UnsubscribeResult,
};
use crate::get_state_value;
use crate::log;

pub mod events_publisher;
pub mod queries;
pub mod utils;

use crate::{TOPICS_MANAGER, NEXT_SUBSCRIPTION_ID};

pub fn init() {
    log!("SubscriptionManager initialized");
}

pub async fn register_subscription(
    registration: SubscriptionRegistration,
) -> RegisterSubscriptionResult {
    let subscriber_principal = registration.canister_to_top_up;
    let filter = registration.filter.clone();

    let subscribers = get_state_value!(subscribers);
    let subscriptions = get_state_value!(subscriptions);

    let is_subscription_exist = subscribers
        .get(&subscriber_principal)
        .and_then(|sub_ids| {
            sub_ids.iter().find_map(|sub_id| {
                subscriptions
                    .get(sub_id)
                    .filter(|sub_info| sub_info.filter == filter)
                    .cloned()
            })
        });

    if is_subscription_exist.is_some() {
        log!(
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
        skip: None,
        stats: vec![],
    };

    // add to subscriptions
    crate::STATE.with(|subs| {
        subs.borrow_mut().subscriptions.insert(sub_id.clone(), subscription_info);
    });

    // add to subscribers
    crate::STATE.with(|subs| {
        subs.borrow_mut().subscribers.entry(subscriber_principal)
            .or_default()
            .push(sub_id.clone());
    });

    TOPICS_MANAGER.with(|manager| {
        let mut manager = manager.borrow_mut();
        manager.add_filter(chain_id, &filter);
    });

    log!(
        "Subscription registered: ID={}, Namespace={}",
        sub_id,
        registration.chain_id,
    );

    RegisterSubscriptionResult::Ok(sub_id)
}

pub fn unsubscribe(caller: Principal, subscription_id: Nat) -> UnsubscribeResult {
    // remove subscription from the state
    let removed_subscription = crate::STATE.with(|subs| {
        subs.borrow_mut().subscriptions.remove(&subscription_id)
    });

    if let Some(subscription_info) = removed_subscription {
        let filter = subscription_info.filter;

        let chain_id = subscription_info.chain_id;

        // remove subscription filter from the filter manager
        TOPICS_MANAGER.with(|manager| {
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

        UnsubscribeResult::Ok()
    } else {
        UnsubscribeResult::Err(format!(
            "Subscription with ID {} not found",
            subscription_id
        ))
    }
}
