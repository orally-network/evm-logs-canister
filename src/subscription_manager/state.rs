use std::cell::RefCell;
use std::collections::HashMap;
use candid::{Principal, Nat};
use crate::log_filters::filter_manager::FilterManager;

use evm_logs_types::{SubscriptionInfo, Event};

thread_local! {
    pub static SUBSCRIPTIONS: RefCell<HashMap<Nat, SubscriptionInfo>> = RefCell::new(HashMap::new());
    pub static SUBSCRIBERS: RefCell<HashMap<Principal, Vec<Nat>>> = RefCell::new(HashMap::new());
    pub static EVENTS: RefCell<HashMap<Nat, Event>> = RefCell::new(HashMap::new());

    pub static NEXT_SUBSCRIPTION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    pub static NEXT_EVENT_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));
    pub static NEXT_NOTIFICATION_ID: RefCell<Nat> = RefCell::new(Nat::from(1u32));

    // pub static ADDRESSES: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());

    pub static TOPICS_MANAGER: RefCell<FilterManager> = RefCell::new(FilterManager::new());
}
