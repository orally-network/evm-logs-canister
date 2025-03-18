pub mod decoders;
pub mod read_contract;
pub mod state;
pub mod utils;

use candid::{CandidType, Deserialize, Nat, Principal};
use decoders::{chainfusion_deposit_decoder, ethereum_sync_decoder, primex_deposit_decoder, swap_event_data_decoder};
use evm_logs_types::{EventNotification, RegisterSubscriptionResult, TopUpBalanceResult, UnsubscribeResult};
use ic_cdk::api::{
  call::{call, call_with_payment, call_with_payment128},
  canister_balance128,
};
use ic_cdk_macros::{init, query, update};
use ic_utils::{
  api_type::{GetInformationRequest, GetInformationResponse, UpdateInformationRequest},
  get_information, update_information,
};
use state::{DECODED_NOTIFICATIONS, DECODERS, NOTIFICATIONS};
use utils::*;

use crate::{
  decoders::{mainnet_fantom_token, mainnet_uniswap_exchange_1},
  read_contract::SolidityToken,
};

#[derive(CandidType, Deserialize, Clone)]
struct DecodedNotification {
  notification: EventNotification,
  tokens: Vec<SolidityToken>,
}

#[init]
async fn init() {
  log_with_metrics!("Test Canister Initialized!");
}

// Candid update methods
#[update]
async fn subscribe(evm_logs_canister: Principal) {
  log_with_metrics!("Starting subscription registration");

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
async fn subscribe_base_swaps(evm_logs_canister: Principal) {
  log_with_metrics!("Starting subscription registration (base swaps)");
  let base_swaps_filter = create_base_swaps_config();
  register_subscription_and_map_decoder(evm_logs_canister, base_swaps_filter, swap_event_data_decoder).await;
}

#[update]
async fn subscribe_etherum_sync(evm_logs_canister: Principal) {
  log_with_metrics!("Starting subscription registration (etherum sync)");
  let eth_sync_filter = create_ethereum_sync_config();
  register_subscription_and_map_decoder(evm_logs_canister, eth_sync_filter, ethereum_sync_decoder).await;
}

#[update]
async fn subscribe_primex(evm_logs_canister: Principal) {
  log_with_metrics!("Starting subscription registration (primex)");
  let primex_deposit_filter = create_primex_deposit_config();
  register_subscription_and_map_decoder(evm_logs_canister, primex_deposit_filter, primex_deposit_decoder).await;
}

#[update]
async fn subscribe_chainfusion(evm_logs_canister: Principal) {
  log_with_metrics!("Starting subscription registration (chainfusion)");
  let chainfusion_deposit_filter = create_chainfusion_deposit_config();
  register_subscription_and_map_decoder(
    evm_logs_canister,
    chainfusion_deposit_filter,
    chainfusion_deposit_decoder,
  )
  .await;
}

#[update]
async fn subscribe_token_exchange(evm_logs_canister: Principal) {
  log_with_metrics!("Starting subscription registration (token exchange)");
  let curve_token_exchange_config = create_curve_token_exchange_config();
  register_subscription_and_map_decoder(
    evm_logs_canister,
    curve_token_exchange_config,
    chainfusion_deposit_decoder,
  )
  .await;
}

#[update]
async fn subscribe_uniswap_exchange_1(evm_logs_canister: Principal) {
  log_with_metrics!("Starting subscription registration (uniswap_exchange_1)");
  let uniswap_exchange_1_config = create_uniswap_exchange_1_config();
  register_subscription_and_map_decoder(evm_logs_canister, uniswap_exchange_1_config, mainnet_uniswap_exchange_1).await;
}

#[update]
async fn subscribe_fantom_token(evm_logs_canister: Principal) {
  log_with_metrics!("Starting subscription registration (fantom token)");
  let fantom_token_config = create_fantom_token_config();
  register_subscription_and_map_decoder(evm_logs_canister, fantom_token_config, mainnet_fantom_token).await;
}

#[update]
async fn unsubscribe(canister_id: Principal, subscription_id: candid::Nat) {
  log_with_metrics!("Calling unsubscribe for subscription ID: {:?}", subscription_id);

  let result: Result<(evm_logs_types::UnsubscribeResult,), _> =
    call(canister_id, "unsubscribe", (subscription_id.clone(),)).await;
  // TODO remove corresponding decoder
  match result {
    Ok((response,)) => match response {
      UnsubscribeResult::Ok() => {
        log_with_metrics!("Successfully unsubscribed from {:?}", subscription_id)
      }
      UnsubscribeResult::Err(err) => log_with_metrics!("Error unsubscribing: {:?}", err),
    },
    Err(e) => {
      log_with_metrics!("Error calling canister: {:?}", e);
    }
  }
}

#[update]
async fn handle_notification(notification: EventNotification) {
  log_with_metrics!("Received notification for event ID: {:?}", notification.event_id);
  log_with_metrics!("Notification details: {:?}", notification);

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
          log_with_metrics!("Error decoding event data: {:?}", e);
        }
      }
    } else {
      log_with_metrics!("No decoder found for subscription_id: {:?}", notification.sub_id);
    }
  });
}

#[update]
async fn top_up_evm_logs_canister(evm_logs_canister: Principal, cycles: u128) {
  log_with_metrics!("Topping up balance of evm canister '{evm_logs_canister}' with {cycles} cycles");
  let can_id = ic_cdk::api::id();
  let result: Result<(TopUpBalanceResult,), _> =
    call_with_payment128(evm_logs_canister, "top_up_balance", (can_id,), cycles).await;
  log_with_metrics!("Result of Topping up balance ['{evm_logs_canister}', {cycles} cycles], result: {result:?}");
}

#[query]
fn get_current_balance() -> u128 {
  let balance = canister_balance128();
  log_with_metrics!("Current balance: {balance} cycles");
  balance
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
  log_with_metrics!("Calling get_subscriptions");

  let result: Result<(Vec<evm_logs_types::SubscriptionInfo>,), _> =
    call(canister_id, "get_user_subscriptions", ()).await;

  match result {
    Ok((subscriptions,)) => {
      log_with_metrics!("Successfully fetched subscriptions: {:?}", subscriptions);
      subscriptions
    }
    Err(e) => {
      log_with_metrics!("Error fetching subscriptions: {:?}", e);
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

#[query(name = "getCanistergeekInformation")]
pub async fn get_canistergeek_information(request: GetInformationRequest) -> GetInformationResponse<'static> {
  get_information(request)
}

#[update(name = "updateCanistergeekInformation")]
pub async fn update_canistergeek_information(request: UpdateInformationRequest) {
  update_information(request);
}

ic_cdk::export_candid!();
