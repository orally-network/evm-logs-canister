use crate::state::mutate_state;
use crate::types::CycleUsageStats;
use candid::{Nat, Principal};

pub fn update_cycle_usage_stats(cycles_used: u64) {
    mutate_state(|state| {
        let stats = &mut state.cycle_usage_stats;
        
        stats.total_cycles_used += cycles_used;
        stats.last_execution_cycles = cycles_used;
        stats.execution_count += 1;
        
        if stats.execution_count > 0 {
            stats.average_cycles_per_block = stats.total_cycles_used / stats.execution_count;
        }
        
        stats.last_updated = ic_cdk::api::time();
    });
}

pub fn charge_subscribers_fairly(cycles_used: u64) {
    mutate_state(|state| {
        // First collect subscription IDs and subscriber principals to avoid borrowing conflicts
        let active_subscription_data: Vec<_> = state.subscriptions.iter()
            .filter(|(_, sub)| matches!(sub.status, crate::types::SubscriptionStatus::Active))
            .map(|(id, sub)| (id.clone(), sub.subscriber_principal))
            .collect();
        
        if active_subscription_data.is_empty() {
            return;
        }
        
        let cycles_per_subscriber = cycles_used / active_subscription_data.len() as u64;
        
        for (subscription_id, subscriber) in active_subscription_data {
            // Check if user has sufficient balance
            let current_balance = state.user_balances.get(&subscriber).cloned().unwrap_or(Nat::from(0u32));
            let cycles_nat = Nat::from(cycles_per_subscriber);
            
            if current_balance >= cycles_nat {
                // Deduct cycles
                let new_balance = current_balance - cycles_nat;
                state.user_balances.insert(subscriber, new_balance);
                
                // Update subscription stats
                if let Some(sub) = state.subscriptions.get_mut(&subscription_id) {
                    sub.cycles_consumed += cycles_per_subscriber;
                    sub.last_updated = ic_cdk::api::time();
                }
            } else {
                // Insufficient balance - mark subscription as such
                if let Some(sub) = state.subscriptions.get_mut(&subscription_id) {
                    sub.status = crate::types::SubscriptionStatus::InsufficientBalance {
                        since: ic_cdk::api::time(),
                    };
                    sub.last_updated = ic_cdk::api::time();
                }
                
                // TODO: Future notification system
                // notify_subscription_status_change(subscription_id, old_status, new_status).await;
            }
        }
    });
}

pub fn estimate_cycles_per_day_from_state(
    stats: &crate::types::CycleUsageStats,
    config: &crate::types::ChainConfig,
) -> u64 {
    let blocks_per_day = 86400 / config.block_interval_seconds; // seconds per day / block interval
    
    if stats.average_cycles_per_block > 0 {
        stats.average_cycles_per_block * blocks_per_day
    } else {
        // Default estimate if no data available
        1_000_000 * blocks_per_day // 1M cycles per block as default
    }
}

pub fn estimate_cycles_per_day() -> u64 {
    crate::state::read_state(|state| {
        estimate_cycles_per_day_from_state(&state.cycle_usage_stats, &state.config)
    })
}

pub async fn logs_fetching_and_processing_task() {
    // Record cycles before execution
    let cycles_before = ic_cdk::api::canister_balance();
    
    // Get active filters and addresses
    let (addresses, topics) = crate::subscriptions::get_active_addresses_and_topics();
    
    if addresses.is_empty() && topics.is_none() {
        return;
    }
    
    // Fetch logs from blockchain
    let last_processed_block = crate::state::read_state(|state| state.last_processed_block.clone());
    let config = crate::state::read_state(|state| state.config.clone());
    
    match fetch_logs(&config, last_processed_block + 1u32, Some(addresses), topics).await {
        Ok(logs) => {
            if !logs.is_empty() {
                // Process and publish events
                process_and_publish_events(logs).await;
            }
        },
        Err(e) => {
            ic_cdk::println!("Error during logs extraction: {}", e);
            // TODO: Update error stats
        }
    }
    
    // Record cycles after execution
    let cycles_after = ic_cdk::api::canister_balance();
    let cycles_used = cycles_before.saturating_sub(cycles_after);
    
    // Update cycle usage statistics
    update_cycle_usage_stats(cycles_used);
    
    // Charge subscribers fairly
    charge_subscribers_fairly(cycles_used);
}


async fn fetch_logs(
    config: &crate::types::ChainConfig,
    from_block: Nat,
    addresses: Option<Vec<String>>,
    topics: Option<Vec<Vec<String>>>,
) -> Result<Vec<evm_logs_types::LogEntry>, String> {
    crate::logs_fetcher::fetch_logs(from_block, addresses, topics).await
}

async fn process_and_publish_events(logs: Vec<evm_logs_types::LogEntry>) {
    crate::events_processor::process_and_publish_events(logs).await
}
