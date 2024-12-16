
use evm_logs_types::Filter;
use candid::Principal;
use crate::read_contract::SolidityToken;
use evm_logs_types::{SubscriptionRegistration, RegisterSubscriptionResult, EventNotification};
use ic_cdk::api::call::call;
use super::state::DECODERS;


// Helper to register a subscription and store the decoder
pub async fn register_subscription_and_map_decoder(
    canister_id: Principal,
    subscription: SubscriptionRegistration,
    decoder: fn(&EventNotification) -> Result<Vec<SolidityToken>, String>
) {
    ic_cdk::println!("Registering subscription with filter: {:?}", subscription.filter);

    let result: Result<(RegisterSubscriptionResult,), _> = call(
        canister_id,
        "register_subscription",
        (subscription,),
    )
    .await;

    match result {
        Ok((response,)) => {
            match response {
                RegisterSubscriptionResult::Ok(sub_id) => {
                    ic_cdk::println!("Subscription registered successfully with sub_id: {:?}", sub_id);
                    DECODERS.with(|decoders| {
                        decoders.borrow_mut().insert(sub_id.clone(), Box::new(decoder));
                    });
                }
                RegisterSubscriptionResult::Err(err) => {
                    ic_cdk::println!("Error registering subscription: {:?}", err);
                }
            }
        }
        Err(e) => ic_cdk::println!("Error calling canister: {:?}", e),
    }
}


pub fn create_base_swaps_config() -> SubscriptionRegistration {
    // address and topics to monitor
    let address = vec!["0xb2cc224c1c9feE385f8ad6a55b4d94E92359DC59".to_string()];
    let topics = Some(vec![
        vec![
            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67".to_string(),
        ],
    ]);

    let filter = Filter {
        address,
        topics,
    };

    SubscriptionRegistration {
        namespace: "com.events.Base".to_string(),
        filter,
        memo: None,
    }
}

pub fn create_ethereum_sync_config() -> SubscriptionRegistration {
    // address and topics to monitor
    let address = vec!["0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".to_string()];

    let topics = Some(vec![
        vec![
            "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1".to_string(),
        ],
    ]);

    let filter = Filter {
        address,
        topics,
    };

    SubscriptionRegistration {
        namespace: "com.events.Base".to_string(),
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