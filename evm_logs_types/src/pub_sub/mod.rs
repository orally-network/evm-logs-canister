use candid::{CandidType, Deserialize, Nat, Principal};
use serde::Serialize;
use std::str::FromStr;
// A note on specifying topic filters:

// A transaction with a log with topics [A, B] will be matched by the following topic filters:

// [] “anything”
// [A] “A in first position (and anything after)”
// [null, B] “anything in first position AND B in second position (and anything after)”
// [A, B] “A in first position AND B in second position (and anything after)”
// [[A, B], [A, B]] “(A OR B) in first position AND (A OR B) in second position (and anything after)”

type TopicsPosition = Vec<String>;

#[derive(CandidType, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Filter {
    pub address: String,
    pub topics: Option<Vec<TopicsPosition>>, // there is maximum of 4 topics position in the filter
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct Event {
    pub id: Nat,
    pub prev_id: Option<Nat>,
    pub timestamp: u64, // UTC Nanoseconds
    pub namespace: String,
    pub address: String,
    pub topics: Option<Vec<String>>, // TODO remove optional(?)
    pub data: ICRC16Value,
    pub tx_hash: String,
    pub headers: Option<Vec<ICRC16Map>>,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct EventNotification {
    pub sub_id: Nat,
    pub event_id: Nat,
    pub event_prev_id: Option<Nat>,
    pub timestamp: u64,
    pub namespace: String,
    pub data: ICRC16Value,
    pub topics: Vec<String>,
    pub tx_hash: String,
    pub headers: Option<Vec<ICRC16Map>>,
    pub source: Principal,
    pub filter: Option<String>,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct EventRelay {
    pub id: Nat,
    pub prev_id: Option<Nat>,
    pub timestamp: u64,
    pub namespace: String,
    pub source: Principal,
    pub data: ICRC16Value,
    pub headers: Option<Vec<ICRC16Map>>,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct SubscriptionRegistration {
    pub chain: String,
    pub filter: Filter,
    pub memo: Option<Vec<u8>>, // Blob
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct SubscriptionInfo {
    pub subscription_id: Nat,
    pub subscriber_principal: Principal,
    pub namespace: String,
    pub filter: Filter,
    pub skip: Option<Skip>,
    pub stats: Vec<ICRC16Map>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ChainName {
    Ethereum,
    Base,
    Optimism,
    Polygon,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct Skip {
    pub modulus: Nat,
    pub offset: Option<Nat>,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub enum ICRC16Value {
    Bool(bool),
    Bytes(Vec<u8>),
    Float(f64),
    Map(Vec<ICRC16Map>),
    Nat(u128),
    Principal(Principal),
    Text(String),
}

impl TryFrom<ICRC16Value> for Vec<u8> {
    type Error = &'static str;

    fn try_from(value: ICRC16Value) -> Result<Self, Self::Error> {
        match value {
            ICRC16Value::Text(text) => Ok(text.into_bytes()),
            _ => Err("Cannot convert non-text value to Vec<u8>"),
        }
    }
}

// experimental
impl TryFrom<ICRC16Value> for String {
    type Error = &'static str;

    fn try_from(value: ICRC16Value) -> Result<Self, Self::Error> {
        match value {
            ICRC16Value::Text(text) => {
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
pub struct ICRC16Property {
    pub name: String,
    pub value: ICRC16Value,
    pub immutable: bool,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ICRC16Map {
    pub key: ICRC16Value,
    pub value: ICRC16Value,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ICRC16ValueMap {
    pub key: ICRC16Value,
    pub value: ICRC16Value,
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

impl FromStr for ChainName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ethereum" => Ok(ChainName::Ethereum),
            "base" => Ok(ChainName::Base),
            "optimism" => Ok(ChainName::Optimism),
            "polygon" => Ok(ChainName::Polygon),
            _ => Err(format!("Invalid chain name: {}", s)),
        }
    }
}
