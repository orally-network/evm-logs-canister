use std::{cell::RefCell, collections::HashMap};

use candid::Nat;
use evm_logs_types::EventNotification;

use crate::read_contract::SolidityToken;

thread_local! {
    pub static NOTIFICATIONS: RefCell<Vec<EventNotification>> = RefCell::new(Vec::new());
    pub static DECODERS: RefCell<HashMap<Nat, Box<dyn Fn(&EventNotification) -> Result<Vec<SolidityToken>, String>>>> = RefCell::new(HashMap::new());
    pub static DECODED_NOTIFICATIONS: RefCell<Vec<(EventNotification, Vec<SolidityToken>)>> = RefCell::new(Vec::new());
}
