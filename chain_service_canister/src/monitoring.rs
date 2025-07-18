use ic_cdk_timers::{set_timer_interval, clear_timer, TimerId};
use std::time::Duration;
use crate::state::{read_state, mutate_state};
use crate::types::TIMER_ID;
use crate::cycle_tracking::logs_fetching_and_processing_task;

pub fn start_monitoring() -> Result<(), String> {
    let interval_seconds = read_state(|state| state.config.block_interval_seconds);
    
    // Clear existing timer if any
    stop_monitoring()?;
    
    // Set up new timer
    let timer_id = set_timer_interval(
        Duration::from_secs(interval_seconds),
        || {
            ic_cdk::spawn(async {
                logs_fetching_and_processing_task().await;
            });
        }
    );
    
    // Store timer ID in both runtime and persistent storage
    TIMER_ID.with(|id| {
        *id.borrow_mut() = Some(timer_id);
    });
    
    mutate_state(|state| {
        state.timer_id = Some(format!("{:?}", timer_id));
    });
    
    ic_cdk::println!("Started monitoring for chain {}", read_state(|state| state.config.chain_id));
    Ok(())
}

pub fn stop_monitoring() -> Result<(), String> {
    // Clear runtime timer
    TIMER_ID.with(|id| {
        if let Some(timer_id) = id.borrow_mut().take() {
            clear_timer(timer_id);
        }
    });
    
    // Clear persistent storage
    mutate_state(|state| {
        state.timer_id = None;
    });
    
    ic_cdk::println!("Stopped monitoring for chain {}", read_state(|state| state.config.chain_id));
    Ok(())
}

pub fn is_monitoring() -> bool {
    TIMER_ID.with(|id| id.borrow().is_some())
}

// This function should be called during post_upgrade to restore monitoring
pub fn restore_monitoring_after_upgrade() {
    let should_restore = read_state(|state| state.timer_id.is_some());
    
    if should_restore {
        if let Err(e) = start_monitoring() {
            ic_cdk::println!("Failed to restore monitoring after upgrade: {}", e);
        } else {
            ic_cdk::println!("Successfully restored monitoring after upgrade");
        }
    }
}
