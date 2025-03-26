mod internals;
mod test_config;

use std::{str::FromStr, time::Duration};

use anyhow::Result;
use candid::{self, Nat, Principal};
use evm_logs_types::{Event, EventNotification, Filter, SubscriptionRegistration};
use evm_rpc_types::{Hex, Hex20, Hex32, LogEntry};
use internals::*;
use pocket_ic::{WasmResult, management_canister::CanisterId, nonblocking::PocketIc};
use tokio::time::sleep;

use crate::test_config::TestConfig;

static EVENT_DATA: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffe61b66a6b5b0dc6a000000000000000000000000000000000000000000000000000000017ab51b0e00000000000000000000000000000000000000000003d2da2f154b7d200000000000000000000000000000000000000000000000000000006bf4f47dc85f3730fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd064f";
const ADDR1_HEX20: &str = "0xb2cc224c1c9feE385f8ad6a55b4d94E92359DC59";
const TOPIC1_HEX32: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

#[tokio::test]
async fn test_event_publishing_and_notification_delivery() -> Result<()> {
  let (mut pic, evm_logs_can_id, _evm_rpc_mocked_can_id, _proxy_can_id, cycles_wallet_can_id) =
    init_pocket_ic_all_cans().await?;
  let subscriber_can_id = init_pocket_ic_subscribers(&mut pic, 1).await?[0];

  let addr1 = Hex20::from_str(ADDR1_HEX20).unwrap();
  let topic1 = Hex32::from_str(TOPIC1_HEX32).unwrap();

  // Register a subscription from the subscriber canister
  let sub_registration = SubscriptionRegistration {
    chain_id: 8453,
    filter: Filter {
      address: addr1.clone(), // Example address
      topics: Some(vec![vec![topic1.clone()]]),
    },
    memo: None,
    canister_to_top_up: subscriber_can_id,
  };
  let sub_reg_encoded = candid::encode_args((sub_registration,))?;

  let call_args = WalletCall128Args {
    canister: evm_logs_can_id,
    method_name: "subscribe".to_string(),
    args: sub_reg_encoded,
    cycles: 2_000_000_000_000u128.into(),
  };
  let bytes = candid::encode_args((call_args,)).expect("Failed to encode wallet_call128 args");

  let subscribe_via_cycles_wallet = pic
    .update_call(cycles_wallet_can_id, Principal::anonymous(), "wallet_call128", bytes)
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
    timestamp: 0,
    chain_id: 8453,
    log_entry: LogEntry {
      address: addr1.clone(),
      topics: vec![topic1.clone()],
      data: Hex::from(hex::decode(EVENT_DATA)?),
      block_number: None,
      transaction_hash: None,
      transaction_index: None,
      block_hash: None,
      log_index: None,
      removed: false,
    },
  };
  let _ = pic
    .update_call(
      evm_logs_can_id,
      Principal::anonymous(),
      "publish_events",
      candid::encode_one(vec![event.clone()])?,
    )
    .await;

  // Wait for the notification to be sent
  sleep(Duration::from_millis(500)).await;

  // Query the subscriber canister to retrieve notifications
  let get_notifications_result = pic
    .query_call(
      subscriber_can_id,
      Principal::anonymous(),
      "get_notifications",
      candid::encode_args(())?,
    )
    .await;

  // Verify that the subscriber received the notification
  match get_notifications_result {
    Ok(WasmResult::Reply(data)) => {
      let notifications: Vec<EventNotification> = candid::decode_one(&data)?;
      println!("Received notifications: {:?}", notifications);
      assert_eq!(notifications.len(), 1, "Expected one notification");

      let notification = &notifications[0];
      assert_eq!(notification.chain_id, 8453, "Incorrect chain_id in notification");
      assert_eq!(notification.event_id, Nat::from(1u64), "Incorrect event_id");

      let event_data_bytes = hex::decode(EVENT_DATA)?;
      assert_eq!(
        notification.log_entry.data.as_ref(),
        event_data_bytes,
        "Incorrect event data"
      );

      assert!(notification.filter.is_none(), "Expected no filter");
    }
    Ok(WasmResult::Reject(err)) => {
      panic!("Get notifications rejected: {:?}", err);
    }
    Err(e) => {
      panic!("Get notifications call error: {:?}", e);
    }
  }
  Ok(())
}

/// Initializes PocketIc and creates canisters without subscribers in such order:
/// * evm_logs_can_id
/// * evm_rpc_mocked_can_id
/// * proxy_can_id
/// * cycles_wallet_can_id
async fn init_pocket_ic_all_cans() -> Result<(PocketIc, CanisterId, CanisterId, CanisterId, CanisterId)> {
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

async fn init_pocket_ic_subscribers(pic: &mut PocketIc, amount: usize) -> Result<Vec<CanisterId>> {
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
