mod internals;
mod test_config;

use std::{collections::HashMap, time::Duration};

use anyhow::{Result, anyhow};
use candid::{Nat, Principal};
use evm_logs_types::{EventNotification, Filter, SubscriptionInfo, SubscriptionRegistration};
use internals::*;
use pocket_ic::{WasmResult, management_canister::CanisterId, nonblocking::PocketIc};

use crate::test_config::TestConfig;

#[tokio::test]
async fn test_main_workflow_with_bunch_subscribers() -> Result<()> {
  // This hashmap will store the subscriber canister ID -> filter
  let mut subscriber_filters = HashMap::<Principal, Filter>::new();

  let (mut pic, evm_logs_can_id, _evm_rpc_mocked_can_id, _proxy_can_id, cycles_wallet_can_id) =
    init_pocket_ic_all_cans().await?;

  let num_subscribers = 2;
  let subscribers = init_pocket_ic_subscribers(&mut pic, num_subscribers).await?;

  // subscribe on evm-logs-canister from each subscriber with random topic
  for subscriber_canister_id in subscribers.iter() {
    let filter = generate_random_filter();

    // Put it in our local hashmap so we can verify later
    subscriber_filters.insert(*subscriber_canister_id, filter.clone());

    let sub_registration = SubscriptionRegistration {
      chain_id: 8453,
      filter,
      memo: None,
      canister_to_top_up: *subscriber_canister_id,
    };

    let sub_reg_encoded = candid::encode_args((sub_registration,))?;

    let call_args = WalletCall128Args {
      canister: evm_logs_can_id,
      method_name: "subscribe".to_string(),
      args: sub_reg_encoded,
      cycles: 2_000_000_000_000u128.into(),
    };

    let bytes = candid::encode_args((call_args,))?;

    match pic
      .update_call(cycles_wallet_can_id, Principal::anonymous(), "wallet_call128", bytes)
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
      evm_logs_can_id,
      Principal::anonymous(),
      "get_subscriptions",
      candid::encode_args((None::<i32>, None::<Nat>, None::<Vec<Filter>>))?,
    )
    .await
    .map_err(|e| anyhow!("Failed to get subscriptions: {e:?}"))?;

  if let WasmResult::Reply(data) = sub_info_bytes {
    let subscriptions: Vec<SubscriptionInfo> = candid::decode_one(&data)?;
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

  for subscriber_canister_id in subscribers.iter() {
    let received_notifications_bytes = pic
      .query_call(
        *subscriber_canister_id,
        Principal::anonymous(),
        "get_notifications",
        candid::encode_args(())?,
      )
      .await
      .map_err(|e| anyhow!("Failed to get notifications, err: {e:?}"))?;

    if let WasmResult::Reply(reply_data) = received_notifications_bytes {
      let notifications: Vec<EventNotification> = candid::decode_one(&reply_data)?;

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
  Ok(())
}

/// Initializes PocketIc and creates canisters without subscribers in such order:
/// * evm_logs_can_id
/// * evm_rpc_mocked_can_id
/// * proxy_can_id
/// * cycles_wallet_can_id
async fn init_pocket_ic_all_cans() -> anyhow::Result<(PocketIc, CanisterId, CanisterId, CanisterId, CanisterId)> {
  let pic = PocketIc::new().await;
  let test_config = TestConfig::new()?;

  let evm_logs_can_id = pic.create_canister().await;
  let evm_rpc_mocked_can_id = pic.create_canister().await;
  let proxy_can_id = pic.create_canister().await;
  let cycles_wallet_can_id = pic.create_canister().await;

  pic.add_cycles(evm_rpc_mocked_can_id, DEFAULT_CYCLES_VALUE).await;
  let evm_rpc_mocked_bytes = tokio::fs::read(test_config.evm_rpc_mocked_wasm_path).await?;
  let evm_rpc_mocked_init_args = candid::encode_args((EvmRpcMockedConfig {
    evm_logs_canister_id: evm_logs_can_id,
  },))?;
  pic
    .install_canister(
      evm_rpc_mocked_can_id,
      evm_rpc_mocked_bytes,
      evm_rpc_mocked_init_args,
      None,
    )
    .await;

  pic.add_cycles(proxy_can_id, DEFAULT_CYCLES_VALUE).await;
  let proxy_wasm_bytes = tokio::fs::read(test_config.proxy_canister_wasm_path).await?;
  pic.install_canister(proxy_can_id, proxy_wasm_bytes, vec![], None).await;

  pic.add_cycles(evm_logs_can_id, DEFAULT_CYCLES_VALUE).await;
  let evm_logs_wasm_bytes = tokio::fs::read(test_config.evm_logs_canister_wasm_path).await?;
  let init_args = candid::encode_args((EvmLogsInitArgs {
    evm_rpc_canister: evm_rpc_mocked_can_id,
    proxy_canister: proxy_can_id,
    estimate_events_num: 5,
    max_response_bytes: 10000,
  },))?;
  pic
    .install_canister(evm_logs_can_id, evm_logs_wasm_bytes, init_args, None)
    .await;

  // initialize and install cycles-wallet, for calling evm-logs-canister with payment from different subscribers
  pic.add_cycles(cycles_wallet_can_id, DEFAULT_CYCLES_VALUE).await;
  let cycles_wallet_wasm_bytes = tokio::fs::read(test_config.cycles_wallet_wasm_path).await?;
  pic
    .install_canister(cycles_wallet_can_id, cycles_wallet_wasm_bytes, vec![], None)
    .await;

  Ok((
    pic,
    evm_logs_can_id,
    evm_rpc_mocked_can_id,
    proxy_can_id,
    cycles_wallet_can_id,
  ))
}

async fn init_pocket_ic_subscribers(pic: &mut PocketIc, amount: usize) -> anyhow::Result<Vec<CanisterId>> {
  let test_config = TestConfig::new()?;
  let mut subscribers = Vec::with_capacity(amount);
  let subscriber_wasm_bytes = tokio::fs::read(test_config.test_canister_wasm_path).await?;

  for _ in 0..amount {
    let subscriber_can_id = pic.create_canister().await;
    pic.add_cycles(subscriber_can_id, DEFAULT_CYCLES_VALUE).await;
    pic
      .install_canister(subscriber_can_id, subscriber_wasm_bytes.clone(), vec![], None)
      .await;
    subscribers.push(subscriber_can_id);
  }
  Ok(subscribers)
}
