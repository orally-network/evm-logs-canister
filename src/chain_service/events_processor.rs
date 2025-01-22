use super::service::ChainService;
use crate::{
    log,
    subscription_manager::events_publisher::publish_events,
};
use candid::Nat;
use evm_logs_types::{Event, Value};
use evm_rpc_types::LogEntry;
use ic_cdk::api::time;

pub async fn process_events(service: &ChainService, logs: Vec<LogEntry>) -> Result<(), String> {
    let events: Vec<Event> = logs
        .iter()
        .enumerate()
        .map(|(index, log)| Event {
            id: Nat::from(index as u64 + 1),
            prev_id: None,
            timestamp: time() / 1_000_000,
            chain_id: service.config.chain_id,
            data: Value::Text(log.data.clone().to_string()),
            address: log.address.clone().to_string(),
            topics: Some(
                log.topics
                   .iter()
                   .map(|topic| topic.to_string())
                   .collect()
            ),
            tx_hash: log.transaction_hash.clone().unwrap().to_string(),
            headers: None,
        })
        .collect();

    let publish_result = publish_events(events).await;

    for opt_result in publish_result {
        match opt_result {
            Some(Ok(event_ids)) => {
                log!(
                    "Event published and sent to subscribers with Event IDs: {:?}",
                    event_ids
                );
            }
            Some(Err(error)) => {
                log!("Failed to publish or send event: {:?}", error);
            }
            None => {
                log!("Event was not published (no result available).");
            }
        }
    }
    Ok(())
}
