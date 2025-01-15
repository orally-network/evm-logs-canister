use candid::Principal;
use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct EventsPerInterval {
    pub interval: u32, // interval of events monitoring process 
    pub events_num: u32, // number of the events per this interval for one address
}
#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct Config {
    pub evm_rpc_canister: Principal,
    pub proxy_canister: Principal,
    pub rpc_wrapper: String,
    pub events_per_interval: EventsPerInterval,
}