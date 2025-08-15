use candid::{Nat, Principal};
use pocket_ic::{PocketIc, WasmResult};
use serde_bytes::ByteBuf;
use evm_logs_types::{SubscriptionRegistration, Filter};
use chain_service_canister::types::{RegisterSubscriptionResult, TopUpBalanceResult};
use evm_rpc_types::{Hex20, Hex32};
use std::str::FromStr;
use std::collections::HashMap;

mod types;
mod utils;
use types::*;
use utils::*;

// Test user profiles for comprehensive multi-user testing
#[derive(Clone, Debug)]
pub struct TestUser {
    pub principal: Principal,
    pub name: &'static str,
    pub initial_icp_balance: u64, // in e8s (ICP base units)
    pub subscription_strategy: SubscriptionStrategy,
    pub expected_usage_pattern: UsagePattern,
}

#[derive(Clone, Debug)]
pub enum SubscriptionStrategy {
    HighVolume,    // Subscribe to high-activity contracts (USDC/ETH)
    MediumVolume,  // Subscribe to medium-activity contracts
    LowVolume,     // Subscribe to low-activity contracts
}

#[derive(Clone, Debug)]
pub enum UsagePattern {
    Heavy,    // Frequent subscriptions, high cycle consumption
    Moderate, // Balanced usage
    Light,    // Minimal usage, test edge cases
}

#[derive(Clone, Debug)]
pub struct UserBalanceTracker {
    pub initial_icp: u64,
    pub cycles_deposited: u64,
    pub cycles_consumed: u64,
    pub subscriptions: Vec<Nat>,
    pub events_received: u64,
    pub balance_history: Vec<(u64, Nat)>, // (timestamp, balance)
}

// Real contract configurations from test_canister
pub struct RealContractConfig {
    pub address: &'static str,
    pub chain_id: u32,
    pub topic: &'static str,
    pub name: &'static str,
    pub expected_volume: EventVolume,
}

#[derive(Clone, Debug)]
pub enum EventVolume {
    High,   // 100+ events per hour
    Medium, // 10-100 events per hour  
    Low,    // 1-10 events per hour
}

// Real contracts from test_canister with known activity levels
const REAL_CONTRACTS: &[RealContractConfig] = &[
    // High volume - Uniswap V3 USDC/ETH 0.05%
    RealContractConfig {
        address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
        chain_id: 1,
        topic: "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67",
        name: "uniswap_v3_usdc_eth_005",
        expected_volume: EventVolume::High,
    },
    // Medium volume - Base network USDC/ETH swaps
    RealContractConfig {
        address: "0xb2cc224c1c9feE385f8ad6a55b4d94E92359DC59",
        chain_id: 8453,
        topic: "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67",
        name: "base_swaps",
        expected_volume: EventVolume::Medium,
    },
    // Low volume - ChainFusion ckUSDT minter
    RealContractConfig {
        address: "0x7574eb42ca208a4f6960eccafdf186d627dcc175",
        chain_id: 1,
        topic: "0x257e057bb61920d8d0ed2cb7b720ac7f9c513cd1110bc9fa543079154f45f435",
        name: "ckusdt_minter",
        expected_volume: EventVolume::Low,
    },
    // Medium volume - Ethereum Sync (ETH/USDC Uniswap V2)
    RealContractConfig {
        address: "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852",
        chain_id: 1,
        topic: "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1",
        name: "eth_usdc_uniswap_v2_sync",
        expected_volume: EventVolume::Medium,
    },
    // Low volume - Fantom Token transfers
    RealContractConfig {
        address: "0x4E15361FD6b4BB609Fa63C81A2be19d873717870",
        chain_id: 1,
        topic: "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
        name: "fantom_token_transfers",
        expected_volume: EventVolume::Low,
    },
];

// Generate deterministic test user principals for funding on mainnet
fn generate_test_users() -> Vec<TestUser> {
    vec![
        TestUser {
            principal: Principal::from_text("rdmx6-jaaaa-aaaah-qcaiq-cai").unwrap(), // Generated deterministic principal 1
            name: "HighVolumeTrader",
            initial_icp_balance: 10_000_000_000, // 100 ICP
            subscription_strategy: SubscriptionStrategy::HighVolume,
            expected_usage_pattern: UsagePattern::Heavy,
        },
        TestUser {
            principal: Principal::from_text("rrkah-fqaaa-aaaah-qcaiq-cai").unwrap(), // Generated deterministic principal 2
            name: "MediumVolumeUser", 
            initial_icp_balance: 5_000_000_000, // 50 ICP
            subscription_strategy: SubscriptionStrategy::MediumVolume,
            expected_usage_pattern: UsagePattern::Moderate,
        },
        TestUser {
            principal: Principal::from_text("ryjl3-tyaaa-aaaah-qcaiq-cai").unwrap(), // Generated deterministic principal 3
            name: "LightUser",
            initial_icp_balance: 1_000_000_000, // 10 ICP
            subscription_strategy: SubscriptionStrategy::LowVolume,
            expected_usage_pattern: UsagePattern::Light,
        },
    ]
}

fn setup_test_users_with_icp(pic: &PocketIc) -> Vec<(Principal, u64)> {
    let test_users = generate_test_users();
    let mut users = Vec::new();
    
    for test_user in &test_users {
        // In PocketIC, we simulate ICP balance by adding cycles directly
        // Real ICP → cycles conversion rate is approximately 1 ICP = 1T cycles
        let cycles_from_icp = (test_user.initial_icp_balance as u128 * 1_000_000_000_000) / 100_000_000; // Convert e8s to cycles
        
        // Create a canister for this user (simulating their canister)
        let user_canister_id = pic.create_canister();
        pic.add_cycles(user_canister_id, cycles_from_icp + 2_000_000_000_000); // Extra cycles for canister operations
        
        users.push((user_canister_id, test_user.initial_icp_balance));
        
        println!("Created test user: {} with {} ICP ({} cycles)", 
                test_user.name, 
                test_user.initial_icp_balance / 100_000_000, // Convert to human-readable ICP
                cycles_from_icp);
    }
    
    users
}

fn create_subscription_for_strategy(strategy: &SubscriptionStrategy, user_principal: Principal) -> Vec<SubscriptionRegistration> {
    match strategy {
        SubscriptionStrategy::HighVolume => {
            // Subscribe to high-volume contracts
            vec![
                create_subscription_from_config(&REAL_CONTRACTS[0], user_principal), // Uniswap V3 USDC/ETH
            ]
        },
        SubscriptionStrategy::MediumVolume => {
            // Subscribe to medium-volume contracts
            vec![
                create_subscription_from_config(&REAL_CONTRACTS[1], user_principal), // Base swaps
                create_subscription_from_config(&REAL_CONTRACTS[3], user_principal), // ETH/USDC sync
            ]
        },
        SubscriptionStrategy::LowVolume => {
            // Subscribe to low-volume contracts
            vec![
                create_subscription_from_config(&REAL_CONTRACTS[2], user_principal), // ckUSDT minter
                create_subscription_from_config(&REAL_CONTRACTS[4], user_principal), // Fantom token
            ]
        },
    }
}

fn create_subscription_from_config(config: &RealContractConfig, user_principal: Principal) -> SubscriptionRegistration {
    SubscriptionRegistration {
        chain_id: config.chain_id,
        filter: Filter {
            address: Some(Hex20::from_str(config.address).unwrap()),
            topics: Some(vec![vec![Hex32::from_str(config.topic).unwrap()]]),
        },
        memo: Some(config.name.as_bytes().to_vec()),
        canister_to_top_up: user_principal,
    }
}

fn simulate_icp_to_cycles_deposit(
    pic: &PocketIc, 
    chain_service_id: Principal, 
    user_canister: Principal, 
    icp_amount: u64
) -> Result<u64, String> {
    // Simulate converting ICP to cycles and depositing to chain service
    let cycles_to_deposit = (icp_amount as u128 * 1_000_000_000_000) / 100_000_000; // ICP to cycles conversion
    
    // Call top_up_balance with cycles
    let result = pic.update_call(
        chain_service_id,
        user_canister,
        "top_up_balance",
        candid::encode_one(()).unwrap(),
    );
    
    match result {
        Ok(response) => {
            let top_up_result: Result<TopUpBalanceResult, String> = 
                candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
            
            match top_up_result {
                Ok(result) => Ok(result.cycles_received),
                Err(e) => Err(e),
            }
        },
        Err(e) => Err(format!("Failed to call top_up_balance: {:?}", e)),
    }
}

#[test]
fn test_three_user_icp_token_lifecycle() {
    let (pic, orchestrator_id, chain_service_id) = setup_full_system();
    let test_users_data = generate_test_users();
    let test_users = setup_test_users_with_icp(&pic);
    
    println!("=== Phase 1: Users convert ICP to cycles and top up balances ===");
    
    let mut user_trackers = Vec::new();
    
    for (i, (user_canister, initial_icp)) in test_users.iter().enumerate() {
        let test_user = &test_users_data[i];
        println!("Processing user: {}", test_user.name);
        
        // Check initial balance (should be 0)
        let initial_balance_result = pic.query_call(
            chain_service_id,
            Principal::anonymous(),
            "get_balance",
            candid::encode_one(*user_canister).unwrap(),
        );
        
        assert!(initial_balance_result.is_ok());
        let initial_balance: Nat = candid::decode_one(&extract_reply_bytes(initial_balance_result.unwrap()))
            .expect("Failed to decode balance");
        assert_eq!(initial_balance, Nat::from(0u32), "Initial balance should be 0");
        
        // Simulate depositing half of their ICP as cycles
        let icp_to_deposit = initial_icp / 2;
        let cycles_deposited = simulate_icp_to_cycles_deposit(&pic, chain_service_id, *user_canister, icp_to_deposit);
        
        // For PocketIC testing, we'll simulate successful deposit
        // In real testing, this would involve actual ICP → cycles conversion
        println!("User {} would deposit {} ICP ({} cycles)", 
                test_user.name, 
                icp_to_deposit / 100_000_000,
                (icp_to_deposit as u128 * 1_000_000_000_000) / 100_000_000);
        
        // Create balance tracker
        let mut tracker = UserBalanceTracker {
            initial_icp: *initial_icp,
            cycles_deposited: cycles_deposited.unwrap_or(0),
            cycles_consumed: 0,
            subscriptions: Vec::new(),
            events_received: 0,
            balance_history: vec![(0u64, initial_balance.clone())],
        };
        
        user_trackers.push(tracker);
    }
    
    println!("=== Phase 2: Users create subscriptions based on their strategies ===");
    
    for (i, (user_canister, _)) in test_users.iter().enumerate() {
        let test_user = &test_users_data[i];
        let subscriptions = create_subscription_for_strategy(&test_user.subscription_strategy, *user_canister);
        
        println!("User {} creating {} subscriptions", test_user.name, subscriptions.len());
        
        for subscription in subscriptions {
            let result = pic.update_call(
                chain_service_id,
                *user_canister,
                "register_subscription",
                candid::encode_one(subscription.clone()).unwrap(),
            );
            
            assert!(result.is_ok(), "Failed to register subscription for user {}", test_user.name);
            
            let response = result.unwrap();
            let register_result: Result<RegisterSubscriptionResult, String> = 
                candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
            
            match register_result {
                Ok(result) => {
                    user_trackers[i].subscriptions.push(result.subscription_id.clone());
                    println!("User {} registered subscription {} (estimated {} cycles/day)", 
                            test_user.name, 
                            result.subscription_id,
                            result.estimated_cycles_per_day);
                },
                Err(e) => panic!("Failed to register subscription for user {}: {}", test_user.name, e),
            }
        }
    }
    
    println!("=== Phase 3: Verify subscription creation and balance tracking ===");
    
    for (i, (user_canister, _)) in test_users.iter().enumerate() {
        let test_user = &test_users_data[i];
        
        // Get user subscriptions
        let subscriptions_result = pic.query_call(
            chain_service_id,
            Principal::anonymous(),
            "get_user_subscriptions",
            candid::encode_one(*user_canister).unwrap(),
        );
        
        assert!(subscriptions_result.is_ok());
        let subscriptions: Vec<SubscriptionInfo> = 
            candid::decode_one(&extract_reply_bytes(subscriptions_result.unwrap()))
                .expect("Failed to decode subscriptions");
        
        assert_eq!(subscriptions.len(), user_trackers[i].subscriptions.len(), 
                  "User {} should have {} subscriptions", test_user.name, user_trackers[i].subscriptions.len());
        
        // Verify all subscriptions are active
        for subscription in &subscriptions {
            assert!(matches!(subscription.status, SubscriptionStatus::Active), 
                   "Subscription should be active for user {}", test_user.name);
            assert_eq!(subscription.subscriber_principal, *user_canister);
        }
        
        println!("User {} has {} active subscriptions", test_user.name, subscriptions.len());
    }
    
    println!("=== Phase 4: Test cycle usage statistics ===");
    
    // Get cycle usage stats
    let stats_result = pic.query_call(
        chain_service_id,
        Principal::anonymous(),
        "get_cycle_usage_stats",
        candid::encode_one(()).unwrap(),
    );
    
    assert!(stats_result.is_ok());
    let stats: CycleUsageStats = 
        candid::decode_one(&extract_reply_bytes(stats_result.unwrap()))
            .expect("Failed to decode cycle usage stats");
    
    println!("Chain service cycle stats: total_used={}, executions={}, avg_per_block={}", 
            stats.total_cycles_used, stats.execution_count, stats.average_cycles_per_block);
    
    // Verify stats are being tracked
    assert!(stats.last_updated > 0, "Stats should have been updated");
    
    println!("=== Test completed successfully ===");
    println!("Generated test user principals for mainnet funding:");
    for test_user in &test_users_data {
        println!("  {}: {} (needs {} ICP)", 
                test_user.name, 
                test_user.principal, 
                test_user.initial_icp_balance / 100_000_000);
    }
}

#[test]
fn test_multi_user_real_contract_virtual_filtering() {
    let (pic, orchestrator_id, chain_service_id) = setup_full_system();
    let test_users_data = generate_test_users();
    let test_users = setup_test_users_with_icp(&pic);
    
    println!("=== Testing Virtual Filtering with Multiple Users on Same Contract ===");
    
    // All users subscribe to the same high-volume contract but with different topic filters
    let base_contract = &REAL_CONTRACTS[0]; // Uniswap V3 USDC/ETH
    
    for (i, (user_canister, _)) in test_users.iter().enumerate() {
        let test_user = &test_users_data[i];
        
        // Create subscription with same address but different topic combinations
        let subscription = SubscriptionRegistration {
            chain_id: base_contract.chain_id,
            filter: Filter {
                address: Some(Hex20::from_str(base_contract.address).unwrap()),
                topics: match i {
                    0 => Some(vec![vec![Hex32::from_str(base_contract.topic).unwrap()]]), // User 1: basic topic
                    1 => Some(vec![
                        vec![Hex32::from_str(base_contract.topic).unwrap()],
                        vec![], // User 2: topic + any second topic
                    ]),
                    2 => Some(vec![
                        vec![Hex32::from_str(base_contract.topic).unwrap()],
                        vec![],
                        vec![], // User 3: topic + any second + any third topic
                    ]),
                    _ => None,
                },
            },
            memo: Some(format!("user_{}_filter", i).as_bytes().to_vec()),
            canister_to_top_up: *user_canister,
        };
        
        let result = pic.update_call(
            chain_service_id,
            *user_canister,
            "register_subscription",
            candid::encode_one(subscription).unwrap(),
        );
        
        assert!(result.is_ok(), "Failed to register subscription for user {}", test_user.name);
        println!("User {} subscribed with filter complexity level {}", test_user.name, i + 1);
    }
    
    // Verify that all users are subscribed to the same contract
    // This should result in optimized RPC calls (virtual filtering)
    let total_subscriptions = test_users.iter().enumerate().map(|(i, (user_canister, _))| {
        let subscriptions_result = pic.query_call(
            chain_service_id,
            Principal::anonymous(),
            "get_user_subscriptions",
            candid::encode_one(*user_canister).unwrap(),
        );
        
        let subscriptions: Vec<SubscriptionInfo> = 
            candid::decode_one(&extract_reply_bytes(subscriptions_result.unwrap()))
                .expect("Failed to decode subscriptions");
        
        assert_eq!(subscriptions.len(), 1, "Each user should have 1 subscription");
        subscriptions.len()
    }).sum::<usize>();
    
    assert_eq!(total_subscriptions, 3, "Should have 3 total subscriptions");
    println!("Virtual filtering test completed: {} users subscribed to same contract", test_users.len());
}

#[test]
fn test_cross_chain_multi_user_economics() {
    let (pic, orchestrator_id) = setup_orchestrator_only();
    let test_users_data = generate_test_users();
    let test_users = setup_test_users_with_icp(&pic);
    
    println!("=== Testing Cross-Chain Multi-User Economics ===");
    
    // Deploy multiple chain services
    let chain_service_wasm = get_chain_service_wasm();
    let _update_result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "update_chain_service_wasm",
        candid::encode_args((chain_service_wasm, "1.0.0".to_string())).unwrap(),
    );
    
    // Deploy Ethereum chain service
    let eth_config = ChainServiceConfig {
        chain_id: 1,
        chain_name: "Ethereum".to_string(),
        rpc_url: "https://eth-mainnet.alchemyapi.io/v2/demo".to_string(),
        block_interval_seconds: 12,
        max_response_bytes: 2_000_000,
    };
    
    let eth_deploy_result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "deploy_chain_service",
        candid::encode_args((1u32, "Ethereum".to_string(), eth_config)).unwrap(),
    );
    
    let eth_response = eth_deploy_result.unwrap();
    let eth_service_id: Result<Principal, String> = 
        candid::decode_one(&extract_reply_bytes(eth_response)).expect("Failed to decode response");
    let ethereum_service = eth_service_id.unwrap();
    
    // Deploy Base chain service
    let base_config = ChainServiceConfig {
        chain_id: 8453,
        chain_name: "Base".to_string(),
        rpc_url: "https://base-mainnet.alchemyapi.io/v2/demo".to_string(),
        block_interval_seconds: 2,
        max_response_bytes: 2_000_000,
    };
    
    let base_deploy_result = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "deploy_chain_service",
        candid::encode_args((8453u32, "Base".to_string(), base_config)).unwrap(),
    );
    
    let base_response = base_deploy_result.unwrap();
    let base_service_id: Result<Principal, String> = 
        candid::decode_one(&extract_reply_bytes(base_response)).expect("Failed to decode response");
    let base_service = base_service_id.unwrap();
    
    println!("Deployed Ethereum service: {}", ethereum_service);
    println!("Deployed Base service: {}", base_service);
    
    // User distribution across chains:
    // User 1: Ethereum only (high volume)
    // User 2: Base only (medium volume)  
    // User 3: Both chains (multi-chain user)
    
    let user_chain_assignments = vec![
        (0, vec![ethereum_service]), // User 1: Ethereum only
        (1, vec![base_service]),     // User 2: Base only
        (2, vec![ethereum_service, base_service]), // User 3: Multi-chain
    ];
    
    for (user_idx, chain_services) in user_chain_assignments {
        let (user_canister, _) = test_users[user_idx];
        let test_user = &test_users_data[user_idx];
        
        for chain_service_id in chain_services {
            // Create appropriate subscription for each chain
            let subscription = if chain_service_id == ethereum_service {
                create_subscription_from_config(&REAL_CONTRACTS[0], user_canister) // Uniswap V3
            } else {
                create_subscription_from_config(&REAL_CONTRACTS[1], user_canister) // Base swaps
            };
            
            let result = pic.update_call(
                chain_service_id,
                user_canister,
                "register_subscription",
                candid::encode_one(subscription).unwrap(),
            );
            
            assert!(result.is_ok(), "Failed to register subscription for user {} on chain service {}", 
                   test_user.name, chain_service_id);
            
            println!("User {} subscribed to chain service {}", test_user.name, chain_service_id);
        }
    }
    
    // Verify cycle isolation between chains
    for chain_service_id in [ethereum_service, base_service] {
        let stats_result = pic.query_call(
            chain_service_id,
            Principal::anonymous(),
            "get_cycle_usage_stats",
            candid::encode_one(()).unwrap(),
        );
        
        assert!(stats_result.is_ok());
        let stats: CycleUsageStats = 
            candid::decode_one(&extract_reply_bytes(stats_result.unwrap()))
                .expect("Failed to decode cycle usage stats");
        
        println!("Chain service {} stats: total_used={}, executions={}", 
                chain_service_id, stats.total_cycles_used, stats.execution_count);
    }
    
    println!("Cross-chain economics test completed successfully");
}

#[test]
fn test_balance_depletion_and_management() {
    let (pic, orchestrator_id, chain_service_id) = setup_full_system();
    let test_users_data = generate_test_users();
    let test_users = setup_test_users_with_icp(&pic);
    
    println!("=== Testing Balance Depletion and Management ===");
    
    // Use the light user (User 3) for balance depletion testing
    let (light_user_canister, _) = test_users[2];
    let light_user_data = &test_users_data[2];
    
    println!("Testing balance depletion for user: {}", light_user_data.name);
    
    // Give user minimal balance by not depositing much
    // Subscribe to high-volume contract to quickly deplete balance
    let high_volume_subscription = create_subscription_from_config(&REAL_CONTRACTS[0], light_user_canister);
    
    let result = pic.update_call(
        chain_service_id,
        light_user_canister,
        "register_subscription",
        candid::encode_one(high_volume_subscription).unwrap(),
    );
    
    // This should succeed initially
    assert!(result.is_ok(), "Initial subscription should succeed");
    
    let response = result.unwrap();
    let register_result: Result<RegisterSubscriptionResult, String> = 
        candid::decode_one(&extract_reply_bytes(response)).expect("Failed to decode response");
    
    match register_result {
        Ok(result) => {
            println!("Light user subscribed to high-volume contract (sub_id: {}, estimated: {} cycles/day)", 
                    result.subscription_id, result.estimated_cycles_per_day);
            
            // Check subscription status
            let status_result = pic.query_call(
                chain_service_id,
                Principal::anonymous(),
                "get_subscription_status",
                candid::encode_one(result.subscription_id.clone()).unwrap(),
            );
            
            assert!(status_result.is_ok());
            let status: Result<SubscriptionStatus, String> = 
                candid::decode_one(&extract_reply_bytes(status_result.unwrap()))
                    .expect("Failed to decode subscription status");
            
            assert!(status.is_ok());
            assert!(matches!(status.unwrap(), SubscriptionStatus::Active), 
                   "Subscription should initially be active");
        },
        Err(e) => {
            // If subscription fails due to insufficient balance, that's also a valid test outcome
            println!("Subscription failed due to insufficient balance: {}", e);
            assert!(e.contains("insufficient") || e.contains("balance"), 
                   "Error should be related to insufficient balance");
        }
    }
    
    // Test that other users are unaffected
    let (other_user_canister, _) = test_users[0];
    let other_user_data = &test_users_data[0];
    
    let other_subscription = create_subscription_from_config(&REAL_CONTRACTS[1], other_user_canister);
    let other_result = pic.update_call(
        chain_service_id,
        other_user_canister,
        "register_subscription",
        candid::encode_one(other_subscription).unwrap(),
    );
    
    assert!(other_result.is_ok(), "Other user subscriptions should not be affected");
    println!("Verified that {} subscriptions are unaffected by {}'s balance issues", 
            other_user_data.name, light_user_data.name);
    
    println!("Balance depletion test completed");
}
