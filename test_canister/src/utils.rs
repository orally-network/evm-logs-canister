use std::str::FromStr;

use candid::Principal;
use evm_logs_types::{EventNotification, Filter, RegisterSubscriptionResult, SubscriptionRegistration};
use evm_rpc_types::{Hex20, Hex32};
use ic_cdk::api::call::call_with_payment;

use super::state::DECODERS;
use crate::{debug_log, read_contract::SolidityToken};

// Helper to register a subscription and store the decoder
pub async fn register_subscription_and_map_decoder(
  canister_id: Principal,
  subscription: SubscriptionRegistration,
  decoder: fn(&EventNotification) -> Result<Vec<SolidityToken>, String>,
) {
  debug_log!("Registering subscription with filter: {:?}", subscription.filter);

  let result: Result<(RegisterSubscriptionResult,), _> =
    call_with_payment(canister_id, "subscribe", (subscription,), 10_000_000_000).await;

  match result {
    Ok((response,)) => match response {
      RegisterSubscriptionResult::Ok(sub_id) => {
        debug_log!("Subscription registered successfully with sub_id: {:?}", sub_id);
        DECODERS.with(|decoders| {
          decoders.borrow_mut().insert(sub_id.clone(), Box::new(decoder));
        });
      }
      RegisterSubscriptionResult::Err(err) => {
        debug_log!("Error registering subscription: {:?}", err);
      }
    },
    Err(e) => debug_log!("Error calling canister: {:?}", e),
  }
}

pub fn create_base_swaps_config() -> SubscriptionRegistration {
  // address and topics to monitor
  let address = Hex20::from_str("0xb2cc224c1c9feE385f8ad6a55b4d94E92359DC59").unwrap();
  let topics = Some(vec![vec![
    Hex32::from_str("0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67").unwrap(),
  ]]);

  let filter = Filter {
    address: Hex20::from(address),
    topics,
  };

  SubscriptionRegistration {
    chain_id: 8453,
    filter,
    memo: None,
    canister_to_top_up: ic_cdk::id(),
  }
}

pub fn create_ethereum_sync_config() -> SubscriptionRegistration {
  // address and topics to monitor
  let address = Hex20::from_str("0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852").unwrap();
  let topics = Some(vec![vec![
    Hex32::from_str("0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1").unwrap(),
  ]]);

  let filter = Filter { address, topics };

  SubscriptionRegistration {
    chain_id: 1,
    filter,
    memo: None,
    canister_to_top_up: ic_cdk::id(),
  }
}

pub fn create_primex_deposit_config() -> SubscriptionRegistration {
  // address and topics to monitor

  let address = Hex20::from_str("0x12c125181Eb7c944EaEfcB2AE881475870f0Aff3").unwrap();
  let topics = Some(vec![vec![
    Hex32::from_str("0x5548c837ab068cf56a2c2479df0882a4922fd203edb7517321831d95078c5f62").unwrap(),
  ]]);

  let filter = Filter { address, topics };

  SubscriptionRegistration {
    chain_id: 137,
    filter,
    memo: None,
    canister_to_top_up: ic_cdk::id(),
  }
}

pub fn create_chainfusion_deposit_config() -> SubscriptionRegistration {
  let address = Hex20::from_str("0x7574eb42ca208a4f6960eccafdf186d627dcc175").unwrap();
  let topics = Some(vec![vec![
    Hex32::from_str("0x257e057bb61920d8d0ed2cb7b720ac7f9c513cd1110bc9fa543079154f45f435").unwrap(),
  ]]);

  let filter = Filter { address, topics };

  SubscriptionRegistration {
    chain_id: 1,
    filter,
    memo: None,
    canister_to_top_up: ic_cdk::id(),
  }
}

pub fn create_curve_token_exchange_config() -> SubscriptionRegistration {
  // address and topics to monitor
  let address = Hex20::from_str("0x92215849c439E1f8612b6646060B4E3E5ef822cC").unwrap();
  let topics = Some(vec![vec![
    Hex32::from_str("0xb2e76ae99761dc136e598d4a629bb347eccb9532a5f8bbd72e18467c3c34cc98").unwrap(),
  ]]);

  let filter = Filter { address, topics };

  SubscriptionRegistration {
    chain_id: 137,
    filter,
    memo: None,
    canister_to_top_up: ic_cdk::id(),
  }
}
/// Extracts and decodes the event data bytes from the notification.
/// This function converts the event's data from a hex string to raw bytes.
/// Returns an error if any step of the conversion fails.
pub fn extract_data_bytes(notification: &EventNotification) -> Result<Vec<u8>, String> {
  // Convert the notification data into a hex string without the "0x" prefix
  let data_str = match String::try_from(notification.log_entry.data.clone()) {
    Ok(s) => s.trim_start_matches("0x").to_string(),
    Err(e) => {
      return Err(format!("Error converting notification data to String: {:?}", e));
    }
  };

  // Decode the hex string into bytes
  let data = match hex::decode(&data_str) {
    Ok(bytes) => bytes,
    Err(e) => {
      return Err(format!("Error decoding data hex string: {:?}", e));
    }
  };

  Ok(data)
}
