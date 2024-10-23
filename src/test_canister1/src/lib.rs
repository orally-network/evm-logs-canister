use ic_cdk::api::call::call;
use ic_cdk_macros::{update, query, init};
use candid::Principal;
use std::cell::RefCell;
use evm_logs_types::{SubscriptionRegistration, RegisterSubscriptionResult, EventNotification, UnsubscribeResult};
use ic_web3_rs::ethabi::{decode, ParamType};
use std::convert::TryInto;
use hex;

mod read_contract;

use read_contract::{SolidityToken, SwapEventData};

const DECIMALS: i128 = 10i128.pow(18); // Constant representing 10^18

thread_local! {
    static NOTIFICATIONS: RefCell<Vec<EventNotification>> = RefCell::new(Vec::new());
}

#[init]
async fn init() {
    ic_cdk::println!("Test_canister1 initialized");
}

#[update]
async fn register_subscription(canister_id: Principal, registrations: Vec<SubscriptionRegistration>) {
    ic_cdk::println!("Calling register_subscription for namespaces:");
    for reg in &registrations {
        ic_cdk::println!(" - {:?}", reg.namespace);
    }

    let result: Result<(Vec<RegisterSubscriptionResult>,), _> = call(
        canister_id,
        "register_subscription",
        (registrations,),
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
            let data: Vec<u8> = hex::decode(String::try_from(notification.data.clone()).unwrap()).unwrap();
            let decoded_data = decode_event_data(data);
            match decoded_data {
                Ok(tokens) => {
                    let event_data = SwapEventData {
                        tx_hash: notification.tx_hash.clone(),
                        amount0: tokens[0].clone(), 
                        amount1: tokens[1].clone(), 
                        sqrtPriceX96: tokens[2].clone(),
                        liquidity: tokens[3].clone(),
                        tick: tokens[4].clone(),

                    };

                    result.push(event_data);
                }
                Err(e) => {
                    ic_cdk::println!("Error decoding event data: {:?}", e);
                }
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
