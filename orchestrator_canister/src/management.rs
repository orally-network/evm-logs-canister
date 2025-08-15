use candid::Principal;
use ic_cdk::api::management_canister::main::{
    create_canister, install_code, CanisterSettings, CreateCanisterArgument,
    InstallCodeArgument, CanisterInstallMode,
};
use crate::state::{read_state, mutate_state};
use crate::types::*;

const MAX_PREVIOUS_WASMS: usize = 5;

// Chain service types that the chain service canister expects
#[derive(candid::CandidType, serde::Deserialize, Clone)]
struct ChainConfig {
    chain_id: u32,
    chain_name: String,
    rpc_url: String,
    block_interval_seconds: u64,
    max_response_bytes: u64,
    proxy_canister_id: Option<Principal>,
    evm_rpc_canister_id: Option<Principal>,
    rpc_service: RpcServiceConfig,
}

#[derive(candid::CandidType, serde::Deserialize, Clone)]
enum RpcServiceConfig {
    EthMainnet { providers: Option<Vec<String>> },
    EthSepolia { providers: Option<Vec<String>> },
    ArbitrumOne { providers: Option<Vec<String>> },
    BaseMainnet { providers: Option<Vec<String>> },
    OptimismMainnet { providers: Option<Vec<String>> },
    Custom { 
        rpc_url: String,
        chain_id: u64,
    },
}


pub async fn deploy_chain_service(
    chain_id: u32,
    chain_name: String,
    config: ChainServiceConfig,
) -> Result<Principal, String> {
    // Check if chain service already exists
    let existing = read_state(|state| {
        state.chain_services.get(&chain_id).is_some()
    });
    
    if existing {
        return Err(format!("Chain service for chain ID {} already exists", chain_id));
    }
    
    // Get WASM module
    let wasm = read_state(|state| {
        state.chain_service_wasm.clone()
            .ok_or_else(|| "No ChainService WASM module available".to_string())
    })?;
    
    // Create canister
    let create_args = CreateCanisterArgument {
        settings: Some(CanisterSettings {
            controllers: Some(vec![ic_cdk::id()]),
            compute_allocation: None,
            memory_allocation: None,
            freezing_threshold: None,
            reserved_cycles_limit: None,
            log_visibility: None,
            wasm_memory_limit: None,
        }),
    };
    
    let (canister_record,) = create_canister(create_args, 1_000_000_000_000u128)
        .await
        .map_err(|e| format!("Failed to create canister: {:?}", e))?;
    
    let canister_id = canister_record.canister_id;
    
    // Resolve orchestrator defaults
    let (default_proxy, default_evm_rpc) = read_state(|state| {
        (state.default_proxy_canister_id, state.default_evm_rpc_canister_id)
    });

    // Convert ChainServiceConfig to ChainConfig that the chain service canister expects
    let chain_config = ChainConfig {
        chain_id: config.chain_id,
        chain_name: config.chain_name.clone(),
        rpc_url: config.rpc_url.clone(),
        block_interval_seconds: config.block_interval_seconds,
        max_response_bytes: config.max_response_bytes,
        proxy_canister_id: default_proxy,
        evm_rpc_canister_id: default_evm_rpc,
        rpc_service: match config.chain_id {
            1 => RpcServiceConfig::EthMainnet { providers: None },
            11155111 => RpcServiceConfig::EthSepolia { providers: None },
            42161 => RpcServiceConfig::ArbitrumOne { providers: None },
            8453 => RpcServiceConfig::BaseMainnet { providers: None },
            10 => RpcServiceConfig::OptimismMainnet { providers: None },
            _ => RpcServiceConfig::Custom { 
                rpc_url: config.rpc_url.clone(),
                chain_id: config.chain_id as u64,
            },
        },
    };
    
    // Install code
    let init_args = candid::encode_args((chain_config,))
        .map_err(|e| format!("Failed to encode init args: {:?}", e))?;
    
    let install_args = InstallCodeArgument {
        mode: CanisterInstallMode::Install,
        canister_id,
        wasm_module: wasm,
        arg: init_args,
    };
    
    install_code(install_args)
        .await
        .map_err(|e| format!("Failed to install code: {:?}", e))?;
    
    // Set orchestrator in the chain service
    let _: () = ic_cdk::call(
        canister_id,
        "set_orchestrator",
        (ic_cdk::id(),)
    ).await.map_err(|e| format!("Failed to set orchestrator: {:?}", e))?;

    // Start monitoring by default after deployment
    let _: () = ic_cdk::call(
        canister_id,
        "start_monitoring",
        ()
    ).await.map_err(|e| format!("Failed to start monitoring: {:?}", e))?;
    
    // Update state
    mutate_state(|state| {
        let chain_service_info = ChainServiceInfo {
            canister_id,
            chain_id,
            chain_name,
            version: state.version.clone(),
            status: ChainServiceStatus::Active,
            deployment_timestamp: ic_cdk::api::time(),
            last_upgrade_timestamp: None,
        };
        
        state.chain_services.insert(chain_id, chain_service_info);
    });
    
    Ok(canister_id)
}

pub async fn upgrade_chain_service(chain_id: u32) -> Result<(), String> {
    let (canister_id, current_version) = read_state(|state| {
        let info = state.chain_services.get(&chain_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))?;
        Ok::<(Principal, String), String>((info.canister_id, info.version.clone()))
    })?;
    
    let wasm = read_state(|state| {
        state.chain_service_wasm.clone()
            .ok_or_else(|| "No ChainService WASM module available".to_string())
    })?;
    
    // Stop monitoring
    let _: () = ic_cdk::call(
        canister_id,
        "stop_monitoring",
        ()
    ).await.map_err(|e| format!("Failed to stop monitoring: {:?}", e))?;
    
    // Upgrade canister
    let install_args = InstallCodeArgument {
        mode: CanisterInstallMode::Upgrade(None),
        canister_id,
        wasm_module: wasm,
        arg: vec![], // No init args for upgrade
    };
    
    install_code(install_args)
        .await
        .map_err(|e| format!("Failed to upgrade canister: {:?}", e))?;
    
    // Start monitoring again
    let _: () = ic_cdk::call(
        canister_id,
        "start_monitoring",
        ()
    ).await.map_err(|e| format!("Failed to start monitoring: {:?}", e))?;
    
    // Update state
    mutate_state(|state| {
        if let Some(info) = state.chain_services.get_mut(&chain_id) {
            info.version = state.version.clone();
            info.last_upgrade_timestamp = Some(ic_cdk::api::time());
            info.status = ChainServiceStatus::Active;
        }
    });
    
    Ok(())
}

pub async fn upgrade_all_chain_services() -> Result<Vec<UpgradeResult>, String> {
    let chain_ids: Vec<u32> = read_state(|state| {
        state.chain_services.keys().cloned().collect()
    });
    
    let mut results = Vec::new();
    
    for chain_id in chain_ids {
        let (canister_id, from_version) = read_state(|state| {
            let info = state.chain_services.get(&chain_id).unwrap();
            (info.canister_id, info.version.clone())
        });
        
        let to_version = read_state(|state| state.version.clone());
        
        let result = match upgrade_chain_service(chain_id).await {
            Ok(()) => UpgradeResult {
                chain_id,
                canister_id,
                success: true,
                error_message: None,
                timestamp: ic_cdk::api::time(),
                from_version,
                to_version,
            },
            Err(error) => UpgradeResult {
                chain_id,
                canister_id,
                success: false,
                error_message: Some(error),
                timestamp: ic_cdk::api::time(),
                from_version,
                to_version,
            },
        };
        
        results.push(result);
    }
    
    Ok(results)
}

pub fn update_chain_service_wasm(wasm: Vec<u8>, version: String) -> Result<(), String> {
    mutate_state(|state| {
        // Store current WASM as previous if it exists
        if let Some(current_wasm) = &state.chain_service_wasm {
            state.previous_wasms.push_back((state.version.clone(), current_wasm.clone()));
            
            // Limit the number of stored previous WASMs
            if state.previous_wasms.len() > MAX_PREVIOUS_WASMS {
                state.previous_wasms.pop_front();
            }
        }
        
        // Update to new WASM
        state.chain_service_wasm = Some(wasm);
        state.version = version;
        
        Ok(())
    })
}

pub async fn pause_chain_service(chain_id: u32) -> Result<(), String> {
    let canister_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;
    
    // Stop monitoring
    let _: () = ic_cdk::call(
        canister_id,
        "stop_monitoring",
        ()
    ).await.map_err(|e| format!("Failed to stop monitoring: {:?}", e))?;
    
    // Update status
    mutate_state(|state| {
        if let Some(info) = state.chain_services.get_mut(&chain_id) {
            info.status = ChainServiceStatus::Paused;
        }
    });
    
    Ok(())
}

pub async fn resume_chain_service(chain_id: u32) -> Result<(), String> {
    let canister_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;
    
    // Start monitoring
    let _: () = ic_cdk::call(
        canister_id,
        "start_monitoring",
        ()
    ).await.map_err(|e| format!("Failed to start monitoring: {:?}", e))?;
    
    // Update status
    mutate_state(|state| {
        if let Some(info) = state.chain_services.get_mut(&chain_id) {
            info.status = ChainServiceStatus::Active;
        }
    });
    
    Ok(())
}

pub async fn configure_chain_service(
    chain_id: u32,
    proxy_canister_id: Option<Principal>,
    evm_rpc_canister_id: Option<Principal>,
) -> Result<(), String> {
    // Resolve chain service canister ID
    let canister_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;

    // Update proxy canister ID if provided (or clear if None)
    let _: () = ic_cdk::call(
        canister_id,
        "update_proxy_canister_id",
        (proxy_canister_id,)
    ).await.map_err(|e| format!("Failed to update proxy canister ID: {:?}", e))?;

    // Update EVM RPC canister ID if provided (or clear if None)
    let _: () = ic_cdk::call(
        canister_id,
        "update_evm_rpc_canister_id",
        (evm_rpc_canister_id,)
    ).await.map_err(|e| format!("Failed to update EVM RPC canister ID: {:?}", e))?;

    Ok(())
}

pub async fn rollback_chain_service(chain_id: u32, version: String) -> Result<(), String> {
    let canister_id = read_state(|state| {
        state.chain_services.get(&chain_id)
            .map(|info| info.canister_id)
            .ok_or_else(|| format!("Chain service for chain ID {} not found", chain_id))
    })?;
    
    // Find the requested version
    let wasm = read_state(|state| {
        state.previous_wasms.iter()
            .find(|(ver, _)| ver == &version)
            .map(|(_, wasm)| wasm.clone())
            .ok_or_else(|| format!("Version {} not found", version))
    })?;
    
    // Stop monitoring
    let _: () = ic_cdk::call(
        canister_id,
        "stop_monitoring",
        ()
    ).await.map_err(|e| format!("Failed to stop monitoring: {:?}", e))?;
    
    // Upgrade to the previous version
    let install_args = InstallCodeArgument {
        mode: CanisterInstallMode::Upgrade(None),
        canister_id,
        wasm_module: wasm,
        arg: vec![],
    };
    
    install_code(install_args)
        .await
        .map_err(|e| format!("Failed to rollback canister: {:?}", e))?;
    
    // Start monitoring again
    let _: () = ic_cdk::call(
        canister_id,
        "start_monitoring",
        ()
    ).await.map_err(|e| format!("Failed to start monitoring: {:?}", e))?;
    
    // Update version information
    mutate_state(|state| {
        if let Some(info) = state.chain_services.get_mut(&chain_id) {
            info.version = version;
            info.last_upgrade_timestamp = Some(ic_cdk::api::time());
            info.status = ChainServiceStatus::Active;
        }
    });
    
    Ok(())
}
