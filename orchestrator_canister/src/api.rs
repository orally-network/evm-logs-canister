use candid::{Nat, Principal};
use crate::state::{read_state, mutate_state};
use crate::types::*;

pub fn register_user() -> Result<(), String> {
    let caller = ic_cdk::caller();
    
    if caller == Principal::anonymous() {
        return Err("Anonymous users cannot register".to_string());
    }
    
    mutate_state(|state| {
        let user_info = UserInfo {
            principal: caller,
            subscribed_chains: Vec::new(),
            registration_timestamp: ic_cdk::api::time(),
            last_activity_timestamp: ic_cdk::api::time(),
        };
        
        state.user_registry.insert(caller, user_info);
        Ok(())
    })
}

pub fn get_available_chain_services() -> Vec<ChainServiceInfo> {
    read_state(|state| {
        state.chain_services.values().cloned().collect()
    })
}

pub async fn subscribe_to_chain(chain_id: u32, filter: evm_logs_types::Filter) -> Result<SubscriptionResult, String> {
    let caller = ic_cdk::caller();
    
    if caller == Principal::anonymous() {
        return Err("Anonymous users cannot subscribe".to_string());
    }
    
    // Get chain service canister ID
    let chain_service_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;
    
    // Create subscription registration
    let registration = evm_logs_types::SubscriptionRegistration {
        chain_id,
        filter: filter.clone(),
        memo: None,
        canister_to_top_up: caller,
    };
    
    // Call chain service to register subscription
    let result: (Nat,) = ic_cdk::call(
        chain_service_id,
        "register_subscription",
        (registration,)
    ).await.map_err(|e| format!("Failed to call chain service: {:?}", e))?;
    
    let subscription_id = result.0;
    
    // Update user registry
    mutate_state(|state| {
        if let Some(user_info) = state.user_registry.get_mut(&caller) {
            if !user_info.subscribed_chains.contains(&chain_id) {
                user_info.subscribed_chains.push(chain_id);
            }
            user_info.last_activity_timestamp = ic_cdk::api::time();
        }
    });
    
    Ok(SubscriptionResult {
        subscription_id,
        chain_service_canister_id: chain_service_id,
    })
}

pub async fn batch_subscribe_to_chains(
    subscriptions: Vec<ChainSubscriptionRequest>,
) -> Result<BatchSubscriptionResult, String> {
    let mut successful = Vec::new();
    let mut failed = Vec::new();
    
    for request in subscriptions {
        match subscribe_to_chain(request.chain_id, request.filter.clone()).await {
            Ok(result) => successful.push(result),
            Err(error) => failed.push((request, error)),
        }
    }
    
    Ok(BatchSubscriptionResult {
        successful,
        failed,
    })
}

pub fn get_all_subscriptions() -> Vec<SubscriptionInfo> {
    let caller = ic_cdk::caller();
    
    // This is a placeholder - in reality, we'd need to query all chain services
    // For now, return empty vec as this would require async calls to all chain services
    Vec::new()
}

pub fn get_system_status() -> SystemStatus {
    read_state(|state| {
        let total_chain_services = state.chain_services.len() as u32;
        let active_chain_services = state.chain_services.values()
            .filter(|info| matches!(info.status, ChainServiceStatus::Active))
            .count() as u32;
        
        SystemStatus {
            total_chain_services,
            active_chain_services,
            total_subscriptions: 0, // Would need to query all chain services
            orchestrator_version: state.version.clone(),
        }
    })
}

pub fn get_chain_service_canister_id(chain_id: u32) -> Result<Principal, String> {
    read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })
}

pub fn get_orchestrator_info() -> OrchestratorInfo {
    read_state(|state| {
        OrchestratorInfo {
            admin: state.admin,
            chain_services: state.chain_services.values().cloned().collect(),
            version: state.version.clone(),
            total_users: state.user_registry.len() as u64,
        }
    })
}
