use candid::CandidType;
use candid::Principal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct Config {
    pub evm_rpc_canister: Principal,
    pub proxy_canister: Principal,
    pub estimate_events_num: u32,
    pub max_response_bytes: u32,
}
