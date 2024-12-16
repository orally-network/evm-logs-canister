pub mod read_contract;
pub mod utils;
pub mod state;
pub mod decoders;

use ic_cdk::api::call::call;
use ic_cdk_macros::{update, query, init};
use candid::Principal;
use evm_logs_types::{EventNotification, UnsubscribeResult};
use utils::{create_base_swaps_config, create_ethereum_sync_config, register_subscription_and_map_decoder};
use state::NOTIFICATIONS;
use decoders::{ethereum_sync_decoder, swap_event_data_decoder, primex_deposit_decoder, chainfusion_deposit_decoder};
use state::{DECODERS, DECODED_NOTIFICATIONS};
use crate::read_contract::SolidityToken;
use candid::{CandidType, Deserialize};

#[derive(CandidType, Deserialize, Clone)]
struct DecodedNotification {
    notification: EventNotification,
    tokens: Vec<SolidityToken>,
}

#[init]
async fn init() {
    ic_cdk::println!("Test_canister1 initialized");
}

// Candid update methods
#[update]
async fn subscribe(canister_id: Principal) {
    ic_cdk::println!("Starting subscription registration");

    let base_swaps_filter = create_base_swaps_config();
    let eth_sync_filter = create_ethereum_sync_config(); 
    let primex_deposit_filter = create_base_swaps_config();
    let chainfusion_deposit_filter = create_ethereum_sync_config(); 

    register_subscription_and_map_decoder(canister_id, base_swaps_filter, swap_event_data_decoder).await;
    register_subscription_and_map_decoder(canister_id, eth_sync_filter, ethereum_sync_decoder).await;
    register_subscription_and_map_decoder(canister_id, primex_deposit_filter, primex_deposit_decoder).await;
    register_subscription_and_map_decoder(canister_id, chainfusion_deposit_filter, chainfusion_deposit_decoder).await;
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
async fn handle_notification(notification: EventNotification) {
    ic_cdk::println!("Received notification for event ID: {:?}", notification.event_id);
    ic_cdk::println!("Notification details: {:?}", notification);

    NOTIFICATIONS.with(|notifs| {
        notifs.borrow_mut().push(notification.clone());
    });

    DECODERS.with(|decoders| {
        if let Some(decoder) = decoders.borrow().get(&notification.sub_id) {
            match decoder(&notification) {
                Ok(decoded_tokens) => {
                    DECODED_NOTIFICATIONS.with(|decoded| {
                        decoded.borrow_mut().push((notification.clone(), decoded_tokens));
                    });
                }
                Err(e) => {
                    ic_cdk::println!("Error decoding event data: {:?}", e);
                }
            }
        } else {
            ic_cdk::println!("No decoder found for subscription_id: {:?}", notification.sub_id);
        }
    });
}

#[query]
fn get_decoded_notifications() -> Vec<DecodedNotification> {
    DECODED_NOTIFICATIONS.with(|decoded| {
        decoded.borrow().iter().map(|(notif, toks)| {
            DecodedNotification {
                notification: notif.clone(),
                tokens: toks.clone(),
            }
        }).collect()
    })
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
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();
