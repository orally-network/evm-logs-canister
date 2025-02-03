use std::{
    cell::RefCell, sync::Arc, time::Duration
};

use candid::{Nat, Principal, CandidType, Deserialize};

use ic_cdk_macros::*;
use evm_rpc_types::{LogEntry, GetLogsArgs, RpcServices,
    RpcConfig, MultiRpcResult, Hex, Hex32, Nat256, Hex20
};

#[init]
async fn init() {}

#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();

#[update(name = "eth_getLogs")]
pub async fn eth_get_logs(
    _source: RpcServices,
    _config: Option<RpcConfig>,
    _args: GetLogsArgs,
) -> MultiRpcResult<Vec<LogEntry>> {
    let log_entries = vec![
        LogEntry {
            transaction_hash: Some(Hex32::from([0; 32])),
            block_number: Some(Nat256::from(1111u32)),
            data: Hex::from(vec![]),
            block_hash: Some(Hex32::from([0; 32])),
            transaction_index: Some(Nat256::from(1u32)),
            topics: vec![Hex32::from([0; 32])],
            address: Hex20::from([0; 20]),
            log_index: Some(Nat256::from(0u32)),
            removed: false,
        },
        LogEntry {
            transaction_hash: Some(Hex32::from([0; 32])),
            block_number: Some(Nat256::from(2222u32)),
            data: Hex::from(vec![]),
            block_hash: Some(Hex32::from([0; 32])),
            transaction_index: Some(Nat256::from(1u32)),
            topics: vec![Hex32::from([0; 32])],
            address: Hex20::from([0; 20]),
            log_index: Some(Nat256::from(0u32)),
            removed: false,
        },
    ];
    
    MultiRpcResult::Consistent(Ok(log_entries))
}
