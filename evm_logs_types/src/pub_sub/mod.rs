use candid::{CandidType, Deserialize, Principal, Nat};
use serde::Serialize;

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct Event {
    pub id: Nat,
    pub prev_id: Option<Nat>,
    pub timestamp: u64, // UTC Nanoseconds
    pub namespace: String,
    pub data: ICRC16Value,
    pub headers: Option<Vec<ICRC16Map>>,
    pub address: String,
    pub topics: Option<Vec<String>>
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct EventNotification {
    pub id: Nat,
    pub event_id: Nat,
    pub event_prev_id: Option<Nat>,
    pub timestamp: u64,
    pub namespace: String,
    pub data: ICRC16Value,
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
    pub namespace: String,
    pub filters: Vec<Filter>,
    pub memo: Option<Vec<u8>>, // Blob
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct SubscriptionInfo {
    pub subscription_id: Nat,
    pub subscriber_principal: Principal,
    pub namespace: String,
    pub filters: Vec<Filter>,
    pub skip: Option<Skip>,
    pub stats: Vec<ICRC16Map>,
}

#[derive(Clone, Debug)]
pub enum ChainName {
    Ethereum,
    Base,
    Optimism,
}

#[derive(CandidType, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Filter {
    pub addresses: Vec<String>,
    pub topics: Option<Vec<Vec<String>>>,
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
impl TryFrom<ICRC16Value> for String{
    type Error = &'static str;

    fn try_from(value: ICRC16Value) -> Result<Self, Self::Error> {
        match value {
            ICRC16Value::Text(text) => Ok(text),
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