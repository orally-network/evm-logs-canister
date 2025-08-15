use candid::{Nat, Principal};
use pocket_ic::{PocketIc, WasmResult};
use serde_bytes::ByteBuf;
use evm_logs_types::{SubscriptionRegistration, Filter, Hex20, Hex32};
use std::str::FromStr;

mod types;
mod utils;
use types::*;
use utils::*;


#[test]
fn test_chain_service_deployment() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    // Test that chain service is deployed and responding
    let result = pic.query_call(
        chain_service_id,
        Principal::anonymous(),
        "get_monitoring_status",
        candid::encode_one(()).unwrap(),
    );
    
    assert!(result.is_ok(), "Failed to call get_monitoring_status: {:?}", result.err());
    
    let response = result.unwrap();
    let monitoring_status: MonitoringStatus = match response {
        WasmResult::Reply(bytes) => candid::decode_one(&bytes).expect("Failed to decode response"),
        WasmResult::Reject(msg) => panic!("Call was rejected: {}", msg),
    };
    
    assert_eq!(monitoring_status.monitoring_interval_seconds, 12);
    assert_eq!(monitoring_status.last_processed_block, Nat::from(0u32));
}

#[test]
fn test_balance_management() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    let user_principal = Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap();
    
    // Check initial balance (should be 0)
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "get_balance",
        candid::encode_one(user_principal).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let balance: Nat = candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    assert_eq!(balance, Nat::from(0u32));
    
    // Top up balance (simulate sending cycles)
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "top_up_balance",
        candid::encode_one(user_principal).unwrap(),
    );
    
    // Note: In a real test with cycles, this would work, but PocketIC doesn't simulate cycles transfer
    // So we expect this to fail with "No cycles sent with the call"
    assert!(result.is_ok());
    let response = result.unwrap();
    let top_up_result: Result<TopUpBalanceResult, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    // Should fail because no cycles were sent
    assert!(top_up_result.is_err());
    assert!(top_up_result.unwrap_err().contains("No cycles sent with the call"));
}

#[test]
fn test_subscription_lifecycle() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    let user_principal = Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap();
    
    // Create a subscription registration
    let registration = SubscriptionRegistration {
        chain_id: 1,
        filter: Filter {
            address: Some(Hex20::from_str("0x1234567890123456789012345678901234567890").unwrap()),
            topics: None,
        },
        memo: None,
        canister_to_top_up: user_principal,
    };
    
    // Register subscription
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "register_subscription",
        candid::encode_one(registration).unwrap(),
    );
    
    assert!(result.is_ok(), "Failed to call register_subscription: {:?}", result.err());
    
    let response = result.unwrap();
    let register_result: Result<RegisterSubscriptionResult, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(register_result.is_ok(), "Register subscription failed: {:?}", register_result.err());
    let subscription_result = register_result.unwrap();
    let subscription_id = subscription_result.subscription_id;
    
    // Verify subscription was created
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "get_user_subscriptions",
        candid::encode_one(user_principal).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let subscriptions: Vec<SubscriptionInfo> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert_eq!(subscriptions.len(), 1);
    assert_eq!(subscriptions[0].subscription_id, subscription_id);
    assert_eq!(subscriptions[0].subscriber_principal, user_principal);
    assert_eq!(subscriptions[0].chain_id, 1);
    assert!(matches!(subscriptions[0].status, SubscriptionStatus::Active));
    
    // Test subscription status query
    let result = pic.query_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "get_subscription_status",
        candid::encode_one(subscription_id.clone()).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let status_result: Result<SubscriptionStatus, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(status_result.is_ok());
    assert!(matches!(status_result.unwrap(), SubscriptionStatus::Active));
    
    // Pause subscription
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "pause_subscription",
        candid::encode_args((subscription_id.clone(), user_principal)).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let pause_result: Result<(), String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(pause_result.is_ok(), "Pause subscription failed: {:?}", pause_result.err());
    
    // Verify subscription is paused
    let result = pic.query_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "get_subscription_status",
        candid::encode_one(subscription_id.clone()).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let status_result: Result<SubscriptionStatus, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(status_result.is_ok());
    assert!(matches!(status_result.unwrap(), SubscriptionStatus::PausedByUser { .. }));
    
    // Resume subscription
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "resume_subscription",
        candid::encode_args((subscription_id.clone(), user_principal)).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let resume_result: Result<(), String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(resume_result.is_ok(), "Resume subscription failed: {:?}", resume_result.err());
    
    // Verify subscription is active again
    let result = pic.query_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "get_subscription_status",
        candid::encode_one(subscription_id.clone()).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let status_result: Result<SubscriptionStatus, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(status_result.is_ok());
    assert!(matches!(status_result.unwrap(), SubscriptionStatus::Active));
    
    // Unsubscribe
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "unsubscribe",
        candid::encode_args((subscription_id.clone(), user_principal)).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let unsubscribe_result: Result<UnsubscribeResult, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(unsubscribe_result.is_ok(), "Unsubscribe failed: {:?}", unsubscribe_result.err());
    
    // Verify subscription was removed
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "get_user_subscriptions",
        candid::encode_one(user_principal).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let subscriptions: Vec<SubscriptionInfo> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert_eq!(subscriptions.len(), 0);
}

#[test]
fn test_batch_subscription_operations() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    let user_principal = Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap();
    
    // Create multiple subscription registrations
    let registrations = vec![
        SubscriptionRegistration {
            chain_id: 1,
            filter: Filter {
                address: Some(Hex20::from_str("0x1111111111111111111111111111111111111111").unwrap()),
                topics: None,
            },
            memo: None,
            canister_to_top_up: user_principal,
        },
        SubscriptionRegistration {
            chain_id: 1,
            filter: Filter {
                address: Some(Hex20::from_str("0x2222222222222222222222222222222222222222").unwrap()),
                topics: None,
            },
            memo: None,
            canister_to_top_up: user_principal,
        },
    ];
    
    // Batch register subscriptions
    let result = pic.update_call(
        chain_service_id,
        user_principal,
        "batch_register_subscriptions",
        candid::encode_one(registrations).unwrap(),
    );
    
    assert!(result.is_ok(), "Failed to call batch_register_subscriptions: {:?}", result.err());
    
    let response = result.unwrap();
    let batch_result: Result<Vec<RegisterSubscriptionResult>, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(batch_result.is_ok(), "Batch register subscriptions failed: {:?}", batch_result.err());
    let results = batch_result.unwrap();
    assert_eq!(results.len(), 2);
    
    let subscription_ids: Vec<Nat> = results.into_iter().map(|r| r.subscription_id).collect();
    
    // Verify both subscriptions were created
    let result = pic.query_call(
        chain_service_id,
        Principal::anonymous(),
        "get_user_subscriptions",
        candid::encode_one(user_principal).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let subscriptions: Vec<SubscriptionInfo> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert_eq!(subscriptions.len(), 2);
    
    // Batch unsubscribe
    let result = pic.update_call(
        chain_service_id,
        user_principal,
        "batch_unsubscribe",
        candid::encode_one(subscription_ids).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let batch_unsubscribe_result: Result<Vec<UnsubscribeResult>, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(batch_unsubscribe_result.is_ok(), "Batch unsubscribe failed: {:?}", batch_unsubscribe_result.err());
    let unsubscribe_results = batch_unsubscribe_result.unwrap();
    assert_eq!(unsubscribe_results.len(), 2);
    
    // Verify all subscriptions were removed
    let result = pic.query_call(
        chain_service_id,
        Principal::anonymous(),
        "get_user_subscriptions",
        candid::encode_one(user_principal).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let subscriptions: Vec<SubscriptionInfo> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert_eq!(subscriptions.len(), 0);
}

#[test]
fn test_access_control() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    let user1 = Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap();
    let user2 = Principal::from_text("mxzaz-hqaaa-aaaar-qaada-cai").unwrap();
    
    // User1 creates a subscription
    let registration = SubscriptionRegistration {
        chain_id: 1,
        filter: Filter {
            address: Some(Hex20::from_str("0x1234567890123456789012345678901234567890").unwrap()),
            topics: None,
        },
        memo: None,
        canister_to_top_up: user1,
    };
    
    let result = pic.update_call(
        chain_service_id,
        user1,
        "register_subscription",
        candid::encode_one(registration).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let register_result: Result<RegisterSubscriptionResult, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    let subscription_id = register_result.unwrap().subscription_id;
    
    // User2 tries to pause User1's subscription (should fail)
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "pause_subscription",
        candid::encode_args((subscription_id.clone(), user2)).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let pause_result: Result<(), String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(pause_result.is_err());
    assert!(pause_result.unwrap_err().contains("Access denied"));
    
    // User2 tries to unsubscribe User1's subscription (should fail)
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "unsubscribe",
        candid::encode_args((subscription_id.clone(), user2)).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let unsubscribe_result: Result<UnsubscribeResult, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(unsubscribe_result.is_err());
    assert!(unsubscribe_result.unwrap_err().contains("Access denied"));
    
    // User1 can still manage their own subscription
    let result = pic.update_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "pause_subscription",
        candid::encode_args((subscription_id.clone(), user1)).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let pause_result: Result<(), String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(pause_result.is_ok());
}

#[test]
fn test_anonymous_user_restrictions() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    // Anonymous caller should be rejected by orchestrator guard
    let result = pic.update_call(
        chain_service_id,
        Principal::anonymous(),
        "top_up_balance",
        candid::encode_one(Principal::anonymous()).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let top_up_result: Result<TopUpBalanceResult, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(top_up_result.is_err());
    assert!(top_up_result.unwrap_err().contains("Access denied: caller is not orchestrator"));
    
    // Anonymous caller tries to register subscription -> should be rejected by orchestrator guard
    let registration = SubscriptionRegistration {
        chain_id: 1,
        filter: Filter {
            address: Some(Hex20::from_str("0x1234567890123456789012345678901234567890").unwrap()),
            topics: None,
        },
        memo: None,
        canister_to_top_up: Principal::anonymous(),
    };
    
    let result = pic.update_call(
        chain_service_id,
        Principal::anonymous(),
        "register_subscription",
        candid::encode_one(registration).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let register_result: Result<RegisterSubscriptionResult, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(register_result.is_err());
    assert!(register_result.unwrap_err().contains("Access denied: caller is not orchestrator"));
}

#[test]
fn test_chain_id_validation() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    let user_principal = Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap();
    
    // Try to register subscription with wrong chain ID
    let registration = SubscriptionRegistration {
        chain_id: 999, // Wrong chain ID (service is configured for chain 1)
        filter: Filter {
            address: Some(Hex20::from_str("0x1234567890123456789012345678901234567890").unwrap()),
            topics: None,
        },
        memo: None,
        canister_to_top_up: user_principal,
    };
    
    let result = pic.update_call(
        chain_service_id,
        user_principal,
        "register_subscription",
        candid::encode_one(registration).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let register_result: Result<RegisterSubscriptionResult, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(register_result.is_err());
    assert!(register_result.unwrap_err().contains("Chain ID mismatch"));
}

#[test]
fn test_cycle_usage_stats() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    // Get initial cycle usage stats
    let result = pic.query_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "get_cycle_usage_stats",
        candid::encode_one(()).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let stats: CycleUsageStats = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    // Initial stats should be zero
    assert_eq!(stats.total_cycles_used, 0);
    assert_eq!(stats.execution_count, 0);
    assert_eq!(stats.average_cycles_per_block, 0);
}

#[test]
fn test_health_status() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    // Get health status
    let result = pic.query_call(
        chain_service_id,
        Principal::from_text("mqygn-kiaaa-aaaar-qaadq-cai").unwrap(),
        "get_health_status",
        candid::encode_one(()).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let health: HealthStatus = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert_eq!(health.chain_id, 1);
    assert!(health.is_healthy);
    assert_eq!(health.consecutive_failures, 0);
}

#[test]
fn test_orchestrator_management() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    let orchestrator_principal = Principal::from_text(ORCHESTRATOR_PRINCIPAL).unwrap();
    let non_orchestrator_principal = Principal::from_text(USER_PRINCIPAL).unwrap();
    
    // The orchestrator was set during init, so it can now update config
    let new_config = ChainConfig {
        chain_id: 1,
        chain_name: "Ethereum Mainnet".to_string(),
        rpc_url: "https://mainnet.infura.io/v3/demo".to_string(),
        block_interval_seconds: 15,
        max_response_bytes: 3_000_000,
        proxy_canister_id: None,
        evm_rpc_canister_id: None,
        rpc_service: RpcServiceConfig::EthMainnet { providers: None },
    };
    
    let result = pic.update_call(
        chain_service_id,
        orchestrator_principal,
        "update_config",
        candid::encode_one(new_config.clone()).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let update_result: Result<(), String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert!(update_result.is_ok(), "Update config failed: {:?}", update_result.err());
    
    // Verify config was updated by checking monitoring status
    let result = pic.query_call(
        chain_service_id,
        Principal::anonymous(),
        "get_monitoring_status",
        candid::encode_one(()).unwrap(),
    );
    
    assert!(result.is_ok());
    let response = result.unwrap();
    let monitoring_status: MonitoringStatus = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    assert_eq!(monitoring_status.monitoring_interval_seconds, 15);
    
    // Test that non-orchestrator cannot update config
    let result = pic.update_call(
        chain_service_id,
        non_orchestrator_principal,
        "update_config",
        candid::encode_one(new_config).unwrap(),
    );
    
    // The call should succeed but return an error result
    match result {
        Ok(WasmResult::Reply(bytes)) => {
            let update_result: Result<(), String> = 
                candid::decode_one(&bytes).expect("Failed to decode response");
            
            assert!(update_result.is_err());
            assert!(update_result.unwrap_err().contains("Access denied: caller is not orchestrator"));
        }
        Ok(WasmResult::Reject(msg)) => {
            // If the guard rejects at the canister level, that's also acceptable
            assert!(msg.contains("Access denied: caller is not orchestrator"));
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

#[test]
fn test_metrics_endpoint() {
    let (pic, chain_service_id) = setup_chain_service_only();
    
    // Test HTTP metrics endpoint
    let http_request = HttpRequest {
        method: "GET".to_string(),
        url: "/metrics".to_string(),
        headers: vec![],
        body: ByteBuf::new(),
    };
    
    let result = pic.query_call(
        chain_service_id,
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
    assert!(body_str.contains("chain_service_subscriptions_total"));
    assert!(body_str.contains("cycle_balance"));
}

// Test basic functionality without requiring complex setup
#[test]
fn test_basic_types_and_serialization() {
    // Test that our types compile correctly
    let config = ChainConfig {
        chain_id: 1,
        chain_name: "Ethereum".to_string(),
        rpc_url: "https://eth-mainnet.alchemyapi.io/v2/demo".to_string(),
        block_interval_seconds: 12,
        max_response_bytes: 2_000_000,
        proxy_canister_id: None,
        evm_rpc_canister_id: None,
        rpc_service: RpcServiceConfig::EthMainnet { providers: None },
    };
    
    // Test serialization
    let encoded = candid::encode_one(&config).expect("Failed to encode config");
    let decoded: ChainConfig = candid::decode_one(&encoded).expect("Failed to decode config");
    
    assert_eq!(decoded.chain_id, 1);
    assert_eq!(decoded.chain_name, "Ethereum");
    assert_eq!(decoded.block_interval_seconds, 12);
    
    // Test filter serialization
    let filter = Filter {
        address: Some(Hex20::from_str("0x1234567890123456789012345678901234567890").unwrap()),
        topics: Some(vec![vec![Hex32::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").unwrap()]]),
    };
    
    let encoded = candid::encode_one(&filter).expect("Failed to encode filter");
    let decoded: Filter = candid::decode_one(&encoded).expect("Failed to decode filter");
    
    assert_eq!(decoded.address, Some(Hex20::from_str("0x1234567890123456789012345678901234567890").unwrap()));
    assert!(decoded.topics.is_some());
    assert_eq!(decoded.topics.unwrap().len(), 1);
}
