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
    let mut log_entries = vec![
        LogEntry {
            address: Hex20::from([0; 20]),
            topics: vec![Hex32::from([0; 32])],
            transaction_hash: None,
            block_number: None,
            data: Hex::from(vec![]),
            block_hash: None,
            transaction_index: None,
            log_index: None,
            removed: false,
        },
        LogEntry {
            address: Hex20::from([0; 20]),
            topics: vec![Hex32::from([0; 32])],
            transaction_hash: None,
            block_number: None,
            data: Hex::from(vec![]),
            block_hash: None,
            transaction_index: None,
            log_index: None,
            removed: false,
        },
    ];
    
    for _i in 1..100 {
        let log_entry_instance = LogEntry {
            address: Hex20::from([0; 20]),
            topics: vec![Hex32::from([0; 32])],
            transaction_hash: None,
            block_number: None,
            data: Hex::from(vec![]),
            block_hash: None,
            transaction_index: None,
            log_index: None,
            removed: false,
        };
        log_entries.push(log_entry_instance);
    }
    MultiRpcResult::Consistent(Ok(log_entries))
}
