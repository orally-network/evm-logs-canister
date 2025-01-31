use candid::{CandidType, Nat, Principal};
use evm_logs_types::{Filter, SubscriptionRegistration, SubscriptionInfo};
use pocket_ic::nonblocking::PocketIc;
use pocket_ic::WasmResult;
use rand::{distr::Alphanumeric, Rng};
use serde::Deserialize;

#[derive(CandidType, Deserialize)]
struct EvmLogsInitArgs {
    evm_rpc_canister: Principal,
    proxy_canister: Principal,
    pub estimate_events_num: u32,
}

#[derive(CandidType, Deserialize)]
struct WalletCall128Args {
    canister: Principal,
    method_name: String,
    args: Vec<u8>,
    cycles: Nat,
}

#[tokio::test]
async fn test_main_worflow_with_bunch_subscribers() {
    let pic = PocketIc::new().await;
    let num_subscribers = 5;

    // initialize and install proxy canister
    let proxy_canister_id = pic.create_canister().await;
    pic.add_cycles(proxy_canister_id, 4_000_000_000_000).await;

    let proxy_wasm_bytes = tokio::fs::read(std::env::var("PROXY_CANISTER_WASM_PATH").unwrap()).await.unwrap();
    pic.install_canister(proxy_canister_id, proxy_wasm_bytes, vec![], None).await;

    // initialize and install evm-logs-canister
    let evm_logs_canister_id = pic.create_canister().await;
    pic.add_cycles(evm_logs_canister_id, 4_000_000_000_000).await;

    let evm_logs_wasm_bytes = tokio::fs::read(std::env::var("EVM_LOGS_CANISTER_PATH").unwrap()).await.unwrap();

    let init_args = candid::encode_args((EvmLogsInitArgs {
        evm_rpc_canister: Principal::from_text("aaaaa-aa").unwrap(),
        proxy_canister: proxy_canister_id,
        estimate_events_num: 5,
    },)).unwrap();
    pic.install_canister(evm_logs_canister_id, evm_logs_wasm_bytes, init_args, None).await;

    // initialize and install cycles-wallet, for calling evm-logs-canister with payment from different subscribers
    let cycles_wallet_id = pic.create_canister().await;
    pic.add_cycles(cycles_wallet_id, 4_000_000_000_000).await;
    
    let cycles_wallet_wasm_bytes = tokio::fs::read(std::env::var("CYCLES_WALLET_WASM_PATH").unwrap()).await.unwrap();
    pic.install_canister(cycles_wallet_id, cycles_wallet_wasm_bytes, vec![], None).await;

    // all subscribers will have the same WASM file  
    let subscriber_wasm_bytes = tokio::fs::read(std::env::var("TEST_CANISTER_WASM_PATH").unwrap()).await.unwrap();
    let mut subscriber_canisters = Vec::new();

    // create subscribers caisters 
    for _ in 0..num_subscribers {
        let subscriber_canister_id = pic.create_canister().await;
        pic.add_cycles(subscriber_canister_id, 4_000_000_000_000).await;
        pic.install_canister(subscriber_canister_id, subscriber_wasm_bytes.clone(), vec![], None).await;
        subscriber_canisters.push(subscriber_canister_id);
    }

    // subscribe on evm-logs-canister from each subscriber with random topic 
    for subscriber_canister_id in subscriber_canisters {
        let random_topic = format!("0x{}", rand::rng().sample_iter(&Alphanumeric).take(64).map(char::from).collect::<String>());

        let sub_registration = SubscriptionRegistration {
            chain_id: 8453,
            filter: Filter {
                address: "0xb2cc224c1c9feE385f8ad6a55b4d94E92359DC59".to_string(),
                topics: Some(vec![vec![random_topic]]),
            },
            memo: None,
            canister_to_top_up: subscriber_canister_id,
        };

        let sub_reg_encoded = candid::encode_args((sub_registration,)).unwrap();

        let call_args = WalletCall128Args {
            canister: evm_logs_canister_id,
            method_name: "subscribe".to_string(),
            args: sub_reg_encoded,
            cycles: 2_000_000_000_000u128.into(),
        };

        let bytes = candid::encode_args((call_args,)).unwrap();

        match pic.update_call(cycles_wallet_id, Principal::anonymous(), "wallet_call128", bytes).await {
            Ok(WasmResult::Reply(data)) => ic_cdk::println!("Subscription successful: {:?}", data),

            Ok(WasmResult::Reject(err)) => panic!("Subscription rejected: {:?}", err),

            Err(e) => panic!("Subscription call error: {:?}", e),
        }

    }

    // Verify subscription count
    let sub_info_bytes = pic.query_call(
        evm_logs_canister_id, 
        Principal::anonymous(), 
        "get_subscriptions", 
        candid::encode_args(
            (None::<i32>, None::<Nat>, None::<Vec<Filter>>)
        ).unwrap()
    ).await
    .unwrap();

    if let WasmResult::Reply(data) = sub_info_bytes {
        let subscriptions: Vec<SubscriptionInfo> = candid::decode_one(&data).unwrap();

        assert_eq!(subscriptions.len(), num_subscribers, "Subscription count mismatch");

    } else {
        panic!("Failed to get subscriptions");
    }

}
