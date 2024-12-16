use super::service::ChainService;
use crate::subscription_manager::events_publisher::publish_events;
use candid::Nat;
use evm_logs_types::{Event, ICRC16Value};
use evm_rpc_canister_types::LogEntry;
use ic_cdk::api::time;

pub async fn process_events(service: &ChainService, logs: Vec<LogEntry>) -> Result<(), String> {
    let events: Vec<Event> = logs
        .iter()
        .enumerate()
        .map(|(index, log)| Event {
            id: Nat::from(index as u64 + 1),
            prev_id: None,
            timestamp: time() / 1_000_000,
            namespace: format!("com.events.{:?}", service.config.chain_name),
            data: ICRC16Value::Text(log.data.clone()),
            address: log.address.clone(),
            topics: Some(log.topics.clone()),
            tx_hash: log.transactionHash.clone().unwrap(),
            headers: None,
        })
        .collect();

    let publish_result = publish_events(events).await;

    for opt_result in publish_result {
        match opt_result {
            Some(Ok(event_ids)) => {
                ic_cdk::println!(
                    "Event published and sent to subscribers with Event IDs: {:?}",
                    event_ids
                );
            }
            Some(Err(error)) => {
                ic_cdk::println!("Failed to publish or send event: {:?}", error);
            }
            None => {
                ic_cdk::println!("Event was not published (no result available).");
            }
        }
    }
    Ok(())
}
