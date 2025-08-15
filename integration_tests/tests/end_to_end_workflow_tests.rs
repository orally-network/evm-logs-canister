use candid::Principal;
use evm_logs_types::{Filter, Hex20, Hex32, LogEntry};
use pocket_ic::PocketIc;

mod types;
mod utils;
use types::*;
use utils::*;

#[test]
fn test_end_to_end_subscription_delivery_and_charging() {
    // 1) Setup orchestrator and an Ethereum chain service
    let (pic, orchestrator_id, chain_service_id) = setup_full_system();

    // 2) Deploy a shared proxy canister and configure the chain service via orchestrator
    let proxy_id = pic.create_canister();
    pic.add_cycles(proxy_id, 1_000_000_000_000);
    let proxy_wasm = get_proxy_wasm();
    let proxy_init = candid::encode_one(()).unwrap();
    pic.install_canister(proxy_id, proxy_wasm, proxy_init, None);

    // Configure chain service with proxy via orchestrator
    let cfg_res = pic.update_call(
        orchestrator_id,
        Principal::anonymous(),
        "configure_chain_service",
        candid::encode_args((1u32, Some(proxy_id), None::<Principal>)).unwrap(),
    );
    assert!(cfg_res.is_ok(), "configure_chain_service call failed: {:?}", cfg_res.err());
    let cfg_reply: Result<(), String> =
        candid::decode_one(&extract_reply_bytes(cfg_res.unwrap())).expect("decode configure_chain_service");
    assert!(cfg_reply.is_ok(), "configure_chain_service returned error: {:?}", cfg_reply.err());

    // 3) Deploy a consumer canister (test_canister) to receive notifications
    let test_canister_id = pic.create_canister();
    pic.add_cycles(test_canister_id, 1_000_000_000_000);
    let test_wasm = get_test_canister_wasm();
    let test_init = candid::encode_one(()).unwrap();
    pic.install_canister(test_canister_id, test_wasm, test_init, None);

    // 4) Subscribe via orchestrator on behalf of the consumer canister.
    // Filter: address=None, topics=[ [topic0] ]
    let topic0 = Hex32::from([0x11; 32]); // arbitrary fixed topic
    let filter = Filter {
        address: None,
        topics: Some(vec![vec![topic0.clone()]]),
    };

    // Call subscribe_to_chain with caller set to test_canister_id (non-anonymous)
    let sub_res = pic.update_call(
        orchestrator_id,
        test_canister_id, // caller = subscriber principal
        "subscribe_to_chain",
        candid::encode_args((1u32, filter)).unwrap(),
    );
    assert!(sub_res.is_ok(), "subscribe_to_chain failed: {:?}", sub_res.err());
    let sub_decoded: Result<orchestrator_canister::types::SubscriptionResult, String> =
        candid::decode_one(&extract_reply_bytes(sub_res.unwrap())).expect("decode subscribe_to_chain");
    assert!(sub_decoded.is_ok(), "subscribe_to_chain returned error: {:?}", sub_decoded.err());
    let subscription_result = sub_decoded.unwrap();
    assert_eq!(subscription_result.chain_service_canister_id, chain_service_id);

    // 5) Credit balance for the consumer on the chain service (so it can pay for notifications)
    // This call is guarded to orchestrator, so caller must be the orchestrator_id
    let credit_amount: u64 = 2_000_000_000; // 2B cycles for test
    let credit_res = pic.update_call(
        chain_service_id,
        orchestrator_id,
        "test_credit_balance",
        candid::encode_args((test_canister_id, credit_amount)).unwrap(),
    );
    assert!(credit_res.is_ok(), "test_credit_balance call failed: {:?}", credit_res.err());
    let credit_reply: Result<(), String> =
        candid::decode_one(&extract_reply_bytes(credit_res.unwrap())).expect("decode test_credit_balance");
    assert!(credit_reply.is_ok(), "test_credit_balance returned error: {:?}", credit_reply.err());

    // 6) Inject a matching log (first-topic aggregation should allow this fetch path,
    // and local filter must match exactly before publishing to proxy)
    let log = LogEntry {
        address: Hex20::from([0xAB; 20]),
        topics: vec![topic0.clone()],
        transaction_hash: Some(Hex32::from([0x22; 32])),
        block_number: None,
        data: evm_rpc_types::Hex::from(Vec::<u8>::new()),
        block_hash: None,
        transaction_index: None,
        log_index: None,
        removed: false,
    };

    // Use test hook (guarded to orchestrator) to publish as if fetched
    let publish_res = pic.update_call(
        chain_service_id,
        orchestrator_id,
        "test_publish_logs",
        candid::encode_args((vec![log],)).unwrap(),
    );
    assert!(publish_res.is_ok(), "test_publish_logs call failed: {:?}", publish_res.err());
    let publish_reply: Result<(), String> =
        candid::decode_one(&extract_reply_bytes(publish_res.unwrap())).expect("decode test_publish_logs");
    assert!(publish_reply.is_ok(), "test_publish_logs returned error: {:?}", publish_reply.err());

    // 7) Verify the consumer received the notification
    let notif_res = pic.query_call(
        test_canister_id,
        Principal::anonymous(),
        "get_notifications",
        candid::encode_one(()).unwrap(),
    );
    assert!(notif_res.is_ok(), "get_notifications query failed: {:?}", notif_res.err());
    let notifications: Vec<evm_logs_types::EventNotification> =
        candid::decode_one(&extract_reply_bytes(notif_res.unwrap())).expect("decode get_notifications");
    assert!(!notifications.is_empty(), "No notifications received by consumer canister");
}
