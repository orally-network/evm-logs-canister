use super::state::DECODERS;
use crate::read_contract::SolidityToken;
use candid::Principal;
use evm_logs_types::Filter;
use evm_logs_types::{EventNotification, RegisterSubscriptionResult, SubscriptionRegistration};
use ic_cdk::api::call::call;
use crate::log;

// Helper to register a subscription and store the decoder
pub async fn register_subscription_and_map_decoder(
    canister_id: Principal,
    subscription: SubscriptionRegistration,
    decoder: fn(&EventNotification) -> Result<Vec<SolidityToken>, String>,
) {
    log!(
        "Registering subscription with filter: {:?}",
        subscription.filter
    );

    let result: Result<(RegisterSubscriptionResult,), _> =
        call(canister_id, "subscribe", (subscription,)).await;

    match result {
        Ok((response,)) => match response {
            RegisterSubscriptionResult::Ok(sub_id) => {
                log!(
                    "Subscription registered successfully with sub_id: {:?}",
                    sub_id
                );
                DECODERS.with(|decoders| {
                    decoders
                        .borrow_mut()
                        .insert(sub_id.clone(), Box::new(decoder));
                });
            }
            RegisterSubscriptionResult::Err(err) => {
                log!("Error registering subscription: {:?}", err);
            }
        },
        Err(e) => log!("Error calling canister: {:?}", e),
    }
}

pub fn create_base_swaps_config() -> SubscriptionRegistration {
    // address and topics to monitor
    let address = "0xb2cc224c1c9feE385f8ad6a55b4d94E92359DC59".to_string();
    let topics = Some(vec![vec![
        "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67".to_string(),
    ]]);

    let filter = Filter { address, topics };

    SubscriptionRegistration {
        chain_id: 8453,
        filter,
        memo: None,
    }
}

pub fn create_ethereum_sync_config() -> SubscriptionRegistration {
    // address and topics to monitor
    let address = "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".to_string();

    let topics = Some(vec![vec![
        "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1".to_string(),
    ]]);

    let filter = Filter { address, topics };

    SubscriptionRegistration {
        chain_id: 1,
        filter,
        memo: None,
    }
}

pub fn create_primex_deposit_config() -> SubscriptionRegistration {
    // address and topics to monitor
    let address = "0x12c125181Eb7c944EaEfcB2AE881475870f0Aff3".to_string();

    let topics = Some(vec![vec![
        "0x5548c837ab068cf56a2c2479df0882a4922fd203edb7517321831d95078c5f62".to_string(),
    ]]);

    let filter = Filter { address, topics };

    SubscriptionRegistration {
        chain_id: 137,
        filter,
        memo: None,
    }
}

pub fn create_chainfusion_deposit_config() -> SubscriptionRegistration {
    let address = "0x7574eb42ca208a4f6960eccafdf186d627dcc175".to_string();
    let topics = Some(vec![vec![
        "0x257e057bb61920d8d0ed2cb7b720ac7f9c513cd1110bc9fa543079154f45f435".to_string(),
    ]]);

    let filter = Filter { address, topics };

    SubscriptionRegistration {
        chain_id: 1,
        filter,
        memo: None,
    }
}

pub fn create_curve_token_exchange_config() -> SubscriptionRegistration {
    // address and topics to monitor
    let address = "0x92215849c439E1f8612b6646060B4E3E5ef822cC".to_string();
    let topics = Some(vec![vec![
        "0xb2e76ae99761dc136e598d4a629bb347eccb9532a5f8bbd72e18467c3c34cc98".to_string(),
    ]]);

    let filter = Filter { address, topics };

    SubscriptionRegistration {
        chain_id: 137,
        filter,
        memo: None,
    }
}
/// Extracts and decodes the event data bytes from the notification.
/// This function converts the event's data from a hex string to raw bytes.
/// Returns an error if any step of the conversion fails.
pub fn extract_data_bytes(notification: &EventNotification) -> Result<Vec<u8>, String> {
    // Convert the notification data into a hex string without the "0x" prefix
    let data_str = match String::try_from(notification.data.clone()) {
        Ok(s) => s.trim_start_matches("0x").to_string(),
        Err(e) => {
            return Err(format!(
                "Error converting notification data to String: {:?}",
                e
            ));
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
