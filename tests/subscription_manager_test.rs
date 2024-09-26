use pocket_ic::WasmResult;
use pocket_ic::nonblocking::PocketIc;
use candid;
use tokio::time::sleep;
use candid::Principal;
use evm_logs_types::{
    SubscriptionRegistration, RegisterSubscriptionResult, SubscriptionInfo
};
use std::time::Duration;
use std::io::{self, Write};


#[tokio::test]
async fn test_register_subscription() {
    println!("Starting test: test_register_subscription");

    let pic = PocketIc::new().await;

    let canister_id = pic.create_canister().await;
    println!("Canister created, ID: {:?}", canister_id);

    pic.add_cycles(canister_id, 2_000_000_000_000).await;
    println!("Cycles added to the canister");

    // Read the WASM bytes from the path set in the environment variable
    let wasm_path = std::env::var("EVM_LOGS_CANISTER_PATH")
        .expect("EVM_LOGS_CANISTER_PATH must be set");

    let wasm_bytes = tokio::fs::read(wasm_path)
        .await
        .expect("Failed to read the WASM file");

    pic.install_canister(canister_id, wasm_bytes.to_vec(), vec![], None).await;
    println!("Wasm installed in the canister");

    let subscription_params = SubscriptionRegistration {
        namespace: "test_namespace".to_string(),
        config: vec![],
        memo: None,
    };

    // Call the registration function in the canister
    println!("Calling subscription registration");
    let result = pic.update_call(
        canister_id,
        Principal::anonymous(),
        "icrc72_register_subscription",
        candid::encode_one(vec![subscription_params.clone()]).unwrap(),
    ).await;

    // Check the result of the subscription registration
    match result {
        Ok(wasm_result) => {
            match wasm_result {
                WasmResult::Reply(data) => {
                    let decoded_result: Vec<RegisterSubscriptionResult> = candid::decode_one(&data).unwrap();
                    
                    // Verify that the subscription was successfully created
                    match &decoded_result[0] {
                        RegisterSubscriptionResult::Ok(sub_id) => {
                            println!("Subscription successfully created, ID: {:?}", sub_id);

                            // Now query the subscriptions
                            let query_result = pic.query_call(
                                canister_id,
                                Principal::anonymous(),
                                "icrc72_get_subscriptions",
                                candid::encode_args::<(Option<String>, Option<Vec<()>>, Option<u64>, Option<Vec<()>>)>((
                                    Some("test_namespace".to_string()), // namespace
                                    None,                              // prev
                                    None,                              // take
                                    None,                              // stats_filter
                                )).unwrap(),
                            ).await;

                            match query_result {
                                Ok(WasmResult::Reply(data)) => {
                                    // Decode the received data into a list of subscriptions
                                    let subs_info: Vec<SubscriptionInfo> = candid::decode_one(&data).unwrap();
                                    
                                    assert_eq!(subs_info.len(), 1, "Expected one subscription");
                                    assert_eq!(subs_info[0].namespace, "test_namespace", "Incorrect subscription namespace");

                                    println!("Test completed successfully");
                                }
                                Ok(WasmResult::Reject(err)) => {
                                    panic!("Query was rejected: {:?}", err);
                                }
                                Err(e) => {
                                    panic!("Error in subscription query: {:?}", e);
                                }
                            }
                        }
                        RegisterSubscriptionResult::Err(err) => {
                            panic!("Subscription registration error: {:?}", err);
                        }
                    }
                }
                WasmResult::Reject(err) => {
                    panic!("Call was rejected: {:?}", err);
                }
            }
        }
        Err(e) => panic!("Call error: {:?}", e),
    }

    // Reduced sleep time for testing purposes
    sleep(Duration::from_millis(500)).await;
}


// #[tokio::test]
// async fn test_register_subscription() {
//     // Ensure that the log flushes immediately
//     // println!("Starting test: test_register_subscription");
//     // io::stdout().flush().unwrap();
//     let pic = PocketIc::new().await;

//     println!("PocketIc initialized");

//     // Create a canister
//     let canister_id = pic.create_canister().await;
//     println!("Canister created, ID: {:?}", canister_id);

//     pic.add_cycles(canister_id, 2_000_000_000_000).await;
//     println!("Cycles added to the canister");

//     // You can now continue with the wasm installation or other operations
//     // let wasm_bytes = include_bytes!("/path/to/evm_logs_canister.wasm");
//     // pic.install_canister(canister_id, wasm_bytes.to_vec(), vec![], None).await;

//     println!("Test completed successfully");
// }
