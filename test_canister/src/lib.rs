pub mod decoders;
pub mod macros;
pub mod read_contract;
pub mod state;
pub mod utils;

use candid::{CandidType, Deserialize, Principal};
use decoders::{chainfusion_deposit_decoder, ethereum_sync_decoder, primex_deposit_decoder, swap_event_data_decoder};
use evm_logs_types::{EventNotification, UnsubscribeResult};
use ic_cdk::api::call::call;
use ic_cdk_macros::{init, query, update};
use state::{DECODED_NOTIFICATIONS, DECODERS, NOTIFICATIONS};
use utils::*;

use crate::read_contract::SolidityToken;

#[derive(CandidType, Deserialize, Clone)]
struct DecodedNotification {
  notification: EventNotification,
  tokens: Vec<SolidityToken>,
}

#[init]
async fn init() {
  log!("Test Canister Initialized!");
}

// Candid update methods
#[update]
async fn subscribe(evm_logs_canister: Principal) {
  log!("Starting subscription registration");

  let base_swaps_filter = create_base_swaps_config();
  let eth_sync_filter = create_ethereum_sync_config();
  let primex_deposit_filter = create_primex_deposit_config();
  let chainfusion_deposit_filter = create_chainfusion_deposit_config();
  let curve_token_exchange_config = create_curve_token_exchange_config();

  register_subscription_and_map_decoder(evm_logs_canister, base_swaps_filter, swap_event_data_decoder).await;
  register_subscription_and_map_decoder(evm_logs_canister, eth_sync_filter, ethereum_sync_decoder).await;
  register_subscription_and_map_decoder(evm_logs_canister, primex_deposit_filter, primex_deposit_decoder).await;
  register_subscription_and_map_decoder(
    evm_logs_canister,
    chainfusion_deposit_filter,
    chainfusion_deposit_decoder,
  )
  .await;
  register_subscription_and_map_decoder(
    evm_logs_canister,
    curve_token_exchange_config,
    chainfusion_deposit_decoder,
  )
  .await;
}

#[update]
async fn unsubscribe(canister_id: Principal, subscription_id: candid::Nat) {
  log!("Calling unsubscribe for subscription ID: {:?}", subscription_id);

  let result: Result<(evm_logs_types::UnsubscribeResult,), _> =
    call(canister_id, "unsubscribe", (subscription_id.clone(),)).await;
  // TODO remove corresponding decoder
  match result {
    Ok((response,)) => match response {
      UnsubscribeResult::Ok() => {
        log!("Successfully unsubscribed from {:?}", subscription_id)
      }
      UnsubscribeResult::Err(err) => log!("Error unsubscribing: {:?}", err),
    },
    Err(e) => {
      log!("Error calling canister: {:?}", e);
    }
  }
}

#[update]
async fn handle_notification(notification: EventNotification) {
  log!("Received notification for event ID: {:?}", notification.event_id);
  log!("Notification details: {:?}", notification);

  NOTIFICATIONS.with(|notifs| {
    notifs.borrow_mut().push(notification.clone());
  });

  // decode each notification in corresponding way and save decoded data
  DECODERS.with(|decoders| {
    if let Some(decoder) = decoders.borrow().get(&notification.sub_id) {
      match decoder(&notification) {
        Ok(decoded_tokens) => {
          DECODED_NOTIFICATIONS.with(|decoded| {
            decoded.borrow_mut().push((notification.clone(), decoded_tokens));
          });
        }
        Err(e) => {
          log!("Error decoding event data: {:?}", e);
        }
      }
    } else {
      log!("No decoder found for subscription_id: {:?}", notification.sub_id);
    }
  });
}

#[query]
fn get_decoded_notifications() -> Vec<DecodedNotification> {
  DECODED_NOTIFICATIONS.with(|decoded| {
    decoded
      .borrow()
      .iter()
      .map(|(notif, toks)| DecodedNotification {
        notification: notif.clone(),
        tokens: toks.clone(),
      })
      .collect()
  })
}

#[query]
fn get_decoded_notifications_by_subscription(subscription_id: candid::Nat) -> Vec<DecodedNotification> {
  DECODED_NOTIFICATIONS.with(|decoded| {
    decoded
      .borrow()
      .iter()
      .filter(|(notif, _)| notif.sub_id == subscription_id)
      .map(|(notif, toks)| DecodedNotification {
        notification: notif.clone(),
        tokens: toks.clone(),
      })
      .collect()
  })
}

#[query]
fn get_notifications() -> Vec<EventNotification> {
  NOTIFICATIONS.with(|notifs| notifs.borrow().clone())
}

#[update]
async fn get_subscriptions(canister_id: Principal) -> Vec<evm_logs_types::SubscriptionInfo> {
  log!("Calling get_subscriptions");

  let result: Result<(Vec<evm_logs_types::SubscriptionInfo>,), _> =
    call(canister_id, "get_user_subscriptions", ()).await;

  match result {
    Ok((subscriptions,)) => {
      log!("Successfully fetched subscriptions: {:?}", subscriptions);
      subscriptions
    }
    Err(e) => {
      log!("Error fetching subscriptions: {:?}", e);
      vec![]
    }
  }
}

// These methods will only be used in integration tests. They are not part of the public API.
// Note: any change of these methods will require changes in the integration tests.
#[update]
async fn subscribe_test(evm_logs_canister: Principal) {
  let base_swaps_filter = create_base_swaps_config();
  register_subscription_and_map_decoder(evm_logs_canister, base_swaps_filter, swap_event_data_decoder).await;
}

#[query]
fn get_candid_pointer() -> String {
  __export_service()
}

candid::export_service!();
