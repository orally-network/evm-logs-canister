use candid::{CandidType, Deserialize, Nat, Principal};
use evm_rpc_types::{Hex20, Hex32, LogEntry};
use serde::Serialize;

// A note on specifying topic filters:

// A transaction with a log with topics [A, B] will be matched by the following topic filters:

// [] “anything”
// [A] “A in first position (and anything after)”
// [null, B] “anything in first position AND B in second position (and anything after)”
// [A, B] “A in first position AND B in second position (and anything after)”
// [[A, B], [A, B]] “(A OR B) in first position AND (A OR B) in second position (and anything after)”
pub type TopicsPosition = Vec<Hex32>;

#[derive(CandidType, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Filter {
    pub address: Hex20,
    pub topics: Option<Vec<TopicsPosition>>, // there is maximum of 4 topics position in the filter
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct Event {
    pub id: Nat,
    pub timestamp: u64, // UTC Nanoseconds
    pub chain_id: u32,
    pub log_entry: LogEntry,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct EventNotification {
    pub sub_id: Nat,
    pub event_id: Nat,
    pub timestamp: u64,
    pub chain_id: u32,
    pub filter: Option<String>,
    pub source: Principal,
    pub log_entry: LogEntry,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum SendNotificationResult {
    Ok,
    Err(SendNotificationError),
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum SendNotificationError {
    FailedToSend,      // General failure to send notification
    InvalidSubscriber, // Invalid subscriber principal
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct EventRelay {
    pub id: Nat,
    pub prev_id: Option<Nat>,
    pub timestamp: u64,
    pub namespace: String,
    pub source: Principal,
    pub data: Value,
    pub headers: Option<Vec<Map>>,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct SubscriptionRegistration {
    pub chain_id: u32,
    pub filter: Filter,
    pub memo: Option<Vec<u8>>, // Blob
    pub canister_to_top_up: Principal,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct SubscriptionInfo {
    pub subscription_id: Nat,
    pub subscriber_principal: Principal,
    pub chain_id: u32,
    pub filter: Filter,
    pub stats: Vec<Map>,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct Skip {
    pub modulus: Nat,
    pub offset: Option<Nat>,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum Value {
    Bool(bool),
    Bytes(Vec<u8>),
    Float(f64),
    Map(Vec<Map>),
    Nat(u128),
    Principal(Principal),
    Text(String),
}

impl TryFrom<Value> for Vec<u8> {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Text(text) => Ok(text.into_bytes()),
            _ => Err("Cannot convert non-text value to Vec<u8>"),
        }
    }
}

// experimental
impl TryFrom<Value> for String {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Text(text) => {
                if text.len() >= 2 {
                    Ok(text[2..].to_string())
                } else {
                    Err("String is too short to remove first two characters")
                }
            }
            _ => Err("Cannot convert non-text value to String"),
        }
    }
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct Property {
    pub name: String,
    pub value: Value,
    pub immutable: bool,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct Map {
    pub key: Value,
    pub value: Value,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ValueMap {
    pub key: Value,
    pub value: Value,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct GenericError {
    pub error_code: Nat,
    pub message: String,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum RegisterSubscriptionResult {
    Ok(Nat), // sub id
    Err(RegisterSubscriptionError),
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum UnsubscribeResult {
    Ok(),
    Err(String),
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum RegisterSubscriptionError {
    Unauthorized,
    UnauthorizedSubscriber { namespace: String },
    ImproperConfig(String),
    GenericError(GenericError),
    SameFilterExists,
    InvalidChainName,
    InsufficientFunds,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum TopUpBalanceResult {
    Ok,
    Err(TopUpBalanceError),
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum TopUpBalanceError {
    GenericError,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum PublishError {
    Unauthorized,
    ImproperId(String),
    Busy,
    GenericError(GenericError),
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum ConfirmationResult {
    AllAccepted,
    SomeRejected(Vec<Nat>), // rejected id's
}
