use candid::Nat;
use evm_rpc_types::{
    BlockTag, GetLogsArgs, Hex20, Hex32, LogEntry, MultiRpcResult, RpcResult, Nat256
};
use futures::future::join_all;
use crate::types::balances::Balances;
use crate::{get_state_value, log};
use super::{utils::*, ChainConfig};
use ic_cdk::api::call::call_with_payment128;
use std::str::FromStr;

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
        *user_balance -= cycles_per_one_address.clone();
    }

}

pub async fn fetch_logs(
    chain_config: &ChainConfig,
    from_block: Nat,
    addresses: Option<Vec<String>>,
    topics: Option<Vec<Vec<String>>>,
) -> Result<Vec<LogEntry>, String> {
    let addresses = addresses.unwrap_or_default();
    let balance_before = ic_cdk::api::canister_balance();

    if addresses.is_empty() {
        return eth_get_logs_call_with_retry(
            chain_config,
            from_block.clone(),
            None,
            topics.clone(),
        )
        .await;
    }

    let events_per_interval = get_state_value!(estimate_events_num);  
    let chunk_size = calculate_request_chunk_size(events_per_interval, addresses.len() as u32); 

    let chunks_iter = addresses.chunks(chunk_size);

    let mut futures = vec![];

    for chunk in chunks_iter {
        let chunk_vec = chunk.to_vec();
        let topics_clone = topics.clone();
        let from_block = from_block.clone();

        let fut = async move {
            eth_get_logs_call_with_retry(
                chain_config,
                from_block.clone(),
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

async fn eth_get_logs_call_with_retry(
    chain_config: &ChainConfig,
    from_block: Nat,
    addresses: Option<Vec<String>>,
    topics: Option<Vec<Vec<String>>>,
) -> Result<Vec<LogEntry>, String> {
    // Convert addresses to Hex
    let addresses: Vec<Hex20> = addresses
        .unwrap_or_default()
        .into_iter()
        .map(|addr| {
            Hex20::from_str(&addr).map_err(|e| format!("Invalid address {}: {}", addr, e))
        })
        .collect::<Result<_, _>>()?;

    // Convert topics to Hex
    let topics: Option<Vec<Vec<Hex32>>> = topics
        .map(|outer| {
            outer
                .into_iter()
                .map(|inner| {
                    inner
                        .into_iter()
                        .map(|topic| {
                            Hex32::from_str(&topic)
                                .map_err(|e| format!("Invalid topic {}: {}", topic, e))
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;

    // Prepare arguments for the RPC call
    let get_logs_args = GetLogsArgs {
        from_block: Some(BlockTag::Number(Nat256::try_from(from_block.clone()).unwrap())),
        to_block: Some(BlockTag::Latest),
        addresses,
        topics,
    };

    let rpc_config = chain_config.rpc_config.clone();

    let cycles = 10_000_000_000;
    let max_retries = 2; // Set the maximum number of retries

    // Retry logic
    for attempt in 1..=max_retries {
        let result: Result<(MultiRpcResult<Vec<LogEntry>>,), _> = call_with_payment128(
            chain_config.evm_rpc_canister,
            "eth_getLogs",
            (chain_config.rpc_providers.clone(), Some(rpc_config.clone()), get_logs_args.clone()),
            cycles,
        )
        .await;

        match result {
            Ok((result,)) => match result {
                MultiRpcResult::Consistent(r) => match r {
                    RpcResult::Ok(logs) => return Ok(logs),
                    RpcResult::Err(err) => {
                        return Err(format!("GetLogsResult error: {:?}", err))
                    }
                },
                MultiRpcResult::Inconsistent(_) => {
                    if attempt == max_retries {
                        return Err("RPC providers gave inconsistent results".to_string());
                    }
                }
            },
            Err(e) => {
                if attempt == max_retries {
                    return Err(format!("Call failed after {} attempts: {:?}", attempt, e));
                }
            }
        }

        log!("Retrying... attempt {}/{}", attempt, max_retries);
    }

    Err("Failed to get logs after retries.".to_string())
}
