use candid::{Nat, Principal};
use pocket_ic::{PocketIc, WasmResult};
use serde_bytes::ByteBuf;
use std::collections::HashMap;

mod types;
mod utils;
use types::*;
use utils::*;


#[test]
fn test_orchestrator_deployment() {
    let (pic, orchestrator_id) = setup_orchestrator_only();
    
    // Test that orchestrator is deployed and responding
    let result = pic.query_call(
        orchestrator_id,
        Principal::anonymous(),
        "get_available_chain_services",
        candid::encode_one(()).unwrap(),
    );
    
    assert!(result.is_ok(), "Failed to call get_available_chain_services: {:?}", result.err());
    
    let response = result.unwrap();
    let services: Vec<ChainServiceInfo> = match response {
        pocket_ic::WasmResult::Reply(bytes) => candid::decode_one(&bytes).expect("Failed to decode response"),
        pocket_ic::WasmResult::Reject(msg) => return panic!("Call was rejected: {}", msg),
    };
    assert_eq!(services.len(), 0, "Should have no chain services deployed initially");
}

#[test]
fn test_update_chain_service_wasm() {
    let (pic, orchestrator_id) = setup_orchestrator_only();
    
    // Update the chain service WASM
    let chain_service_wasm = get_chain_service_wasm();
    let result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "update_chain_service_wasm",
        candid::encode_args((chain_service_wasm, "1.0.0".to_string())).unwrap(),
    );
    
    assert!(result.is_ok(), "Failed to call update_chain_service_wasm: {:?}", result.err());
    
    let response = result.unwrap();
    let bytes = extract_reply_bytes(response);
    let update_result: Result<(), String> = 
        candid::decode_one(&bytes).expect("Failed to decode response");
    
    assert!(update_result.is_ok(), "Update chain service WASM failed: {:?}", update_result.err());
}

#[test]
fn test_deploy_chain_service() {
    let (pic, orchestrator_id) = setup_orchestrator_only();
    
    // First, update the chain service WASM
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
    
    // Deploy Ethereum chain service
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
    
    assert!(deploy_result.is_ok(), "Failed to call deploy_chain_service: {:?}", deploy_result.err());
    
    let response = deploy_result.unwrap();
    let deploy_result: Result<Principal, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(deploy_result.is_ok(), "Deploy chain service failed: {:?}", deploy_result.err());
    let chain_service_id = deploy_result.unwrap();
    
    // Verify chain service is listed
    let services_result = pic.query_call(
        orchestrator_id,
        Principal::anonymous(),
        "get_available_chain_services",
        candid::encode_one(()).unwrap(),
    );
    
    assert!(services_result.is_ok());
    let response = services_result.unwrap();
    let services: Vec<ChainServiceInfo> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    assert_eq!(services.len(), 1, "Should have one chain service deployed");
    assert_eq!(services[0].chain_id, 1);
    assert_eq!(services[0].chain_name, "Ethereum");
    assert_eq!(services[0].canister_id, chain_service_id);
    assert!(matches!(services[0].status, ChainServiceStatus::Active));
}

#[test]
fn test_user_registration() {
    let (pic, orchestrator_id) = setup_orchestrator_only();
    
    // Register a user using the constant to avoid CheckSequenceNotMatch errors
    let user_principal = Principal::from_text(USER_PRINCIPAL).unwrap();
    let result = pic.update_call(
        orchestrator_id,
        user_principal,
        "register_user",
        candid::encode_one(()).unwrap(),
    );
    
    assert!(result.is_ok(), "Failed to call register_user: {:?}", result.err());
    
    let response = result.unwrap();
    let register_result: Result<(), String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(register_result.is_ok(), "User registration failed: {:?}", register_result.err());
}

#[test]
fn test_service_discovery() {
    let (pic, orchestrator_id) = setup_orchestrator_only();
    
    // Deploy a chain service first
    let chain_service_wasm = get_chain_service_wasm();
    let _update_result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "update_chain_service_wasm",
        candid::encode_args((chain_service_wasm, "1.0.0".to_string())).unwrap(),
    );
    
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
    
    let response = deploy_result.unwrap();
    let deploy_result: Result<Principal, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    let expected_chain_service_id = deploy_result.unwrap();
    
    // Test service discovery
    let discovery_result = pic.query_call(
        orchestrator_id,
        Principal::anonymous(),
        "get_chain_service_canister_id",
        candid::encode_one(1u32).unwrap(),
    );
    
    assert!(discovery_result.is_ok(), "Failed to call get_chain_service_canister_id: {:?}", discovery_result.err());
    
    let response = discovery_result.unwrap();
    let discovery_result: Result<Principal, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(discovery_result.is_ok(), "Service discovery failed: {:?}", discovery_result.err());
    assert_eq!(discovery_result.unwrap(), expected_chain_service_id);
}

#[test]
fn test_metrics_endpoint() {
    let (pic, orchestrator_id) = setup_orchestrator_only();
    
    // Test HTTP metrics endpoint
    let http_request = HttpRequest {
        method: "GET".to_string(),
        url: "/metrics".to_string(),
        headers: vec![],
        body: ByteBuf::new(),
    };
    
    let result = pic.query_call(
        orchestrator_id,
        Principal::anonymous(),
        "http_request",
        candid::encode_one(http_request).unwrap(),
    );
    
    assert!(result.is_ok(), "Failed to call http_request: {:?}", result.err());
    
    let response = result.unwrap();
    let http_response: HttpResponse = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert_eq!(http_response.status_code, 200);
    assert!(!http_response.body.is_empty());
    
    // Check that response contains expected metrics
    let body_str = String::from_utf8(http_response.body.to_vec()).unwrap();
    assert!(body_str.contains("orchestrator_total_chain_services"));
    assert!(body_str.contains("cycle_balance"));
}

#[test]
fn test_pause_resume_chain_service() {
    let (pic, orchestrator_id) = setup_orchestrator_only();
    
    // Deploy a chain service first
    let chain_service_wasm = get_chain_service_wasm();
    let _update_result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "update_chain_service_wasm",
        candid::encode_args((chain_service_wasm, "1.0.0".to_string())).unwrap(),
    );
    
    let config = ChainServiceConfig {
        chain_id: 1,
        chain_name: "Ethereum".to_string(),
        rpc_url: "https://eth-mainnet.alchemyapi.io/v2/demo".to_string(),
        block_interval_seconds: 12,
        max_response_bytes: 2_000_000,
    };
    
    let _deploy_result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "deploy_chain_service",
        candid::encode_args((1u32, "Ethereum".to_string(), config)).unwrap(),
    );
    
    // Pause the chain service
    let pause_result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "pause_chain_service",
        candid::encode_one(1u32).unwrap(),
    );
    
    assert!(pause_result.is_ok(), "Failed to call pause_chain_service: {:?}", pause_result.err());
    
    let response = pause_result.unwrap();
    let pause_result: Result<(), String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(pause_result.is_ok(), "Pause chain service failed: {:?}", pause_result.err());
    
    // Verify service is paused
    let services_result = pic.query_call(
        orchestrator_id,
        Principal::anonymous(),
        "get_available_chain_services",
        candid::encode_one(()).unwrap(),
    );
    
    let response = services_result.unwrap();
    let services: Vec<ChainServiceInfo> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    assert!(matches!(services[0].status, ChainServiceStatus::Paused));
    
    // Resume the chain service
    let resume_result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "resume_chain_service",
        candid::encode_one(1u32).unwrap(),
    );
    
    assert!(resume_result.is_ok(), "Failed to call resume_chain_service: {:?}", resume_result.err());
    
    let response = resume_result.unwrap();
    let resume_result: Result<(), String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(resume_result.is_ok(), "Resume chain service failed: {:?}", resume_result.err());
    
    // Verify service is active again
    let services_result = pic.query_call(
        orchestrator_id,
        Principal::anonymous(),
        "get_available_chain_services",
        candid::encode_one(()).unwrap(),
    );
    
    let response = services_result.unwrap();
    let services: Vec<ChainServiceInfo> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    assert!(matches!(services[0].status, ChainServiceStatus::Active));
}

// Test basic functionality without requiring WASM files
#[test]
fn test_basic_types_and_serialization() {
    // Test that our types compile correctly
    let admin = Principal::anonymous();
    let version = "1.0.0".to_string();
    
    let init_arg = OrchestratorInitArg {
        admin,
        version,
    };
    
    // Test serialization
    let encoded = candid::encode_one(&init_arg).expect("Failed to encode init arg");
    let decoded: OrchestratorInitArg = candid::decode_one(&encoded).expect("Failed to decode init arg");
    
    assert_eq!(decoded.admin, admin);
    assert_eq!(decoded.version, "1.0.0");
    
    // Test chain service config
    let config = ChainServiceConfig {
        chain_id: 1,
        chain_name: "Ethereum".to_string(),
        rpc_url: "https://eth-mainnet.alchemyapi.io/v2/demo".to_string(),
        block_interval_seconds: 12,
        max_response_bytes: 2_000_000,
    };
    
    let encoded = candid::encode_one(&config).expect("Failed to encode config");
    let decoded: ChainServiceConfig = candid::decode_one(&encoded).expect("Failed to decode config");
    
    assert_eq!(decoded.chain_id, 1);
    assert_eq!(decoded.chain_name, "Ethereum");
}
