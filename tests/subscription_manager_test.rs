use candid;
use candid::Nat;
use candid::Principal;
use evm_logs_types::{
    Event, EventNotification, Filter, Value, RegisterSubscriptionResult,
    SubscriptionRegistration,
};
use pocket_ic::nonblocking::PocketIc;
use pocket_ic::WasmResult;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_event_publishing_and_notification_delivery() {
    let pic = PocketIc::new().await;

    let subscription_manager_canister_id = pic.create_canister().await;
    pic.add_cycles(subscription_manager_canister_id, 4_000_000_000_000)
        .await;

    let subscription_manager_wasm_path =
        std::env::var("EVM_LOGS_CANISTER_PATH").expect("EVM_LOGS_CANISTER_PATH must be set");
    let subscription_manager_wasm_bytes = tokio::fs::read(subscription_manager_wasm_path)
        .await
        .expect("Failed to read the subscription manager WASM file");
    pic.install_canister(
        subscription_manager_canister_id,
        subscription_manager_wasm_bytes.to_vec(),
        vec![],
        None,
    )
    .await;

    // Create the subscriber canister
    let subscriber_canister_id = pic.create_canister().await;
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

    // Create the subscriber canister
    let proxy_canister_id = pic.create_canister().await;
    pic.add_cycles(proxy_canister_id, 4_000_000_000_000)
        .await;

    // Install the subscriber wasm
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

    // Register a subscription from the subscriber canister
    let subscription_registration = SubscriptionRegistration {
        chain: "Ethereum".to_string(),
        filter: Filter {
            address: "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".to_string(),
            topics: None,
        },
        memo: None,
    };

    let register_subscription_result = pic
        .update_call(
            subscription_manager_canister_id,
            subscriber_canister_id,
            "subscribe",
            candid::encode_one(subscription_registration.clone()).unwrap(),
        )
        .await;

    // Check the subscription registration result
    match register_subscription_result {
        Ok(WasmResult::Reply(data)) => {
            let decoded_result: RegisterSubscriptionResult = candid::decode_one(&data).unwrap();
            match &decoded_result {
                RegisterSubscriptionResult::Ok(sub_id) => {
                    println!("Subscription successfully created, ID: {:?}", sub_id);
                }
                RegisterSubscriptionResult::Err(err) => {
                    panic!("Subscription registration error: {:?}", err);
                }
            }
        }
        Ok(WasmResult::Reject(err)) => {
            panic!("Subscription registration rejected: {:?}", err);
        }
        Err(e) => {
            panic!("Subscription registration call error: {:?}", e);
        }
    }

    // Register a publication
    let publisher_principal = Principal::anonymous();

    // Publish an event
    let event = Event {
        id: Nat::from(0u64), // ID will be assigned by the canister
        prev_id: None,
        timestamp: 0,
        namespace: "Ethereum".to_string(),
        data: Value::Text("Test event data".to_string()),
        headers: None,
        address: "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".to_string(), // Example address
        topics: None,                                                      // Example topic
        tx_hash: "".to_string(),
    };
    let publish_events_result = pic
        .update_call(
            subscription_manager_canister_id,
            publisher_principal,
            "publish_events",
            candid::encode_one(vec![event.clone()]).unwrap(),
        )
        .await;

    // Check the event publishing result
    match publish_events_result {
        Ok(WasmResult::Reply(data)) => {
            let decoded_results: Vec<Option<Result<Vec<Nat>, String>>> =
                candid::decode_one(&data).unwrap();
            match &decoded_results[0] {
                Some(Ok(event_ids)) => {
                    println!("Event published successfully, IDs: {:?}", event_ids);
                    assert_eq!(event_ids.len(), 1, "Expected one event ID");
                    assert_ne!(event_ids[0], Nat::from(0u32), "Event ID should not be zero");
                }
                Some(Err(err)) => {
                    panic!("Event publish error: {:?}", err);
                }
                None => {
                    panic!("Event publish returned None");
                }
            }
        }
        Ok(WasmResult::Reject(err)) => {
            panic!("Event publish rejected: {:?}", err);
        }
        Err(e) => {
            panic!("Event publish call error: {:?}", e);
        }
    }

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
                notification.namespace, "test_namespace",
                "Incorrect namespace"
            );
            assert_eq!(notification.event_id, Nat::from(1u64), "Incorrect event_id");
            if let Value::Text(ref text) = notification.data {
                assert_eq!(text, "Test event data", "Incorrect event data");
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
