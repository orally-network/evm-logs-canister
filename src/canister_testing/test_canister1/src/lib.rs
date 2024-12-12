pub mod read_contract;
pub mod utils;
pub mod state;
pub mod decoders;

use ic_cdk::api::call::call;
use ic_cdk_macros::{update, query, init};
use candid::Principal;
use evm_logs_types::{EventNotification, UnsubscribeResult};
use ic_web3_rs::types::H160;
use hex;
use hex::FromHex;
use read_contract::{SolidityToken, SwapEventData};
use utils::{create_base_swaps_config, create_ethereum_sync_config, register_subscription_and_map_decoder};
use state::NOTIFICATIONS;
use decoders::decode_swap_event_data;
use state::DECODERS;

#[init]
async fn init() {
    ic_cdk::println!("Test_canister1 initialized");
}

// Candid update methods
#[update]
async fn register_subscription(canister_id: Principal) {
    ic_cdk::println!("Starting subscription registration");

    let base_swaps_filter = create_base_swaps_config();
    let eth_sync_filter = create_ethereum_sync_config(); 

    register_subscription_and_map_decoder(canister_id, base_swaps_filter, decode_swap_event_data).await;
    register_subscription_and_map_decoder(canister_id, eth_sync_filter, decode_swap_event_data).await;
}

#[update]
async fn unsubscribe(canister_id: Principal, subscription_id: candid::Nat) {
    ic_cdk::println!("Calling unsubscribe for subscription ID: {:?}", subscription_id);

    let result: Result<(evm_logs_types::UnsubscribeResult,), _> = call(
        canister_id,
        "unsubscribe",
        (subscription_id.clone(),),
    )
    .await;
    // TODO remove decoder
    match result {
        Ok((response,)) => match response {
            UnsubscribeResult::Ok() => ic_cdk::println!("Successfully unsubscribed from {:?}", subscription_id),
            UnsubscribeResult::Err(err) => ic_cdk::println!("Error unsubscribing: {:?}", err),
        },
        Err(e) => {
            ic_cdk::println!("Error calling canister: {:?}", e);
        }
    }
}

#[update]
async fn icrc72_handle_notification(notification: EventNotification) {
    ic_cdk::println!("Received notification for event ID: {:?}", notification.event_id);
    ic_cdk::println!("Notification details: {:?}", notification);

    NOTIFICATIONS.with(|notifs| {
        notifs.borrow_mut().push(notification.clone());
    });

    let maybe_decoded = DECODERS.with(|decoders| {
        if let Some(decoder) = decoders.borrow().get(&notification.sub_id) {

            // Convert notification.data to String, removing "0x" prefix if present
            let data_str = match String::try_from(notification.data.clone()) {
                Ok(s) => s.trim_start_matches("0x").to_string(),
                Err(e) => {
                    ic_cdk::println!("Error converting notification data to String: {:?}", e);
                    return None;
                }
            };

            // hex => bytes
            let data = match hex::decode(&data_str) {
                Ok(bytes) => bytes,
                Err(e) => {
                    ic_cdk::println!("Error decoding data hex string: {:?}", e);
                    return None;
                }
            };

            // Decode event data
            match decoder(data) {
                Ok(decoded_tokens) => Some(decoded_tokens),
                Err(e) => {
                    ic_cdk::println!("Error decoding event data: {:?}", e);
                    None
                }
            }
        }
        else {
            ic_cdk::println!("No decoder found for subscription_id: {:?}", notification.sub_id);
            None
        }

        // if let Some(decoded_tokens) = maybe_decoded {
        //     DECODED_NOTIFICATIONS.with(|decoded| {
        //         decoded.borrow_mut().push((notification, decoded_tokens));
        //     });
        // }
        
    });
    // Decode each notification depending on the sub_id(map decoder to each event)
}

#[query]
fn get_notifications() -> Vec<EventNotification> {
    NOTIFICATIONS.with(|notifs| notifs.borrow().clone())
}

#[update]
async fn get_subscriptions(canister_id: Principal) -> Vec<evm_logs_types::SubscriptionInfo> {
    ic_cdk::println!("Calling get_subscriptions");

    let result: Result<(Vec<evm_logs_types::SubscriptionInfo>,), _> = call(
        canister_id,
        "get_user_subscriptions",
        ()
    )
    .await;

    match result {
        Ok((subscriptions,)) => {
            ic_cdk::println!("Successfully fetched subscriptions: {:?}", subscriptions);
            subscriptions
        },
        Err(e) => {
            ic_cdk::println!("Error fetching subscriptions: {:?}", e);
            vec![] 
        }
    }
}


#[query]
fn get_swap_events_data() -> Vec<SwapEventData> {
    NOTIFICATIONS.with(|notifications| {
        let mut result = Vec::new();
        for notification in notifications.borrow().iter() {
            // Convert notification.data to String, removing "0x" prefix if present
            let data_str = match String::try_from(notification.data.clone()) {
                Ok(s) => s.trim_start_matches("0x").to_string(),
                Err(e) => {
                    ic_cdk::println!("Error converting notification data to String: {:?}", e);
                    continue;
                }
            };

            let data: Vec<u8> = match hex::decode(&data_str) {
                Ok(bytes) => bytes,
                Err(e) => {
                    ic_cdk::println!("Error decoding data hex string: {:?}", e);
                    continue;
                }
            };

            // Decode event data
            let decoded_data = match decode_swap_event_data(data) {
                Ok(tokens) => tokens,
                Err(e) => {
                    ic_cdk::println!("Error decoding event data: {:?}", e);
                    continue;
                }
            };

            // Extract sender and receiver from notification.topics
            if notification.topics.len() >= 3 {
                // topics[0] is event signature hash
                let sender_topic = &notification.topics[1];
                let receiver_topic = &notification.topics[2];

                // Remove "0x" prefix if present
                let sender_hex = sender_topic.trim_start_matches("0x");
                let receiver_hex = receiver_topic.trim_start_matches("0x");

                // Convert the hex strings to bytes
                let sender_bytes = match Vec::from_hex(sender_hex) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        ic_cdk::println!("Error decoding sender hex string: {:?}", e);
                        continue;
                    }
                };
                let receiver_bytes = match Vec::from_hex(receiver_hex) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        ic_cdk::println!("Error decoding receiver hex string: {:?}", e);
                        continue;
                    }
                };

                // Ensure the bytes are 32 bytes long
                if sender_bytes.len() != 32 || receiver_bytes.len() != 32 {
                    ic_cdk::println!("Invalid topic length for sender or receiver");
                    continue;
                }

                // Take the last 20 bytes to get the address
                let sender_address_bytes = &sender_bytes[12..32];
                let receiver_address_bytes = &receiver_bytes[12..32];

                // Create H160 addresses from bytes
                let sender_address = H160::from_slice(sender_address_bytes);
                let receiver_address = H160::from_slice(receiver_address_bytes);

                // Convert addresses to strings with "0x" prefix
                let sender_address_hex = format!("0x{:x}", sender_address);
                let receiver_address_hex = format!("0x{:x}", receiver_address);

                let sender = SolidityToken::Address(sender_address_hex.clone());
                let receiver = SolidityToken::Address(receiver_address_hex.clone());

                // Construct SwapEventData
                let event_data = SwapEventData {
                    tx_hash: notification.tx_hash.clone(),
                    sender,
                    receiver,
                    amount0: decoded_data[0].clone(),
                    amount1: decoded_data[1].clone(),
                    sqrt_price_x96: decoded_data[2].clone(),
                    liquidity: decoded_data[3].clone(),
                    tick: decoded_data[4].clone(),
                };

                result.push(event_data);
            } else {
                ic_cdk::println!("Not enough topics to extract sender and receiver");
                // Handle the case where topics are insufficient
                continue;
            }
        }
        result
    })
}


#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();
