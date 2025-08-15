use candid::{Nat, Principal};
use crate::state::{read_state, mutate_state};
use crate::types::*;
use crate::cycle_tracking::estimate_cycles_per_day;

pub fn top_up_balance(user: Principal) -> Result<TopUpBalanceResult, String> {
    if user == Principal::anonymous() {
        return Err("Anonymous users cannot top up balance".to_string());
    }

    // Get cycles sent with this call
    let cycles_received = ic_cdk::api::call::msg_cycles_available128();

    if cycles_received == 0 {
        return Err("No cycles sent with the call".to_string());
    }

    // Accept the cycles
    ic_cdk::api::call::msg_cycles_accept128(cycles_received);

    // Update user balance
    let new_balance = mutate_state(|state: &mut ChainServiceState| {
        let current_balance = state.user_balances.get(&user).cloned().unwrap_or(Nat::from(0u32));
        let new_balance = current_balance + Nat::from(cycles_received);
        state.user_balances.insert(user, new_balance.clone());
        new_balance
    });

    Ok(TopUpBalanceResult {
        new_balance,
        cycles_received: u64::try_from(cycles_received).unwrap_or(u64::MAX),
    })
}

pub fn get_balance(user: Principal) -> Nat {
    read_state(|state| {
        state.user_balances.get(&user).cloned().unwrap_or(Nat::from(0u32))
    })
}

pub async fn register_subscription(
    registration: evm_logs_types::SubscriptionRegistration,
) -> Result<RegisterSubscriptionResult, String> {
    let caller = ic_cdk::caller();
    
    if caller == Principal::anonymous() {
        return Err("Anonymous users cannot register subscriptions".to_string());
    }
    
    // Validate that the chain_id matches this service
    let chain_id = read_state(|state| state.config.chain_id);
    if registration.chain_id != chain_id {
        return Err(format!("Chain ID mismatch: expected {}, got {}", chain_id, registration.chain_id));
    }
    
    // Credit any cycles attached to this call to the subscriber balance
    let cycles_received = ic_cdk::api::call::msg_cycles_available128();
    if cycles_received > 0 {
        ic_cdk::api::call::msg_cycles_accept128(cycles_received);
        mutate_state(|state: &mut ChainServiceState| {
            let current = state.user_balances.get(&registration.canister_to_top_up).cloned().unwrap_or(Nat::from(0u32));
            let new_balance = current + Nat::from(cycles_received);
            state.user_balances.insert(registration.canister_to_top_up, new_balance);
        });
    }

    // Create subscription
    let (subscription_id, estimated_cycles) = mutate_state(|state| {
        let subscription_id = state.next_subscription_id.clone();
        state.next_subscription_id += 1u32;
        
        let subscription_info = SubscriptionInfo {
            subscription_id: subscription_id.clone(),
            subscriber_principal: registration.canister_to_top_up,
            chain_id: registration.chain_id,
            filter: registration.filter,
            status: SubscriptionStatus::Active,
            created_at: ic_cdk::api::time(),
            last_updated: ic_cdk::api::time(),
            cycles_consumed: 0,
            events_received: 0,
        };
        
        state.subscriptions.insert(subscription_id.clone(), subscription_info);
        
        let estimated_cycles = crate::cycle_tracking::estimate_cycles_per_day_from_state(
            &state.cycle_usage_stats,
            &state.config,
        );
        (subscription_id, estimated_cycles)
    });
    
    Ok(RegisterSubscriptionResult {
        subscription_id,
        estimated_cycles_per_day: estimated_cycles,
    })
}

pub async fn batch_register_subscriptions(
    registrations: Vec<evm_logs_types::SubscriptionRegistration>,
) -> Result<Vec<RegisterSubscriptionResult>, String> {
    let caller = ic_cdk::caller();
    
    if caller == Principal::anonymous() {
        return Err("Anonymous users cannot register subscriptions".to_string());
    }
    
    // Validate all registrations first
    let chain_id = read_state(|state| state.config.chain_id);
    for registration in &registrations {
        if registration.chain_id != chain_id {
            return Err(format!("Chain ID mismatch: expected {}, got {}", chain_id, registration.chain_id));
        }
    }
    
    // Credit any cycles attached to this call to the subscriber balance (first registration)
    let cycles_received = ic_cdk::api::call::msg_cycles_available128();
    if cycles_received > 0 {
        ic_cdk::api::call::msg_cycles_accept128(cycles_received);
        if let Some(first) = registrations.first() {
            mutate_state(|state: &mut ChainServiceState| {
                let current = state.user_balances.get(&first.canister_to_top_up).cloned().unwrap_or(Nat::from(0u32));
                let new_balance = current + Nat::from(cycles_received);
                state.user_balances.insert(first.canister_to_top_up, new_balance);
            });
        }
    }

    // Process all registrations in a single state mutation
    let results = mutate_state(|state| {
        let mut results = Vec::new();
        
        for registration in registrations {
            let subscription_id = state.next_subscription_id.clone();
            state.next_subscription_id += 1u32;
            
            let subscription_info = SubscriptionInfo {
                subscription_id: subscription_id.clone(),
                subscriber_principal: registration.canister_to_top_up,
                chain_id: registration.chain_id,
                filter: registration.filter,
                status: SubscriptionStatus::Active,
                created_at: ic_cdk::api::time(),
                last_updated: ic_cdk::api::time(),
                cycles_consumed: 0,
                events_received: 0,
            };
            
            state.subscriptions.insert(subscription_id.clone(), subscription_info);
            
            let estimated_cycles = crate::cycle_tracking::estimate_cycles_per_day_from_state(
                &state.cycle_usage_stats,
                &state.config,
            );
            results.push(RegisterSubscriptionResult {
                subscription_id,
                estimated_cycles_per_day: estimated_cycles,
            });
        }
        
        results
    });
    
    Ok(results)
}

pub fn unsubscribe(subscription_id: Nat, user: Principal) -> Result<UnsubscribeResult, String> {
    if user == Principal::anonymous() {
        return Err("Anonymous users cannot unsubscribe".to_string());
    }

    mutate_state(|state| {
        // Check if subscription exists and belongs to the specified user
        let subscription = state.subscriptions.get(&subscription_id)
            .ok_or_else(|| "Subscription not found".to_string())?;
        
        if subscription.subscriber_principal != user {
            return Err("Access denied: subscription belongs to another user".to_string());
        }
        
        // Remove subscription
        state.subscriptions.remove(&subscription_id);
        
        // For now, no refund - in a real implementation, you might refund unused cycles
        Ok(UnsubscribeResult {
            refunded_cycles: Nat::from(0u32),
        })
    })
}

pub fn batch_unsubscribe(subscription_ids: Vec<Nat>, user: Principal) -> Result<Vec<UnsubscribeResult>, String> {
    if user == Principal::anonymous() {
        return Err("Anonymous users cannot unsubscribe".to_string());
    }
    
    // Process all unsubscriptions in a single state mutation
    mutate_state(|state| {
        let mut results = Vec::new();
        
        for subscription_id in subscription_ids {
            // Check if subscription exists and belongs to the specified user
            let subscription = state.subscriptions.get(&subscription_id)
                .ok_or_else(|| format!("Subscription {} not found", subscription_id))?;
            
            if subscription.subscriber_principal != user {
                return Err(format!("Access denied: subscription {} belongs to another user", subscription_id));
            }
            
            // Remove subscription
            state.subscriptions.remove(&subscription_id);
            
            // For now, no refund - in a real implementation, you might refund unused cycles
            results.push(UnsubscribeResult {
                refunded_cycles: Nat::from(0u32),
            });
        }
        
        Ok(results)
    })
}

pub fn pause_subscription(subscription_id: Nat, user: Principal) -> Result<(), String> {
    if user == Principal::anonymous() {
        return Err("Anonymous users cannot pause subscriptions".to_string());
    }
    
    mutate_state(|state| {
        let subscription = state.subscriptions.get_mut(&subscription_id)
            .ok_or_else(|| "Subscription not found".to_string())?;
        
        if subscription.subscriber_principal != user {
            return Err("Access denied: subscription belongs to another user".to_string());
        }
        
        subscription.status = SubscriptionStatus::PausedByUser {
            since: ic_cdk::api::time(),
        };
        subscription.last_updated = ic_cdk::api::time();
        
        Ok(())
    })
}

pub fn resume_subscription(subscription_id: Nat, user: Principal) -> Result<(), String> {
    if user == Principal::anonymous() {
        return Err("Anonymous users cannot resume subscriptions".to_string());
    }
    
    mutate_state(|state| {
        let subscription = state.subscriptions.get_mut(&subscription_id)
            .ok_or_else(|| "Subscription not found".to_string())?;
        
        if subscription.subscriber_principal != user {
            return Err("Access denied: subscription belongs to another user".to_string());
        }
        
        subscription.status = SubscriptionStatus::Active;
        subscription.last_updated = ic_cdk::api::time();
        
        Ok(())
    })
}

pub fn get_user_subscriptions(user: Principal) -> Vec<SubscriptionInfo> {
    read_state(|state| {
        state.subscriptions.values()
            .filter(|sub| sub.subscriber_principal == user)
            .cloned()
            .collect()
    })
}

pub fn get_user_subscriptions_with_status(user: Principal) -> Vec<SubscriptionInfo> {
    // Same as get_user_subscriptions since status is already included
    get_user_subscriptions(user)
}

pub fn get_subscription_status(subscription_id: Nat) -> Result<SubscriptionStatus, String> {
    read_state(|state| {
        state.subscriptions.get(&subscription_id)
            .map(|sub| sub.status.clone())
            .ok_or_else(|| "Subscription not found".to_string())
    })
}

pub fn get_cycle_usage_stats() -> CycleUsageStats {
    read_state(|state| state.cycle_usage_stats.clone())
}

pub fn get_monitoring_status() -> MonitoringStatus {
    read_state(|state| {
        MonitoringStatus {
            is_monitoring: state.timer_id.is_some(),
            last_processed_block: state.last_processed_block.clone(),
            timer_id: state.timer_id.clone(),
            monitoring_interval_seconds: state.config.block_interval_seconds,
        }
    })
}

pub fn get_health_status() -> HealthStatus {
    read_state(|state| {
        // This is a basic health check - in a real implementation, you'd track more metrics
        HealthStatus {
            chain_id: state.config.chain_id,
            is_healthy: true, // TODO: Implement actual health checks
            last_successful_fetch: None, // TODO: Track last successful fetch
            consecutive_failures: 0, // TODO: Track failures
            error_message: None,
        }
    })
}

pub fn set_orchestrator(orchestrator: Principal) -> Result<(), String> {
    mutate_state(|state| {
        state.orchestrator = orchestrator;
        Ok(())
    })
}

pub fn update_config(config: ChainConfig) -> Result<(), String> {
    mutate_state(|state| {
        state.config = config;
        Ok(())
    })
}

pub fn update_proxy_canister_id(proxy_canister_id: Option<Principal>) -> Result<(), String> {
    mutate_state(|state| {
        state.config.proxy_canister_id = proxy_canister_id;
        Ok(())
    })
}

pub fn update_evm_rpc_canister_id(evm_rpc_canister_id: Option<Principal>) -> Result<(), String> {
    mutate_state(|state| {
        state.config.evm_rpc_canister_id = evm_rpc_canister_id;
        Ok(())
    })
}

pub fn update_rpc_service_config(rpc_service: RpcServiceConfig) -> Result<(), String> {
    mutate_state(|state| {
        state.config.rpc_service = rpc_service;
        Ok(())
    })
}
