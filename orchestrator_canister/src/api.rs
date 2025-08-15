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

    // Forward cycles to chain service if provided; otherwise proceed without payment (for tests)
    let cycles = ic_cdk::api::call::msg_cycles_available128();
    let result: (Result<ChainRegisterSubscriptionResult, String>,) = if cycles > 0 {
        // Accept cycles so we can forward them
        ic_cdk::api::call::msg_cycles_accept128(cycles);
        ic_cdk::api::call::call_with_payment128(
            chain_service_id,
            "register_subscription",
            (registration,),
            cycles,
        ).await.map_err(|e| format!("Failed to call chain service: {:?}", e))?
    } else {
        ic_cdk::call(
            chain_service_id,
            "register_subscription",
            (registration,),
        ).await.map_err(|e| format!("Failed to call chain service: {:?}", e))?
    };
    
    let (subscription_id, estimated_cycles_per_day) = match result.0 {
        Ok(res) => (res.subscription_id, res.estimated_cycles_per_day),
        Err(err) => return Err(err),
    };
    
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
        estimated_cycles_per_day,
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

// Admin defaults management (called via lib with controller guard)
pub fn set_default_proxy_canister_id(proxy: Option<Principal>) -> Result<(), String> {
    mutate_state(|state| {
        state.default_proxy_canister_id = proxy;
        Ok(())
    })
}

pub fn set_default_evm_rpc_canister_id(evm_rpc: Option<Principal>) -> Result<(), String> {
    mutate_state(|state| {
        state.default_evm_rpc_canister_id = evm_rpc;
        Ok(())
    })
}

pub fn get_defaults() -> OrchestratorDefaults {
    read_state(|state| OrchestratorDefaults {
        default_proxy_canister_id: state.default_proxy_canister_id,
        default_evm_rpc_canister_id: state.default_evm_rpc_canister_id,
    })
}

// New: forward-cycles top up via orchestrator
pub async fn top_up_balance(chain_id: u32) -> Result<TopUpBalanceResult, String> {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        return Err("Anonymous users cannot top up".to_string());
    }

    let chain_service_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;

    let cycles = ic_cdk::api::call::msg_cycles_available128();
    if cycles == 0 {
        return Err("top_up_balance requires cycles attached; none provided".to_string());
    }
    ic_cdk::api::call::msg_cycles_accept128(cycles);

    let result: (Result<TopUpBalanceResult, String>,) = ic_cdk::api::call::call_with_payment128(
        chain_service_id,
        "top_up_balance",
        (caller,),
        cycles,
    ).await.map_err(|e| format!("Failed to call chain service top_up_balance: {:?}", e))?;

    result.0
}

// New: wrappers to operate through orchestrator only
pub async fn unsubscribe(chain_id: u32, subscription_id: Nat) -> Result<UnsubscribeResult, String> {
    let caller = ic_cdk::caller();
    let chain_service_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;

    let (res,): (Result<UnsubscribeResult, String>,) = ic_cdk::call(
        chain_service_id,
        "unsubscribe",
        (subscription_id, caller),
    ).await.map_err(|e| format!("Failed to call chain service unsubscribe: {:?}", e))?;

    res
}

pub async fn pause_subscription(chain_id: u32, subscription_id: Nat) -> Result<(), String> {
    let caller = ic_cdk::caller();
    let chain_service_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;

    let (res,): (Result<(), String>,) = ic_cdk::call(
        chain_service_id,
        "pause_subscription",
        (subscription_id, caller),
    ).await.map_err(|e| format!("Failed to call chain service pause_subscription: {:?}", e))?;

    res
}

pub async fn resume_subscription(chain_id: u32, subscription_id: Nat) -> Result<(), String> {
    let caller = ic_cdk::caller();
    let chain_service_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;

    let (res,): (Result<(), String>,) = ic_cdk::call(
        chain_service_id,
        "resume_subscription",
        (subscription_id, caller),
    ).await.map_err(|e| format!("Failed to call chain service resume_subscription: {:?}", e))?;

    res
}

pub async fn get_balance(chain_id: u32, user: Principal) -> Result<Nat, String> {
    let chain_service_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;

    let (balance,): (Nat,) = ic_cdk::call(
        chain_service_id,
        "get_balance",
        (user,),
    ).await.map_err(|e| format!("Failed to call chain service get_balance: {:?}", e))?;

    Ok(balance)
}

pub async fn get_user_subscriptions(chain_id: u32, user: Principal) -> Result<Vec<SubscriptionInfo>, String> {
    let chain_service_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;

    let (subs,): (Vec<SubscriptionInfo>,) = ic_cdk::call(
        chain_service_id,
        "get_user_subscriptions",
        (user,),
    ).await.map_err(|e| format!("Failed to call chain service get_user_subscriptions: {:?}", e))?;

    Ok(subs)
}
