use candid::{CandidType, Nat, Principal};
use evm_logs_types::{Filter, SubscriptionRegistration, SubscriptionInfo, EventNotification};
use pocket_ic::nonblocking::PocketIc;
use pocket_ic::WasmResult;
use serde::Deserialize;
use std::time::Duration;
use hex;
use getrandom::getrandom;

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

fn generate_random_topic() -> String {
    let mut random_bytes = [0u8; 32]; // 32 bytes for a valid topic
    getrandom(&mut random_bytes).expect("Failed to generate random bytes");
    format!("0x{}", hex::encode(random_bytes)) // Convert to hex string
}

/// This test verifies the main workflow of the EVM logs canister with multiple subscribers.
/// 
/// ## Overview:
/// - It sets up a simulated Internet Computer environment using PocketIc.
/// - It deploys and initializes multiple canisters: `evm-logs-canister`, `evm-rpc-mocked`, `proxy`, `cycles-wallet`, 
/// and multiple subscriber canisters(const value in the code).
/// - Each subscriber canister subscribes to the `evm-logs-canister` with a randomly generated filter.
/// - The test ensures that all subscriptions are correctly registered(subscription count match).
/// - It advances time and triggers the event processing cycle to simulate the logs fetching.
/// - Finally, it verifies that each subscriber received the expected event notification.
///
/// ## Key Assertions:
/// - The number of registered subscriptions matches the expected count.
/// - Each subscriber receives exactly one event notification after the logs are fetched and processed.
/// 
/// This test ensures the correctness of the subscription workflow and event delivery mechanism in a controlled local environment.

#[tokio::test]
async fn test_main_worflow_with_bunch_subscribers() {
    let pic = PocketIc::new().await;

    let num_subscribers = 3;
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

    // all subscribers will have the same WASM file  
    let subscriber_wasm_bytes = tokio::fs::read(std::env::var("TEST_CANISTER_WASM_PATH").unwrap()).await.unwrap();
    let mut subscriber_canisters = Vec::new();

    // create subscribers caisters 
    for _ in 0..num_subscribers {
        let subscriber_canister_id = pic.create_canister().await;
        pic.add_cycles(subscriber_canister_id, 4_000_000_000_000).await;

        pic.install_canister(subscriber_canister_id, subscriber_wasm_bytes.clone(), vec![], None).await;

        subscriber_canisters.push(subscriber_canister_id);
    }

    // subscribe on evm-logs-canister from each subscriber with random topic 
    for subscriber_canister_id in subscriber_canisters.clone() {
        let random_topic = generate_random_topic();

        let sub_registration = SubscriptionRegistration {
            chain_id: 8453,
            filter: Filter {
                address: "0xb2cc224c1c9feE385f8ad6a55b4d94E92359DC59".to_string(),
                topics: Some(vec![vec![random_topic]]),
            },
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

    // Verify subscription count

    let sub_info_bytes = pic.query_call(
        evm_logs_canister_id, 
        Principal::anonymous(), 
        "get_subscriptions", 
        candid::encode_args(
            (None::<i32>, None::<Nat>, None::<Vec<Filter>>)
        ).unwrap()
    ).await
    .unwrap();

    if let WasmResult::Reply(data) = sub_info_bytes {
        let subscriptions: Vec<SubscriptionInfo> = candid::decode_one(&data).unwrap();

        assert_eq!(subscriptions.len(), num_subscribers, "Subscription count mismatch");

    } 
    else {
        panic!("Failed to get subscriptions");
    }

    // Waiting when evm_logs_canister will fetch logs from mocked evm rpc canister and send it so subscribers
    // TODO adjust time dynamicaly? 
    pic.advance_time(Duration::from_secs(20)).await;
    pic.tick().await;
    pic.advance_time(Duration::from_secs(20)).await;
    pic.tick().await;    

    for subscriber_canister_id in subscriber_canisters {

        let received_notifications_bytes = pic.query_call(
            subscriber_canister_id, 
            Principal::anonymous(), 
            "get_notifications", 
            candid::encode_args(
                ()
            ).unwrap()
        ).await
        .unwrap();

        if let WasmResult::Reply(reply_data) = received_notifications_bytes {
            let notifications: Vec<EventNotification> = candid::decode_one(&reply_data).unwrap();
    
            assert_eq!(notifications.len(), 1, "Notifications count mismatch");

            ic_cdk::println!("{:?}", notifications);
        } 
        else {
            panic!("Failed to get notifications for subscriber");
        }

    }

}
