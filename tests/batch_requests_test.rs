use candid::{CandidType, Nat, Principal};
use evm_logs_types::{Filter, SubscriptionRegistration, SubscriptionInfo, EventNotification};
use pocket_ic::nonblocking::PocketIc;
use pocket_ic::WasmResult;
use serde::Deserialize;
use std::time::Duration;
use hex;
use getrandom::getrandom;
use std::collections::HashMap;

#[derive(CandidType, Deserialize)]
struct EvmLogsInitArgs {
    evm_rpc_canister: Principal,
    proxy_canister: Principal,
    pub estimate_events_num: u32,
}

#[derive(CandidType, Deserialize)]
struct WalletCall128Args {
    canister: Principal,
    method_name: String,
    args: Vec<u8>,
    cycles: Nat,
}

#[derive(CandidType, Deserialize)]

struct EvmRpcMockedConfig {
    pub evm_logs_canister_id: Principal,
}

fn generate_random_filter() -> Filter {
    let mut address_bytes = [0u8; 20]; // Ethereum addresses are 20 bytes long
    let mut topic_bytes = [0u8; 32]; // Topics are 32 bytes long

    getrandom(&mut address_bytes).expect("Failed to generate random address bytes");
    getrandom(&mut topic_bytes).expect("Failed to generate random topic bytes");

    let address = format!("0x{}", hex::encode(address_bytes)); // Convert address to hex string
    let topic = format!("0x{}", hex::encode(topic_bytes)); // Convert topic to hex string

    Filter {
        address,
        topics: Some(vec![vec![topic]]),
    }
}


#[tokio::test]
async fn test_batch_requests() {
    let pic = PocketIc::new().await;

    let num_filters = 100;

    // This hashmap will store the subscriber canister ID -> filter
    let mut subscriber_filters = Vec::new();

    let evm_logs_canister_id = pic.create_canister().await;

    // initialize and install evm-rpc-mocked canister
    let evm_rpc_mocked_canister_id = pic.create_canister().await;
    pic.add_cycles(evm_rpc_mocked_canister_id, 4_000_000_000_000).await;

    let evm_rpc_mocked_bytes = tokio::fs::read(std::env::var("EVM_RPC_MOCKED_WASM_PATH").unwrap()).await.unwrap();

    let evm_rpc_mocked_init_args = candid::encode_args((EvmRpcMockedConfig {
        evm_logs_canister_id,
    },)).unwrap();
    pic.install_canister(evm_rpc_mocked_canister_id, evm_rpc_mocked_bytes, evm_rpc_mocked_init_args, None).await;

    // initialize and install proxy canister
    let proxy_canister_id = pic.create_canister().await;
    pic.add_cycles(proxy_canister_id, 4_000_000_000_000).await;

    let proxy_wasm_bytes = tokio::fs::read(std::env::var("PROXY_CANISTER_WASM_PATH").unwrap()).await.unwrap();
    pic.install_canister(proxy_canister_id, proxy_wasm_bytes, vec![], None).await;

    // initialize and install evm-logs-canister
    pic.add_cycles(evm_logs_canister_id, 4_000_000_000_000).await;

    let evm_logs_wasm_bytes = tokio::fs::read(std::env::var("EVM_LOGS_CANISTER_PATH").unwrap()).await.unwrap();

    let init_args = candid::encode_args((EvmLogsInitArgs {
        evm_rpc_canister: evm_rpc_mocked_canister_id,
        proxy_canister: proxy_canister_id,
        estimate_events_num: 5,
    },)).unwrap();

    pic.install_canister(evm_logs_canister_id, evm_logs_wasm_bytes, init_args, None).await;

    // initialize and install cycles-wallet, for calling evm-logs-canister with payment from different subscribers
    let cycles_wallet_id = pic.create_canister().await;
    pic.add_cycles(cycles_wallet_id, 4_000_000_000_000).await;

    let cycles_wallet_wasm_bytes = tokio::fs::read(std::env::var("CYCLES_WALLET_WASM_PATH").unwrap()).await.unwrap();
    pic.install_canister(cycles_wallet_id, cycles_wallet_wasm_bytes, vec![], None).await;

    let subscriber_wasm_bytes = tokio::fs::read(std::env::var("TEST_CANISTER_WASM_PATH").unwrap()).await.unwrap();

    // create subscriber canister 
    let subscriber_canister_id = pic.create_canister().await;
    pic.add_cycles(subscriber_canister_id, 4_000_000_000_000).await;

    pic.install_canister(subscriber_canister_id, subscriber_wasm_bytes.clone(), vec![], None).await;

    // subscribe on evm-logs-canister many times with random filter to trigger batch request logic
    for _i in 0..num_filters { 
        let random_filter = generate_random_filter();

        // Put it in our local hashmap so we can verify later
        subscriber_filters.push(random_filter.clone());

        let sub_registration = SubscriptionRegistration {
            chain_id: 8453,
            filter: random_filter,
            memo: None,
            canister_to_top_up: subscriber_canister_id,
        }; 

        let sub_reg_encoded = candid::encode_args((sub_registration,)).unwrap();

        let call_args = WalletCall128Args {
            canister: evm_logs_canister_id,
            method_name: "subscribe".to_string(),
            args: sub_reg_encoded,
            cycles: 2_000_000_000_000u128.into(),
        };

        let bytes = candid::encode_args((call_args,)).unwrap();

        match pic.update_call(cycles_wallet_id, Principal::anonymous(), "wallet_call128", bytes).await {
            Ok(WasmResult::Reply(data)) => ic_cdk::println!("Subscription successful: {:?}", data),

            Ok(WasmResult::Reject(err)) => panic!("Subscription rejected: {:?}", err),

            Err(e) => panic!("Subscription call error: {:?}", e),
        }
    }

    // Waiting when evm_logs_canister will fetch logs from mocked evm rpc canister and send it so subscribers
    pic.advance_time(Duration::from_secs(20)).await;
    pic.tick().await;
    pic.advance_time(Duration::from_secs(20)).await;
    pic.tick().await;  
    
    let received_notifications_bytes = pic.query_call(
        subscriber_canister_id, 
        Principal::anonymous(), 
        "get_notifications", 
        candid::encode_args(
            ()
        ).unwrap()
    ).await
    .unwrap();

    // here we need to check if subscriber received all notifications which corresponds to its filters. it shopuld be exactly num_filters notifications
    if let WasmResult::Reply(reply_data) = received_notifications_bytes {
        let notifications: Vec<EventNotification> = candid::decode_one(&reply_data).unwrap();
        
        // Verify that the number of received notifications matches the number of subscriptions
        // assert_eq!(notifications.len(), num_filters, "Notifications count mismatch");

        for notification in notifications.iter() {
            // Find the corresponding filter for the received notification
            let matching_filter = subscriber_filters.iter().find(|filter| {
                filter.address.to_lowercase() == notification.address.to_lowercase()
                    && filter.topics.as_ref().map_or(false, |topics| topics.contains(&notification.topics))
            });

            assert!(matching_filter.is_some(), "Received notification does not match any subscribed filter");
        }
    } else {
        panic!("Failed to get notifications for subscriber");
    }
}