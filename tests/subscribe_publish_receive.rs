/// This test validates the flow of event subscriptions and notifications publication 
/// in the `evm_logs` canister. It ensures that:
///
/// - Canister Setup & Initialization:
///   - Subscriber, Proxy, and EVM Logs canisters are created and installed correctly.
///
/// - Subscription Registration:
///   - A subscription is registered by the subscriber canister with a specific `chain_id` and `filter`.
///   - The subscription request is sent via an `update_call` to the `evm_logs` canister.
///   - Future improvement: Validate the subscription registration response to ensure success.
///
/// - Event Publishing:
///   - An event is constructed with a predefined `chain_id`, `address`, and `topics`.
///   - The event is published using an `update_call` to the `publish_events` method of `evm_logs`.
///
/// - Notification Delivery:
///   - The subscriber canister is queried for received notifications.
///   - The response is validated to ensure:
///     - One notification was received.
///     - The event metadata (`chain_id`, `event_id`, `data`) matches the published event.

use candid;
use candid::Nat;
use candid::Principal;
use evm_logs_types::{
    Event, EventNotification, Filter, Value,
    SubscriptionRegistration,
};
use pocket_ic::nonblocking::PocketIc;
use pocket_ic::WasmResult;
use std::time::Duration;
use tokio::time::sleep;

use candid::CandidType;
use serde::Deserialize;

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


static EVENT_DATA: &str = "0xffffffffffffffffffffffffffffffffffffffffffffffffe61b66a6b5b0dc6a000000000000000000000000000000000000000000000000000000017ab51b0e00000000000000000000000000000000000000000003d2da2f154b7d200000000000000000000000000000000000000000000000000000006bf4f47dc85f3730fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd064f";


// 1. Create and install 100 subscriber canisters
// let mut subscriber_canisters = Vec::new();
// for i in 0..100 {
//     let subscriber_canister_id = pic.create_canister().await;
//     pic.add_cycles(subscriber_canister_id, 4_000_000_000_000).await;

//     // Optionally pass an initialization arg (e.g. which subscriber index)
//     // so each canister knows it is subscriber #i:
//     let init_args = candid::encode_args((i,))?;  
//       // Only if your subscriber canister has an init that takes an integer or something

//     pic.install_canister(
//         subscriber_canister_id,
//         subscriber_wasm_bytes.to_vec(),
//         init_args,
//         None,
//     ).await;

//     subscriber_canisters.push(subscriber_canister_id);
// }

#[tokio::test]
async fn test_event_publishing_and_notification_delivery() {
    let pic = PocketIc::new().await;

    // Create the subscriber canister
    let subscriber_canister_id = pic.create_canister().await;
    ic_cdk::println!("Added cycled for subscriber_id: {:?}", subscriber_canister_id.to_text());
    pic.add_cycles(subscriber_canister_id, 4_000_000_000_000)
        .await;

    // Install the subscriber wasm
    let subscriber_wasm_path =
        std::env::var("TEST_CANISTER_WASM_PATH").expect("TEST_CANISTER_WASM_PATH must be set");
    let subscriber_wasm_bytes = tokio::fs::read(subscriber_wasm_path)
        .await
        .expect("Failed to read the subscriber WASM file");
    pic.install_canister(
        subscriber_canister_id,
        subscriber_wasm_bytes.to_vec(),
        vec![],
        None,
    )
    .await;

    let proxy_canister_id = pic.create_canister().await;
    pic.add_cycles(proxy_canister_id, 4_000_000_000_000)
        .await;

    // Install proxy wasm
    let proxy_canister_wasm_path =
        std::env::var("PROXY_CANISTER_WASM_PATH").expect("PROXY_CANISTER_WASM_PATH must be set");
    let proxy_wasm_bytes = tokio::fs::read(proxy_canister_wasm_path)
        .await
        .expect("Failed to read the proxy canister WASM file");
    pic.install_canister(
        proxy_canister_id,
        proxy_wasm_bytes.to_vec(),
        vec![],
        None,
    )
    .await;

    // create evm_logs canister
    let evm_logs_canister_id = pic.create_canister().await;
    pic.add_cycles(evm_logs_canister_id, 4_000_000_000_000)
        .await;

    let evm_logs_wasm_path =
        std::env::var("EVM_LOGS_CANISTER_PATH").expect("EVM_LOGS_CANISTER_PATH must be set");
    let evm_logs_wasm_bytes = tokio::fs::read(evm_logs_wasm_path)
        .await
        .expect("Failed to read the subscription manager WASM file");

    let init_args_value = EvmLogsInitArgs {
        evm_rpc_canister: Principal::from_text("aaaaa-aa").expect("EVM_RPC_CANISTER incorrect principal"),
        proxy_canister: proxy_canister_id,
        estimate_events_num: 5, // test
    };
    
    let init_args = candid::encode_args((init_args_value,))
    .expect("Failed to encode init arguments");

    pic.install_canister(
        evm_logs_canister_id,
        evm_logs_wasm_bytes.to_vec(),
        init_args,
        None,
    )
    .await;

    // create cycles_wallet canister
    let cycles_wallet_id = pic.create_canister().await;
    pic.add_cycles(cycles_wallet_id, 4_000_000_000_000)
        .await;

    let cycles_wallet_wasm_path =
        std::env::var("CYCLES_WALLET_WASM_PATH").expect("CYCLES_WALLET_WASM_PATH must be set");
    let cycles_wallet_wasm_bytes = tokio::fs::read(cycles_wallet_wasm_path)
        .await
        .expect("Failed to read the cycles wallet WASM file");

    pic.install_canister(
        cycles_wallet_id,
        cycles_wallet_wasm_bytes.to_vec(),
        vec![],
        None,
    )
    .await;

    // Register a subscription from the subscriber canister
    let sub_registration = SubscriptionRegistration {
        chain_id: 8453,
        filter: Filter {
            address: "0xb2cc224c1c9feE385f8ad6a55b4d94E92359DC59".to_string(), // Example address
            topics: Some(vec![vec!["0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67".to_string()]]),
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
    let bytes = candid::encode_args((call_args,))
        .expect("Failed to encode wallet_call128 args");

    let subscribe_via_cycles_wallet = pic
        .update_call(
            cycles_wallet_id,
            Principal::anonymous(),
            "wallet_call128",
            bytes,
        )
        .await;

    match subscribe_via_cycles_wallet {
        Ok(WasmResult::Reply(data)) => {
            ic_cdk::println!("Reply: {:?}", data);
        }
        Ok(WasmResult::Reject(err)) => {
            panic!("Subscription rejected: {:?}", err);
        }
        Err(e) => {
            panic!("Subscription call error: {:?}", e);
        }
    }
    
    // TODO Check the subscription registration result

    // Publish an event
    let event = Event {
        id: Nat::from(0u64), // ID will be assigned by the canister
        prev_id: None,
        timestamp: 0,
        chain_id: 8453,
        data: Value::Text(EVENT_DATA.to_string()),
        headers: None,
        address: "0xb2cc224c1c9feE385f8ad6a55b4d94E92359DC59".to_string(), // Example address
        topics: Some(vec!["0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67".to_string()]),                                                      // Example topic
        tx_hash: "".to_string(),
    };
    let _ = pic
        .update_call(
            evm_logs_canister_id,
            Principal::anonymous(),
            "publish_events",
            candid::encode_one(vec![event.clone()]).unwrap(),
        )
        .await;

    // Wait for the notification to be sent
    sleep(Duration::from_millis(500)).await;

    // Query the subscriber canister to retrieve notifications
    let get_notifications_result = pic
        .query_call(
            subscriber_canister_id,
            Principal::anonymous(),
            "get_notifications",
            candid::encode_args(()).unwrap(),
        )
        .await;

    // Verify that the subscriber received the notification
    match get_notifications_result {
        Ok(WasmResult::Reply(data)) => {
            let notifications: Vec<EventNotification> = candid::decode_one(&data).unwrap();
            println!("Received notifications: {:?}", notifications);
            assert_eq!(notifications.len(), 1, "Expected one notification");
            let notification = &notifications[0];
            assert_eq!(
                notification.chain_id, 8453,
                "Incorrect chain_id in notification"
            );
            assert_eq!(notification.event_id, Nat::from(1u64), "Incorrect event_id");
            if let Value::Text(ref text) = notification.data {
                assert_eq!(text, &EVENT_DATA.to_string(), "Incorrect event data");
            } else {
                panic!("Unexpected data type in notification");
            }
            assert!(notification.filter.is_none(), "Expected no filter");
        }
        Ok(WasmResult::Reject(err)) => {
            panic!("Get notifications rejected: {:?}", err);
        }
        Err(e) => {
            panic!("Get notifications call error: {:?}", e);
        }
    }
}
