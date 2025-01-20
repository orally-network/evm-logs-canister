use candid::Nat;
use evm_rpc_canister_types::{
    BlockTag, EvmRpcCanister, GetLogsArgs, GetLogsResult, LogEntry, MultiGetLogsResult, RpcServices,
};
use futures::future::join_all;
use crate::types::balances::Balances;
use crate::{get_state_value, log};

use super::utils::*;

fn charge_subscribers(addresses_amound: usize, cycles_used: u64) {
    let subscribers = get_state_value!(subscriptions);
    let mut user_balances = get_state_value!(user_balances);
    // charge subscribers accordingly to amount addresses in their filters
    let cycles_per_one_address = Nat::from(cycles_used / addresses_amound as u64);

    for (_sub_id, sub_info) in subscribers.iter() {
        let subscriber_principal = sub_info.subscriber_principal;
        let user_balance = user_balances.balances.get_mut(&subscriber_principal).unwrap();

        if Balances::is_sufficient(subscriber_principal, cycles_per_one_address.clone()).unwrap() {
            Balances::reduce(&subscriber_principal, cycles_per_one_address.clone()).unwrap();
        }
        user_balance.amount -= cycles_per_one_address.clone();

    }

}

pub async fn fetch_logs(
    evm_rpc: &EvmRpcCanister,
    rpc_providers: &RpcServices,
    from_block: u64,
    addresses: Option<Vec<String>>,
    topics: Option<Vec<Vec<String>>>,
) -> Result<Vec<LogEntry>, String> {
    let addresses = addresses.unwrap_or_default();
    let balance_before = ic_cdk::api::canister_balance();

    if addresses.is_empty() {
        return single_eth_get_logs_call(
            evm_rpc,
            rpc_providers,
            from_block,
            None,
            topics.clone(),
        )
        .await;
    }

    let events_per_interval = get_state_value!(estimate_events_num);  
    let chunk_size = calculate_request_chunk_size(events_per_interval, addresses.len() as u32); 

    let chunks_iter = addresses.chunks(chunk_size);

    let mut futures = vec![];

    for (_i, chunk) in chunks_iter.enumerate() {
        let chunk_vec = chunk.to_vec();
        let evm_rpc_clone = evm_rpc.clone();
        let rpc_providers_clone = rpc_providers.clone();
        let topics_clone = topics.clone();

        let fut = async move {
            single_eth_get_logs_call(
                &evm_rpc_clone,
                &rpc_providers_clone,
                from_block,
                Some(chunk_vec),
                topics_clone,
            )
            .await
        };
        futures.push(fut);
    }

    let results = join_all(futures).await;

    let mut merged_logs = Vec::new();
    for res in results {
        match res {
            Ok(logs) => merged_logs.extend(logs),
            Err(e) => return Err(e),
        }
    }
    let balance_after = ic_cdk::api::canister_balance();

    let cycles_used = balance_before - balance_after;
    
    log!(
        "Cost for logs fetching request: {}",
        balance_before - balance_after
    );
    // after sending request we need to charge cycles for each subscriber accordingly 
    // to amount of their subscribtion addresses(filters)
    // note: later events_publisher will charge cycles accordingly to amount of logs received by each subscriber

    charge_subscribers(addresses.len(), cycles_used);
    
    Ok(merged_logs)

}

async fn single_eth_get_logs_call(
    evm_rpc: &EvmRpcCanister,
    rpc_providers: &RpcServices,
    from_block: u64,
    addresses: Option<Vec<String>>,
    topics: Option<Vec<Vec<String>>>,
) -> Result<Vec<LogEntry>, String> {
    let get_logs_args = GetLogsArgs {
        fromBlock: Some(BlockTag::Number(Nat::from(from_block))),
        toBlock: Some(BlockTag::Latest),
        addresses: addresses.unwrap_or_default(),
        topics,
    };

    let cycles = 10_000_000_000;
    let (result,) = evm_rpc
        .eth_get_logs(rpc_providers.clone(), None, get_logs_args, cycles)
        .await
        .map_err(|e| format!("Call failed: {:?}", e))?;

    match result {
        MultiGetLogsResult::Consistent(r) => match r {
            GetLogsResult::Ok(logs) => Ok(logs),
            GetLogsResult::Err(err) => Err(format!("GetLogsResult error: {:?}", err)),
        },
        MultiGetLogsResult::Inconsistent(_) => {
            Err("RPC providers gave inconsistent results".to_string())
        }
    }
}