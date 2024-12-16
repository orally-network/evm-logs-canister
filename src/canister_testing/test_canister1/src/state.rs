use std::cell::RefCell;
use candid::Nat;
use std::collections::HashMap;
use crate::read_contract::SolidityToken;
use evm_logs_types::EventNotification;

thread_local! {
    pub static NOTIFICATIONS: RefCell<Vec<EventNotification>> = RefCell::new(Vec::new());
    pub static DECODERS: RefCell<HashMap<Nat, Box<dyn Fn(&EventNotification) -> Result<Vec<SolidityToken>, String>>>> = RefCell::new(HashMap::new());
    pub static DECODED_NOTIFICATIONS: RefCell<Vec<(EventNotification, Vec<SolidityToken>)>> = RefCell::new(Vec::new());
}