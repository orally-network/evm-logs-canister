use ic_cdk::api::call::call;
use ic_cdk_macros::{update, query, init};
use candid::Principal;
use std::cell::RefCell;
use evm_logs_types::{SubscriptionRegistration, RegisterSubscriptionResult, EventNotification, UnsubscribeResult};
use ic_web3_rs::ethabi::{decode, ParamType};
use ic_web3_rs::types::H160;
use hex;
use hex::FromHex;

mod read_contract;

use read_contract::{SolidityToken, SwapEventData};

thread_local! {
    static NOTIFICATIONS: RefCell<Vec<EventNotification>> = RefCell::new(Vec::new());
}

#[init]
async fn init() {
    ic_cdk::println!("Test_canister1 initialized");
}

#[update]
async fn register_subscription(canister_id: Principal, registration: SubscriptionRegistration) {
    ic_cdk::println!("Calling register_subscription for namespace - {:?}", registration.namespace);
    ic_cdk::println!(" - {:?}", registration.namespace);

    let result: Result<(RegisterSubscriptionResult,), _> = call(
        canister_id,
        "register_subscription",
        (registration,),
    )
    .await;

    match result {
        Ok((response,)) => {
            ic_cdk::println!("Success: {:?}", response);
        }
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
        notifs.borrow_mut().push(notification);
    });
}

#[query]
fn get_notifications() -> Vec<EventNotification> {
    NOTIFICATIONS.with(|notifs| notifs.borrow().clone())
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

fn decode_event_data(data: Vec<u8>) -> Result<Vec<SolidityToken>, String> {
    // index_topic_1 address sender, index_topic_2 address recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick
    
    let param_types = vec![
        ParamType::Int(256), // amount0
        ParamType::Int(256), // amount1
        ParamType::Int(256), // sqrtPriceX96
        ParamType::Int(256), // liquidity
        ParamType::Int(256),   // tick
    ];

    let decoded_tokens = decode(&param_types, &data)
        .map_err(|e| format!("Decoding error: {:?}", e))?;

    let result = decoded_tokens.into_iter().map(SolidityToken::from).collect();
    Ok(result)
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

            // Decode the data
            let data: Vec<u8> = match hex::decode(&data_str) {
                Ok(bytes) => bytes,
                Err(e) => {
                    ic_cdk::println!("Error decoding data hex string: {:?}", e);
                    continue;
                }
            };

            // Decode event data
            let decoded_data = match decode_event_data(data) {
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
