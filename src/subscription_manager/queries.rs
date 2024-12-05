use candid::Nat;
use candid::Principal;
use evm_logs_types::{SubscriptionInfo, Filter};
use super::state::{SUBSCRIPTIONS, SUBSCRIBERS, ADDRESSES, TOPICS_MANAGER};

pub fn get_subscriptions_info(
    namespace: Option<String>,
    from_id: Option<Nat>,
    filters: Option<Vec<Filter>>,
) -> Vec<SubscriptionInfo> {
    let mut subs_vec =
        SUBSCRIPTIONS.with(|subs| subs.borrow().values().cloned().collect::<Vec<_>>());

    if let Some(ns) = namespace {
        subs_vec.retain(|sub| sub.namespace == ns);
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
    SUBSCRIPTIONS.with(|subs| {
        subs.borrow()
            .values()
            .flat_map(|sub| sub.filters.clone().into_iter())
            .collect()
    })
}


// Get unique addresses and topics to pass to eth_getLogs.
pub fn get_active_addresses_and_topics() -> (Vec<String>, Option<Vec<Vec<String>>>) {
    let addresses: Vec<String> = ADDRESSES.with(|addr| {
        addr.borrow().keys().cloned().collect()
    });

    let topics = TOPICS_MANAGER.with(|manager| {
        manager.borrow().get_combined_topics()
    });

    (addresses, topics)
}


pub fn get_user_subscriptions(caller: Principal) -> Vec<SubscriptionInfo> {

    let subscription_ids = SUBSCRIBERS.with(|subs| {
        subs.borrow()
            .get(&caller)
            .cloned()
            .unwrap_or_else(Vec::new)
    });

    SUBSCRIPTIONS.with(|subs| {
        subscription_ids
            .iter()
            .filter_map(|id| subs.borrow().get(id).cloned())
            .collect()
    })
}
