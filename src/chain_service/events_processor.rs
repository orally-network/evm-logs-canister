use super::service::ChainService;
use crate::subscription_manager::events_publisher::publish_events;
use candid::Nat;
use evm_logs_types::Event;
use evm_rpc_types::LogEntry;
use ic_cdk::api::time;

pub async fn process_and_publish_events(service: &ChainService, logs: Vec<LogEntry>) {
    let events: Vec<Event> = logs
        .iter()
        .enumerate()
        .map(|(index, log)| Event {
            id: Nat::from(index as u64 + 1),
            timestamp: time() / 1_000_000,
            chain_id: service.config.chain_id,
            log_entry: log.clone(),
        })
        .collect();

    publish_events(events).await;
}
