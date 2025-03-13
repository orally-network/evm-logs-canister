mod common;

use std::{collections::HashMap, time::Duration};

use candid::{Nat, Principal};
use common::*;
use evm_logs_types::{EventNotification, Filter, SubscriptionInfo, SubscriptionRegistration};
use pocket_ic::{WasmResult, nonblocking::PocketIc};

#[tokio::test]
async fn test_main_worflow_with_bunch_subscribers() {
  let pic = PocketIc::new().await;

  let num_subscribers = 2;

  // This hashmap will store the subscriber canister ID -> filter
  let mut subscriber_filters = HashMap::<Principal, Filter>::new();

  let evm_logs_canister_id = pic.create_canister().await;

  // initialize and install evm-rpc-mocked canister
  let evm_rpc_mocked_canister_id = pic.create_canister().await;
  pic.add_cycles(evm_rpc_mocked_canister_id, 4_000_000_000_000).await;

  let evm_rpc_mocked_bytes = tokio::fs::read(std::env::var("EVM_RPC_MOCKED_WASM_PATH").unwrap())
    .await
    .unwrap();

  let evm_rpc_mocked_init_args = candid::encode_args((EvmRpcMockedConfig { evm_logs_canister_id },)).unwrap();
  pic
    .install_canister(
      evm_rpc_mocked_canister_id,
      evm_rpc_mocked_bytes,
      evm_rpc_mocked_init_args,
      None,
    )
    .await;

  // initialize and install proxy canister
  let proxy_canister_id = pic.create_canister().await;
  pic.add_cycles(proxy_canister_id, 4_000_000_000_000).await;

  let proxy_wasm_bytes = tokio::fs::read(std::env::var("PROXY_CANISTER_WASM_PATH").unwrap())
    .await
    .unwrap();
  pic
    .install_canister(proxy_canister_id, proxy_wasm_bytes, vec![], None)
    .await;

  // initialize and install evm-logs-canister
  pic.add_cycles(evm_logs_canister_id, 4_000_000_000_000).await;

  let evm_logs_wasm_bytes = tokio::fs::read(std::env::var("EVM_LOGS_CANISTER_PATH").unwrap())
    .await
    .unwrap();

  let init_args = candid::encode_args((EvmLogsInitArgs {
    evm_rpc_canister: evm_rpc_mocked_canister_id,
    proxy_canister: proxy_canister_id,
    estimate_events_num: 5,
    max_response_bytes: 1_000_000,
  },))
  .unwrap();

  pic
    .install_canister(evm_logs_canister_id, evm_logs_wasm_bytes, init_args, None)
    .await;

  // initialize and install cycles-wallet, for calling evm-logs-canister with payment from different subscribers
  let cycles_wallet_id = pic.create_canister().await;
  pic.add_cycles(cycles_wallet_id, 4_000_000_000_000).await;

  let cycles_wallet_wasm_bytes = tokio::fs::read(std::env::var("CYCLES_WALLET_WASM_PATH").unwrap())
    .await
    .unwrap();
  pic
    .install_canister(cycles_wallet_id, cycles_wallet_wasm_bytes, vec![], None)
    .await;

  // all subscribers will have the same WASM file
  let subscriber_wasm_bytes = tokio::fs::read(std::env::var("TEST_CANISTER_WASM_PATH").unwrap())
    .await
    .unwrap();
  let mut subscriber_canisters = Vec::new();

  // create subscribers caisters
  for _ in 0..num_subscribers {
    let subscriber_canister_id = pic.create_canister().await;
    pic.add_cycles(subscriber_canister_id, 4_000_000_000_000).await;

    pic
      .install_canister(subscriber_canister_id, subscriber_wasm_bytes.clone(), vec![], None)
      .await;

    subscriber_canisters.push(subscriber_canister_id);
  }

  // subscribe on evm-logs-canister from each subscriber with random topic
  for subscriber_canister_id in subscriber_canisters.clone() {
    let filter = generate_random_filter();

    // Put it in our local hashmap so we can verify later
    subscriber_filters.insert(subscriber_canister_id, filter.clone());

    let sub_registration = SubscriptionRegistration {
      chain_id: 8453,
      filter,
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

    match pic
      .update_call(cycles_wallet_id, Principal::anonymous(), "wallet_call128", bytes)
      .await
    {
      Ok(WasmResult::Reply(data)) => ic_cdk::println!("Subscription successful: {:?}", data),

      Ok(WasmResult::Reject(err)) => panic!("Subscription rejected: {:?}", err),

      Err(e) => panic!("Subscription call error: {:?}", e),
    }
  }

  // Verify subscription count

  let sub_info_bytes = pic
    .query_call(
      evm_logs_canister_id,
      Principal::anonymous(),
      "get_subscriptions",
      candid::encode_args((None::<i32>, None::<Nat>, None::<Vec<Filter>>)).unwrap(),
    )
    .await
    .unwrap();

  if let WasmResult::Reply(data) = sub_info_bytes {
    let subscriptions: Vec<SubscriptionInfo> = candid::decode_one(&data).unwrap();

    assert_eq!(subscriptions.len(), num_subscribers, "Subscription count mismatch");
  } else {
    panic!("Failed to get subscriptions");
  }

  // Waiting when evm_logs_canister will fetch logs from mocked evm rpc canister and send it so subscribers
  // TODO adjust time dynamicaly?
  pic.advance_time(Duration::from_secs(20)).await;
  pic.tick().await;
  pic.advance_time(Duration::from_secs(20)).await;
  pic.tick().await;

  for subscriber_canister_id in subscriber_canisters {
    let received_notifications_bytes = pic
      .query_call(
        subscriber_canister_id,
        Principal::anonymous(),
        "get_notifications",
        candid::encode_args(()).unwrap(),
      )
      .await
      .unwrap();

    if let WasmResult::Reply(reply_data) = received_notifications_bytes {
      let notifications: Vec<EventNotification> = candid::decode_one(&reply_data).unwrap();

      // We assume that each subscriber should get exactly one notification
      assert_eq!(notifications.len(), 1, "Notifications count mismatch");

      let notification = &notifications[0];

      let stored_filter = subscriber_filters
        .get(&subscriber_canister_id)
        .expect("Filter not found for subscriber");

      let stored_address = stored_filter.address.clone().to_string().to_lowercase();
      let stored_topics = &stored_filter.topics.as_ref().unwrap()[0];

      // Check that the notification's address matches the address we originally used in the filter
      assert_eq!(
        notification.log_entry.address.to_string(),
        stored_address,
        "Notification address does not match the original subscriber filter"
      );

      // Check that the notification's address matches the address we originally used in the filter
      assert_eq!(
        &notification.log_entry.topics, stored_topics,
        "Notification topic does not match the original subscriber filter"
      );
    } else {
      panic!("Failed to get notifications for subscriber");
    }
  }
}
