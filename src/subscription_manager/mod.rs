use candid::Nat;
use candid::Principal;
use evm_logs_types::{
    RegisterSubscriptionError, RegisterSubscriptionResult, SubscriptionInfo,
    SubscriptionRegistration, UnsubscribeResult, ChainName,
};

pub mod events_publisher;
pub mod queries;
pub mod state;

use state::{SUBSCRIBERS, SUBSCRIPTIONS, FILTERS_MANAGER, PROXY_CANISTER_ID};

pub fn init(proxy_canister: Principal) {
    PROXY_CANISTER_ID.replace(proxy_canister);
    ic_cdk::println!("SubscriptionManager initialized. Proxy canister ID: {:?}", proxy_canister.to_text());
}

pub async fn register_subscription(
    registration: SubscriptionRegistration,
) -> RegisterSubscriptionResult {
    let caller = ic_cdk::caller();
    let filter = registration.filter.clone();

    let is_subscription_exist = SUBSCRIBERS.with(|subs| {
        subs.borrow().get(&caller).and_then(|sub_ids| {
            sub_ids.iter().find_map(|sub_id| {
                state::SUBSCRIPTIONS.with(|subs| {
                    subs.borrow()
                        .get(sub_id)
                        .filter(|sub_info| sub_info.filter == filter)
                        .cloned()
                })
            })
        })
    });

    if is_subscription_exist.is_some() {
        ic_cdk::println!(
            "Subscription already exists for caller {} with the same filter",
            caller
        );
        return RegisterSubscriptionResult::Err(RegisterSubscriptionError::SameFilterExists);
    }

    let chain = match registration.chain.to_string().parse::<ChainName>() {
        Ok(parsed_chain) => parsed_chain,
        Err(e) => {
            ic_cdk::println!("Failed to parse chain name: {}", e);
            return RegisterSubscriptionResult::Err(RegisterSubscriptionError::InvalidChainName);
        }
    };

    let sub_id = state::NEXT_SUBSCRIPTION_ID.with(|id| {
        let mut id = id.borrow_mut();
        let current_id = id.clone();
        *id += Nat::from(1u32);
        current_id
    });

    let subscription_info = SubscriptionInfo {
        subscription_id: sub_id.clone(),
        subscriber_principal: caller,
        namespace: registration.chain.clone(),
        filter: filter.clone(),
        skip: None,
        stats: vec![],
    };

    SUBSCRIPTIONS.with(|subs| {
        subs.borrow_mut().insert(sub_id.clone(), subscription_info);
    });

    SUBSCRIBERS.with(|subs| {
        subs.borrow_mut()
            .entry(caller)
            .or_default()
            .push(sub_id.clone());
    });

    FILTERS_MANAGER.with(|manager| {
        let mut manager = manager.borrow_mut();
        manager.add_filter(chain, &filter);
    });

    ic_cdk::println!(
        "Subscription registered: ID={}, Namespace={}",
        sub_id,
        registration.chain,
    );

    RegisterSubscriptionResult::Ok(sub_id)
}

pub fn unsubscribe(caller: Principal, subscription_id: Nat) -> UnsubscribeResult {
    let subscription_removed = SUBSCRIPTIONS.with(|subs| {
        let mut subs = subs.borrow_mut();
        subs.remove(&subscription_id)
    });

    if let Some(subscription_info) = subscription_removed {
        let filter = subscription_info.filter;

        let chain = match subscription_info.namespace.parse::<ChainName>() {
            Ok(parsed_chain) => parsed_chain,
            Err(_) => {
                ic_cdk::println!("Failed to parse chain name from namespace");
                return UnsubscribeResult::Err("Invalid namespace in subscription".to_string());
            }
        };

        FILTERS_MANAGER.with(|manager| {
            let mut manager = manager.borrow_mut();
            manager.remove_filter(chain, &filter);
        });

        SUBSCRIBERS.with(|subs| {
            let mut subs = subs.borrow_mut();
            if let Some(sub_list) = subs.get_mut(&caller) {
                sub_list.retain(|id| *id != subscription_id);
                if sub_list.is_empty() {
                    subs.remove(&caller);
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
