use std::str::FromStr;

use candid::{Encode, Nat};
use canister_utils::debug_log;
use evm_rpc_types::{BlockTag, GetLogsArgs, Hex20, Hex32, LogEntry, MultiRpcResult, Nat256, RpcResult};
use futures::future::join_all;
use ic_cdk::api::call::call_with_payment128;

use super::{ChainConfig, utils::*};
use crate::{
  constants::*,
  get_state_value, log_with_metrics,
  types::balances::{BalanceError, Balances},
};

fn estimate_log_entry_size(logs: &[LogEntry]) -> usize {
  logs.iter().map(|log| Encode!(log).unwrap().len()).sum()
}

fn estimate_cycles_used(
  logs_received: &[LogEntry],
  addresses_count: usize,
  topics_count: Option<&Vec<Vec<String>>>,
) -> u64 {
  log_with_metrics!("calculating cycles used for logs: {}", logs_received.len());
  // Estimate request size
  let request_size_bytes = 8 // Base struct size
        + (ETH_ADDRESS_SIZE as usize * addresses_count) // Address bytes
        + topics_count.map_or(0, |t| t.iter().map(|x| ETH_TOPIC_SIZE as usize * x.len()).sum()); // Topics bytes

  // Estimate response size based on received logs
  let response_size_bytes = estimate_log_entry_size(logs_received) as u64; // Logs in bytes

  // Compute cycles for sending request and receiving response
  let cycles_for_request = request_size_bytes as u64 * CYCLES_PER_BYTE_SEND;
  let cycles_for_response = response_size_bytes * CYCLES_PER_BYTE_RECEIVE;
  log_with_metrics!("cycles_for_response actual: {}", cycles_for_response);
  // log!("cycles_for_response theoretical: {}", cycles_for_response);

  // Total cycles usage including base call cost and multiple RPC queries
  let total_cycles_used =
    BASE_CALL_CYCLES + cycles_for_request + cycles_for_response + (cycles_for_request + cycles_for_response);
  total_cycles_used
}

fn charge_subscribers(addresses_amound: usize, cycles_used: u64) {
  let subscriptions = get_state_value!(subscriptions);

  // charge subscribers accordingly to amount addresses in their filters
  let cycles_per_one_address = Nat::from(cycles_used / addresses_amound as u64);

  for (_sub_id, sub_info) in subscriptions.iter() {
    let subscriber_principal = sub_info.subscriber_principal;
    match Balances::reduce(&subscriber_principal, cycles_per_one_address.clone()) {
      Ok(_) => {}
      Err(BalanceError::BalanceDoesNotExist) => {
        debug_log!(
          "Failed to reduce balance: Balance does not exist for {:?}",
          subscriber_principal.to_text()
        );
      }
      Err(BalanceError::InsufficientBalance) => {
        debug_log!(
          "Failed to reduce balance: Insufficient balance for {:?}",
          subscriber_principal.to_text()
        );
      }
    }
  }
}

pub async fn fetch_logs(
  chain_config: &ChainConfig,
  from_block: Nat,
  addresses: Option<Vec<String>>,
  topics: Option<Vec<Vec<String>>>,
) -> Result<Vec<LogEntry>, String> {
  let addresses = addresses.unwrap_or_default();

  if addresses.is_empty() {
    return eth_get_logs_call_with_retry(chain_config, from_block.clone(), None, topics.clone()).await;
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
      eth_get_logs_call_with_retry(chain_config, from_block.clone(), Some(chunk_vec), topics_clone).await
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

  let total_cycles_used = estimate_cycles_used(&merged_logs, addresses.len(), topics.as_ref());
  // log!("cycles used in theoretical estimate: {}", total_cycles_used1);
  // log!("cycles_for_response actual {}", total_cycles_used2);

  // after sending request we need to charge cycles for each subscriber accordingly
  // to amount of their subscribtion addresses(filters)
  // note: later events_publisher will charge cycles accordingly to amount of logs received by each subscriber
  charge_subscribers(addresses.len(), total_cycles_used);

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
    .map(|addr| Hex20::from_str(&addr).map_err(|e| format!("Invalid address {}: {}", addr, e)))
    .collect::<Result<_, _>>()?;

  // Convert topics to Hex
  let topics: Option<Vec<Vec<Hex32>>> = topics
    .map(|outer| {
      outer
        .into_iter()
        .map(|inner| {
          inner
            .into_iter()
            .map(|topic| Hex32::from_str(&topic).map_err(|e| format!("Invalid topic {}: {}", topic, e)))
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

  let cycles = 10_000_000_000; // TODO
  let max_retries = 2; // Set the maximum number of retries

  // Retry logic
  for attempt in 1..=max_retries {
    log_with_metrics!("calling eth_getLogs, attempt {}", attempt);
    let result: Result<(MultiRpcResult<Vec<LogEntry>>,), _> = call_with_payment128(
      chain_config.evm_rpc_canister,
      "eth_getLogs",
      (
        chain_config.rpc_providers.clone(),
        rpc_config.clone(),
        get_logs_args.clone(),
      ),
      cycles,
    )
    .await;

    match result {
      Ok((result,)) => match result {
        MultiRpcResult::Consistent(r) => match r {
          RpcResult::Ok(logs) => return Ok(logs),
          RpcResult::Err(err) => return Err(format!("GetLogsResult error: {:?}", err)),
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

    log_with_metrics!("Retrying... attempt {}/{}", attempt, max_retries);
  }

  Err("Failed to get logs after retries.".to_string())
}
