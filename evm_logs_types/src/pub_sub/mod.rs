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

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
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
    Array(Vec<ICRC16Value>),
    Blob(Vec<u8>),
    Bool(bool),
    Bytes(Vec<u8>),
    Class(Vec<ICRC16Property>),
    Float(f64),
    Floats(Vec<f64>),
    Int(i128),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Int8(i8),
    Map(Vec<ICRC16Map>),
    ValueMap(Vec<ICRC16ValueMap>),
    Nat(u128),
    Nat16(u16),
    Nat32(u32),
    Nat64(u64),
    Nat8(u8),
    Nats(Vec<u128>),
    Option(Box<ICRC16Value>),
    Principal(Principal),
    Set(Vec<ICRC16Value>),
    Text(String),
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