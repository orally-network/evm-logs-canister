use candid::{Principal};
use pocket_ic::{PocketIc, WasmResult};

fn get_evm_rpc_wasm() -> Vec<u8> {
    std::fs::read("assets/evm_rpc.wasm.gz")
        .expect("Failed to read EVM RPC WASM file from assets/evm_rpc.wasm.gz")
}

pub fn setup_evm_rpc_canister(pic: &PocketIc) -> Principal {
    // Use the mainnet EVM RPC canister ID as suggested
    let _evm_rpc_id = Principal::from_text("7hfb6-caaaa-aaaar-qadga-cai").unwrap();

    // Create canister (PocketIC doesn't support specific IDs, so we create a new one)
    let canister_id = pic.create_canister();

    pic.add_cycles(canister_id, 10_000_000_000_000); // 10T cycles for EVM RPC

    // Install EVM RPC canister with proper InstallArgs
    let evm_rpc_wasm = get_evm_rpc_wasm();

    // Create InstallArgs for EVM RPC canister
    #[derive(candid::CandidType, serde::Deserialize)]
    struct InstallArgs {
        demo: Option<bool>,
        manage_api_keys: Option<Vec<Principal>>,
        log_filter: Option<LogFilter>,
        override_provider: Option<OverrideProvider>,
        nodes_in_subnet: Option<u32>,
    }

    #[derive(candid::CandidType, serde::Deserialize)]
    enum LogFilter {
        ShowAll,
        HideAll,
        ShowPattern(String),
        HidePattern(String),
    }

    #[derive(candid::CandidType, serde::Deserialize)]
    struct OverrideProvider {
        override_url: Option<RegexSubstitution>,
    }

    #[derive(candid::CandidType, serde::Deserialize)]
    struct RegexSubstitution {
        pattern: String,
        replacement: String,
    }

    let install_args = InstallArgs {
        demo: Some(true), // Enable demo mode for testing
        manage_api_keys: None,
        log_filter: Some(LogFilter::ShowAll),
        override_provider: None,
        nodes_in_subnet: Some(1),
    };

    let init_arg = candid::encode_one(install_args).unwrap();

    pic.install_canister(canister_id, evm_rpc_wasm, init_arg, None);

    canister_id
}

// Helper function to extract bytes from WasmResult
pub fn extract_reply_bytes(result: WasmResult) -> Vec<u8> {
    match result {
        WasmResult::Reply(bytes) => bytes,
        WasmResult::Reject(msg) => panic!("Call was rejected: {}", msg),
    }
}

// WASM loading functions
pub fn get_orchestrator_wasm() -> Vec<u8> {
    std::fs::read("../target/wasm32-unknown-unknown/release/orchestrator_canister.wasm")
        .expect("Failed to read orchestrator WASM file. Run 'cargo build --target wasm32-unknown-unknown --release --package orchestrator_canister' first")
}

pub fn get_chain_service_wasm() -> Vec<u8> {
    std::fs::read("../target/wasm32-unknown-unknown/release/chain_service_canister.wasm")
        .expect("Failed to read chain service WASM file. Run 'cargo build --target wasm32-unknown-unknown --release --package chain_service_canister' first")
}

// Setup functions
pub fn setup_orchestrator_only() -> (PocketIc, Principal) {
    let pic = PocketIc::new();
    
    // Create orchestrator canister
    let orchestrator_id = pic.create_canister();
    pic.add_cycles(orchestrator_id, 2_000_000_000_000); // 2T cycles
    
    // Install orchestrator
    let orchestrator_wasm = get_orchestrator_wasm();
    let init_arg = candid::encode_one(super::types::OrchestratorInitArg {
        admin: Principal::anonymous(),
        version: "1.0.0".to_string(),
    }).unwrap();
    
    pic.install_canister(orchestrator_id, orchestrator_wasm, init_arg, None);
    
    (pic, orchestrator_id)
}

pub fn setup_chain_service_only() -> (PocketIc, Principal) {
    use super::types::*;
    
    let pic = PocketIc::new();
    
    // Set up EVM RPC canister first
    let evm_rpc_id = Principal::from_text(EVM_RPC_PRINCIPAL).unwrap();
    let orchestrator_principal = Principal::from_text(ORCHESTRATOR_PRINCIPAL).unwrap();

    // Create chain service canister with orchestrator as controller
    let chain_service_id = pic.create_canister_with_id(Some(orchestrator_principal), None, Principal::from_text(CHAIN_SERVICE_CANISTER_ID).unwrap())
        .unwrap_or_else(|_| {
            // If we can't create with specific ID, create normally and set controller
            let canister_id = pic.create_canister();
            pic.set_controllers(canister_id, None, vec![orchestrator_principal]).unwrap();
            canister_id
        });
    
    pic.add_cycles(chain_service_id, 2_000_000_000_000); // 2T cycles
    
    // Install chain service with orchestrator as the deployer (this sets orchestrator in init)
    let chain_service_wasm = get_chain_service_wasm();
    let init_arg = candid::encode_one(ChainConfig {
        chain_id: 1,
        chain_name: "Ethereum".to_string(),
        rpc_url: "https://eth-mainnet.alchemyapi.io/v2/demo".to_string(),
        block_interval_seconds: 12,
        max_response_bytes: 2_000_000,
        proxy_canister_id: None,
        evm_rpc_canister_id: Some(evm_rpc_id),
        rpc_service: RpcServiceConfig::EthMainnet { providers: None },
    }).unwrap();
    
    // Install with orchestrator as the caller - this will set orchestrator during init
    pic.install_canister(chain_service_id, chain_service_wasm, init_arg, Some(orchestrator_principal));
    
    (pic, chain_service_id)
}

pub fn setup_full_system() -> (PocketIc, Principal, Principal) {
    use super::types::*;
    
    let (pic, orchestrator_id) = setup_orchestrator_only();
    
    // Update the chain service WASM in orchestrator
    let chain_service_wasm = get_chain_service_wasm();
    let update_result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "update_chain_service_wasm",
        candid::encode_args((chain_service_wasm, "1.0.0".to_string())).unwrap(),
    );
    
    assert!(update_result.is_ok());
    let response = update_result.unwrap();
    let update_result: Result<(), String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    assert!(update_result.is_ok());
    
    // Deploy Ethereum chain service through orchestrator
    let config = ChainServiceConfig {
        chain_id: 1,
        chain_name: "Ethereum".to_string(),
        rpc_url: "https://eth-mainnet.alchemyapi.io/v2/demo".to_string(),
        block_interval_seconds: 12,
        max_response_bytes: 2_000_000,
    };
    
    let deploy_result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "deploy_chain_service",
        candid::encode_args((1u32, "Ethereum".to_string(), config)).unwrap(),
    );
    
    assert!(deploy_result.is_ok());
    let response = deploy_result.unwrap();
    let deploy_result: Result<Principal, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    let chain_service_id = deploy_result.expect("Failed to deploy chain service");
    
    (pic, orchestrator_id, chain_service_id)
}
