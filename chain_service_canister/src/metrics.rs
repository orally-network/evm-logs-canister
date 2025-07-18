use crate::http_types::{HttpRequest, HttpResponse, HttpResponseBuilder};
use ic_metrics_encoder::MetricsEncoder;
use crate::state::read_state;
use crate::types::*;
use num_traits::{ToPrimitive};

pub fn http_request(req: HttpRequest) -> HttpResponse {
    match req.path() {
        "/metrics" => handle_metrics_request(),
        _ => HttpResponseBuilder::not_found().build(),
    }
}

fn handle_metrics_request() -> HttpResponse {
    let mut writer = MetricsEncoder::new(vec![], ic_cdk::api::time() as i64 / 1_000_000);
    
    match encode_chain_service_metrics(&mut writer) {
        Ok(()) => HttpResponseBuilder::ok()
            .header("Content-Type", "text/plain; version=0.0.4")
            .with_body_and_content_length(writer.into_inner())
            .build(),
        Err(err) => HttpResponseBuilder::server_error(format!("Failed to encode metrics: {}", err))
            .build(),
    }
}

fn encode_chain_service_metrics(writer: &mut MetricsEncoder<Vec<u8>>) -> std::io::Result<()> {
    const WASM_PAGE_SIZE_IN_BYTES: f64 = 65536.0;
    
    // Basic canister metrics
    writer.encode_gauge(
        "stable_memory_bytes",
        ic_cdk::api::stable::stable_size() as f64 * WASM_PAGE_SIZE_IN_BYTES,
        "Size of the stable memory allocated by this canister.",
    )?;
    
    writer.encode_gauge(
        "heap_memory_bytes",
        heap_memory_size_bytes() as f64,
        "Size of the heap memory allocated by this canister.",
    )?;
    
    writer.gauge_vec("cycle_balance", "Cycle balance of this canister.")?
        .value(
            &[("canister", "chain_service")],
            ic_cdk::api::canister_balance128() as f64,
        )?;
    
    // Chain service specific metrics
    read_state(|state| {
        let chain_id_str = state.config.chain_id.to_string();
        let chain_name = &state.config.chain_name;
        
        // Basic chain info
        writer.gauge_vec("chain_service_info", "Information about this chain service.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                1.0,
            )?;
        
        // Subscription metrics
        let total_subscriptions = state.subscriptions.len() as f64;
        let active_subscriptions = state.subscriptions.values()
            .filter(|sub| matches!(sub.status, SubscriptionStatus::Active))
            .count() as f64;
        let paused_subscriptions = state.subscriptions.values()
            .filter(|sub| matches!(sub.status, SubscriptionStatus::PausedByUser { .. }))
            .count() as f64;
        let insufficient_balance_subscriptions = state.subscriptions.values()
            .filter(|sub| matches!(sub.status, SubscriptionStatus::InsufficientBalance { .. }))
            .count() as f64;
        
        writer.gauge_vec("chain_service_subscriptions_total", "Total number of subscriptions.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                total_subscriptions,
            )?;
        
        writer.gauge_vec("chain_service_subscriptions_active", "Number of active subscriptions.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                active_subscriptions,
            )?;
        
        writer.gauge_vec("chain_service_subscriptions_paused", "Number of paused subscriptions.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                paused_subscriptions,
            )?;
        
        writer.gauge_vec("chain_service_subscriptions_insufficient_balance", "Number of subscriptions with insufficient balance.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                insufficient_balance_subscriptions,
            )?;
        
        // Cycle usage metrics
        let stats = &state.cycle_usage_stats;
        writer.gauge_vec("chain_service_cycles_used_total", "Total cycles used by this chain service.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                stats.total_cycles_used as f64,
            )?;
        
        writer.gauge_vec("chain_service_cycles_last_execution", "Cycles used in last execution.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                stats.last_execution_cycles as f64,
            )?;
        
        writer.gauge_vec("chain_service_cycles_average_per_block", "Average cycles used per block.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                stats.average_cycles_per_block as f64,
            )?;
        
        writer.gauge_vec("chain_service_execution_count", "Number of executions performed.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                stats.execution_count as f64,
            )?;
        
        // Block processing metrics
        writer.gauge_vec("chain_service_last_processed_block", "Last processed block number.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                state.last_processed_block.0.to_f64().unwrap_or(0.0),
            )?;
        
        // User balance metrics
        let total_users = state.user_balances.len() as f64;
        let total_deposited_cycles: u128 = state.user_balances.values()
            .map(|balance| balance.0.to_u128().unwrap_or(0))
            .sum();
        let average_balance = if total_users > 0.0 {
            total_deposited_cycles as f64 / total_users
        } else {
            0.0
        };
        
        writer.gauge_vec("chain_service_users_total", "Total number of users with balances.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                total_users,
            )?;
        
        writer.gauge_vec("chain_service_total_deposited_cycles", "Total cycles deposited by all users.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                total_deposited_cycles as f64,
            )?;
        
        writer.gauge_vec("chain_service_average_user_balance", "Average user balance in cycles.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                average_balance,
            )?;
        
        // Monitoring status
        let is_monitoring = if state.timer_id.is_some() { 1.0 } else { 0.0 };
        writer.gauge_vec("chain_service_monitoring_active", "Whether monitoring is active (1=active, 0=inactive).")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                is_monitoring,
            )?;
        
        writer.gauge_vec("chain_service_monitoring_interval_seconds", "Monitoring interval in seconds.")?
            .value(
                &[("chain_id", &chain_id_str), ("chain_name", chain_name)],
                state.config.block_interval_seconds as f64,
            )?;
        
        Ok::<(), std::io::Error>(())
    })?;
    
    Ok(())
}

/// Returns the amount of heap memory in bytes that has been allocated.
#[cfg(target_arch = "wasm32")]
pub fn heap_memory_size_bytes() -> usize {
    const WASM_PAGE_SIZE_BYTES: usize = 65536;
    core::arch::wasm32::memory_size(0) * WASM_PAGE_SIZE_BYTES
}

#[cfg(not(any(target_arch = "wasm32")))]
pub fn heap_memory_size_bytes() -> usize {
    0
}

pub fn get_chain_service_metrics() -> ChainServiceMetrics {
    read_state(|state| {
        let subscription_count = state.subscriptions.len() as u64;
        let active_subscription_count = state.subscriptions.values()
            .filter(|sub| matches!(sub.status, SubscriptionStatus::Active))
            .count() as u64;
        
        // Calculate user balance stats
        let total_users = state.user_balances.len();
        let total_deposited_cycles: u128 = state.user_balances.values()
            .map(|balance| balance.0.to_u128().unwrap_or(0))
            .sum();
        let average_user_balance = if total_users > 0 {
            (total_deposited_cycles / total_users as u128) as u64
        } else {
            0
        };
        let users_with_insufficient_balance = state.subscriptions.values()
            .filter(|sub| matches!(sub.status, SubscriptionStatus::InsufficientBalance { .. }))
            .map(|sub| sub.subscriber_principal)
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;
        
        ChainServiceMetrics {
            chain_id: state.config.chain_id,
            chain_name: state.config.chain_name.clone(),
            subscription_count,
            active_subscription_count,
            cycle_usage_stats: state.cycle_usage_stats.clone(),
            block_processing_stats: BlockProcessingStats {
                last_processed_block: state.last_processed_block.clone(),
                blocks_processed_total: state.cycle_usage_stats.execution_count,
                average_processing_time_ms: 0, // TODO: Track this
                logs_fetched_total: 0, // TODO: Track this
                events_published_total: 0, // TODO: Track this
            },
            error_stats: ErrorStats::default(), // TODO: Implement error tracking
            user_balance_stats: UserBalanceStats {
                total_deposited_cycles,
                total_consumed_cycles: 0, // TODO: Track this
                average_user_balance,
                users_with_insufficient_balance,
            },
            cycle_balance: ic_cdk::api::canister_balance128(),
        }
    })
}
