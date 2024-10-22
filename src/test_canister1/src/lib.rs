use ic_cdk::api::call::call;
use candid::{Principal};
use ic_cdk_macros::{update, query, init};
use std::cell::RefCell;
use evm_logs_types::{SubscriptionRegistration, RegisterSubscriptionResult, EventNotification, UnsubscribeResult, ICRC16Value};
use candid::{CandidType, Deserialize};
use ic_web3_rs::ethabi::{decode, ParamType, Token};
use ic_web3_rs::types::{U256, H160};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct SwapEventData {
    pub sender: SolidityToken,
    pub amount0: SolidityToken,
    pub amount1: SolidityToken,
}

/// SolidityToken is a representation of a web3_rs Token, but with CandidType support
#[derive(CandidType, Deserialize, Clone, Debug)]
pub enum SolidityToken {
    Address(String),
    FixedBytes(ic_web3_rs::ethabi::FixedBytes),
    Bytes(ic_web3_rs::ethabi::Bytes),
    Int(String),
    Uint(String),
    Bool(bool),
    String(String),
    FixedArray(Vec<SolidityToken>),
    Array(Vec<SolidityToken>),
    Tuple(Vec<SolidityToken>),
}

impl From<Token> for SolidityToken {
    fn from(token: Token) -> Self {
        match token {
            Token::Address(address) => SolidityToken::Address(format!("{:?}", address)),
            Token::FixedBytes(bytes) => SolidityToken::FixedBytes(bytes),
            Token::Bytes(bytes) => SolidityToken::Bytes(bytes),
            Token::Int(int) => SolidityToken::Int(format!("{}", int)),
            Token::Uint(uint) => SolidityToken::Uint(format!("{}", uint)),
            Token::Bool(boolean) => SolidityToken::Bool(boolean),
            Token::String(string) => SolidityToken::String(string),
            Token::FixedArray(tokens) => {
                SolidityToken::FixedArray(tokens.into_iter().map(SolidityToken::from).collect())
            }
            Token::Array(tokens) => {
                SolidityToken::Array(tokens.into_iter().map(SolidityToken::from).collect())
            }
            Token::Tuple(tokens) => {
                SolidityToken::Tuple(tokens.into_iter().map(SolidityToken::from).collect())
            }
        }
    }
}

impl From<SolidityToken> for Token {
    fn from(token: SolidityToken) -> Self {
        match token {
            SolidityToken::Address(address) => Token::Address(H160::from_str(&address).unwrap()),
            SolidityToken::FixedBytes(bytes) => Token::FixedBytes(bytes),
            SolidityToken::Bytes(bytes) => Token::Bytes(bytes),
            SolidityToken::Int(int) => Token::Int(U256::from_str_radix(&int, 10).unwrap()),
            SolidityToken::Uint(uint) => Token::Uint(U256::from_str_radix(&uint, 10).unwrap()),
            SolidityToken::Bool(boolean) => Token::Bool(boolean),
            SolidityToken::String(string) => Token::String(string),
            SolidityToken::FixedArray(tokens) => {
                Token::FixedArray(tokens.into_iter().map(Token::from).collect())
            }
            SolidityToken::Array(tokens) => {
                Token::Array(tokens.into_iter().map(Token::from).collect())
            }
            SolidityToken::Tuple(tokens) => {
                Token::Tuple(tokens.into_iter().map(Token::from).collect())
            }
        }
    }
}


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
        ParamType::Address, // sender
        ParamType::Address, // recipient
        ParamType::Int(256), // amount0
        ParamType::Int(256), // amount1
        ParamType::Uint(160), // sqrtPriceX96
        ParamType::Uint(128), // liquidity
        ParamType::Int(24),   // tick
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
            let data: Vec<u8> = notification.data.clone().try_into().unwrap();
            let decoded_data = decode_event_data(data);
            match decoded_data {
                Ok(tokens) => {
                    let event_data = SwapEventData {
                        sender: SolidityToken::from(tokens[0].clone()),
                        amount0: SolidityToken::from(tokens[2].clone()),
                        amount1: SolidityToken::from(tokens[3].clone()),
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
