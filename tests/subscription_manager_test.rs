// use pocket_ic::WasmResult;
// use pocket_ic::nonblocking::PocketIc;
// use candid;
// use tokio::time::sleep;
// use candid::Principal;
// use evm_logs_types::{
//     SubscriptionRegistration, ICRC16Map, Event, EventNotification, ICRC16Value, 
//     RegisterSubscriptionResult, RegisterPublicationResult, PublicationRegistration
// };
// use candid::CandidType;
// use std::time::Duration;
// use candid::Nat;
// use std::collections::HashSet;
// use serde::{Deserialize, Serialize};


// #[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
// pub struct Filter {
//     pub addresses: Vec<String>,
//     pub topics: Option<Vec<Vec<String>>>,
// }


// #[tokio::test]
// async fn test_register_subscription_with_two_filters() {
//     println!("Starting test: test_register_subscription_with_two_filters");

//     let pic = PocketIc::new().await;

//     // Create the first canister
//     let canister_id_1 = pic.create_canister().await;
//     println!("First canister created, ID: {:?}", canister_id_1);

//     pic.add_cycles(canister_id_1, 2_000_000_000_000).await;
//     println!("Cycles added to the first canister");

//     // Create the second canister
//     let canister_id_2 = pic.create_canister().await;
//     println!("Second canister created, ID: {:?}", canister_id_2);

//     pic.add_cycles(canister_id_2, 2_000_000_000_000).await;
//     println!("Cycles added to the second canister");

//     // Install the WASM bytes on both canisters
//     let wasm_path = std::env::var("EVM_LOGS_CANISTER_PATH")
//         .expect("EVM_LOGS_CANISTER_PATH must be set");

//     let wasm_bytes = tokio::fs::read(wasm_path)
//         .await
//         .expect("Failed to read the WASM file");

//     pic.install_canister(canister_id_1, wasm_bytes.clone(), vec![], None).await;
//     println!("Wasm installed in the first canister");

//     pic.install_canister(canister_id_2, wasm_bytes.clone(), vec![], None).await;
//     println!("Wasm installed in the second canister");

//     // Define the first set of addresses and topics
//     let addresses_to_monitor_1 = vec![
//         "0x0d4a11d5eeaac28ec3f61d100daf4d40471f1852".to_string(),
//     ];
//     let topics_to_monitor_1 = vec![
//         "0x0d4a11d5eeaac28ec3f61d100daf4d17f9k3v5h0".to_string(),
//     ];

//     // Define the second set of addresses and topics
//     let addresses_to_monitor_2 = vec![
//         "0x1d4a11d5eeaac28ec3f61d100daf4d40471f1854".to_string(),
//     ];
//     let topics_to_monitor_2 = vec![
//         "0x2d4a11d5eeaac28ec3f61d100daf4d17f9k3v5h5".to_string(),
//     ];

//     // Register the first filter for the first canister
//     let filter_string_1 = format!(
//         r#"address == "{}" && topic == "{}""#,
//         addresses_to_monitor_1[0], topics_to_monitor_1[0]
//     );

//     let filter_config_1 = vec![
//         ICRC16Map {
//             key: ICRC16Value::Text("icrc72:subscription:filter".to_string()),
//             value: ICRC16Value::Text(filter_string_1),
//         },
//     ];

//     let subscription_params_1 = SubscriptionRegistration {
//         namespace: "test_namespace_filters_1".to_string(),
//         config: filter_config_1,
//         memo: None,
//     };

//     // Register the second filter for the second canister
//     let filter_string_2 = format!(
//         r#"address == "{}" && topic == "{}""#,
//         addresses_to_monitor_2[0], topics_to_monitor_2[0]
//     );

//     let filter_config_2 = vec![
//         ICRC16Map {
//             key: ICRC16Value::Text("icrc72:subscription:filter".to_string()),
//             value: ICRC16Value::Text(filter_string_2),
//         },
//     ];

//     let subscription_params_2 = SubscriptionRegistration {
//         namespace: "test_namespace_filters_2".to_string(),
//         config: filter_config_2,
//         memo: None,
//     };

//     // Call the registration function in the first canister
//     println!("Calling subscription registration with filter in the first canister");
//     let result_1 = pic.update_call(
//         canister_id_1,
//         Principal::anonymous(),
//         "icrc72_register_subscription",
//         candid::encode_one(vec![subscription_params_1.clone()]).unwrap(),
//     ).await;

//     // Call the registration function in the second canister
//     println!("Calling subscription registration with filter in the second canister");
//     let result_2 = pic.update_call(
//         canister_id_2,
//         Principal::anonymous(),
//         "icrc72_register_subscription",
//         candid::encode_one(vec![subscription_params_2.clone()]).unwrap(),
//     ).await;

//     // Check the result of the subscription registration for both canisters
//     let check_subscription = |result: Result<WasmResult, pocket_ic::UserError>, expected_address: &str, expected_topic: &str| {
//         match result {
//             Ok(WasmResult::Reply(data)) => {
//                 let decoded_result: Vec<RegisterSubscriptionResult> = candid::decode_one(&data).unwrap();
                
//                 match &decoded_result[0] {
//                     RegisterSubscriptionResult::Ok(sub_id) => {
//                         println!("Subscription successfully created, ID: {:?}", sub_id);
//                     }
//                     RegisterSubscriptionResult::Err(err) => {
//                         panic!("Subscription registration error: {:?}", err);
//                     }
//                 }
//             },
//             Ok(WasmResult::Reject(err)) => {
//                 panic!("Call was rejected: {:?}", err);
//             }
//             Err(e) => panic!("Call error: {:?}", e),
//         }
//     };

//     check_subscription(result_1, &addresses_to_monitor_1[0], &topics_to_monitor_1[0]);
//     check_subscription(result_2, &addresses_to_monitor_2[0], &topics_to_monitor_2[0]);

//     // Now query the active filters for both canisters
//     let filter_result_1 = pic.query_call(
//         canister_id_1,
//         Principal::anonymous(),
//         "get_active_filters",
//         candid::encode_args::<()>(()).unwrap(),
//     ).await;

//     let filter_result_2 = pic.query_call(
//         canister_id_2,
//         Principal::anonymous(),
//         "get_active_filters",
//         candid::encode_args::<()>(()).unwrap(),
//     ).await;

//     let check_filters = |filter_result: Result<WasmResult, pocket_ic::UserError>, expected_address: &str, expected_topic: &str| {
//         match filter_result {
//             Ok(WasmResult::Reply(data)) => {
//                 let received_filters: Vec<Filter> = candid::decode_one(&data).unwrap();
                
//                 println!("\nActive filters for canister: ");
//                 for (i, filter) in received_filters.iter().enumerate() {
//                     println!("Filter {}:", i + 1);
//                     println!("  Addresses: {:?}", filter.addresses);
//                     if let Some(topics) = &filter.topics {
//                         println!("  Topics: {:?}", topics);
//                     } else {
//                         println!("  Topics: None");
//                     }
//                 }

//                 assert_eq!(received_filters.len(), 1, "Expected one filter");
//                 assert_eq!(received_filters[0].addresses[0].trim_matches('"'), expected_address, "Incorrect filter address");
                
//                 if let Some(received_topic) = &received_filters[0].topics {
//                     assert_eq!(
//                         received_topic[0][0].trim_matches('"'),
//                         expected_topic,
//                         "Incorrect filter topics"
//                     );
//                 } else {
//                     assert!(received_filters[0].topics.is_none(), "Unexpected topics in filter");
//                 }
//             },
//             Ok(WasmResult::Reject(err)) => {
//                 panic!("Call was rejected: {:?}", err);
//             }
//             Err(e) => panic!("Error querying filters: {:?}", e),
//         }
//     };

//     check_filters(filter_result_1, &addresses_to_monitor_1[0], &topics_to_monitor_1[0]);
//     check_filters(filter_result_2, &addresses_to_monitor_2[0], &topics_to_monitor_2[0]);

//     // Reduced sleep time for testing purposes
//     sleep(Duration::from_millis(500)).await;
// }



// // #[tokio::test]
// // async fn test_publication_registration() {
// //     // Initialize PocketIc
// //     let pic = PocketIc::new().await;

// //     // Create the subscription manager canister
// //     let subscription_manager_canister_id = pic.create_canister().await;
// //     pic.add_cycles(subscription_manager_canister_id, 2_000_000_000_000).await;

// //     // Install the subscription manager wasm
// //     let subscription_manager_wasm_path = std::env::var("EVM_LOGS_CANISTER_PATH")
// //         .expect("EVM_LOGS_CANISTER_PATH must be set");
// //     let subscription_manager_wasm_bytes = tokio::fs::read(subscription_manager_wasm_path)
// //         .await
// //         .expect("Failed to read the subscription manager WASM file");
// //     pic.install_canister(
// //         subscription_manager_canister_id,
// //         subscription_manager_wasm_bytes.to_vec(),
// //         vec![],
// //         None,
// //     )
// //     .await;

// //     // Register a publication
// //     let publisher_principal = Principal::anonymous(); // Or a specific principal
// //     let publication_registration = PublicationRegistration {
// //         namespace: "test_namespace".to_string(),
// //         config: vec![],
// //         memo: None,
// //     };
// //     let register_publication_result = pic
// //         .update_call(
// //             subscription_manager_canister_id,
// //             publisher_principal,
// //             "call_register_publication",
// //             candid::encode_one(vec![publication_registration.clone()]).unwrap(),
// //         )
// //         .await;

// //     // Check the publication registration result
// //     match register_publication_result {
// //         Ok(WasmResult::Reply(data)) => {
// //             let decoded_result: Vec<RegisterPublicationResult> = candid::decode_one(&data).unwrap();
// //             match &decoded_result[0] {
// //                 RegisterPublicationResult::Ok(pub_id) => {
// //                     println!("Publication successfully created, ID: {:?}", pub_id);
// //                     assert_ne!(*pub_id, Nat::from(0u32), "Publication ID should not be zero");
// //                 }
// //                 RegisterPublicationResult::Err(err) => {
// //                     panic!("Publication registration error: {:?}", err);
// //                 }
// //             }
// //         }
// //         Ok(WasmResult::Reject(err)) => {
// //             panic!("Publication registration rejected: {:?}", err);
// //         }
// //         Err(e) => {
// //             panic!("Publication registration call error: {:?}", e);
// //         }
// //     }
// // }

// #[tokio::test]
// async fn test_publication_registration() {
//     let pic = PocketIc::new().await;

//     let subscription_manager_canister_id = pic.create_canister().await;
//     pic.add_cycles(subscription_manager_canister_id, 2_000_000_000_000).await;

//     let subscription_manager_wasm_path = std::env::var("EVM_LOGS_CANISTER_PATH")
//         .expect("EVM_LOGS_CANISTER_PATH must be set");
//     let subscription_manager_wasm_bytes = tokio::fs::read(subscription_manager_wasm_path)
//         .await
//         .expect("Failed to read the subscription manager WASM file");
//     pic.install_canister(
//         subscription_manager_canister_id,
//         subscription_manager_wasm_bytes.to_vec(),
//         vec![],
//         None,
//     )
//     .await;

//     let publications = vec![
//         PublicationRegistration {
//             namespace: "test_namespace_1".to_string(),
//             config: vec![],
//             memo: None,
//         },
//         PublicationRegistration {
//             namespace: "test_namespace_2".to_string(),
//             config: vec![],
//             memo: None,
//         },
//         PublicationRegistration {
//             namespace: "test_namespace_3".to_string(),
//             config: vec![],
//             memo: None,
//         },
//     ];

//     let publisher_principal = Principal::anonymous();
//     let register_publication_result = pic
//         .update_call(
//             subscription_manager_canister_id,
//             publisher_principal,
//             "call_register_publication",
//             candid::encode_one(publications.clone()).unwrap(),
//         )
//         .await;

//     // Check the publication registration results
//     match register_publication_result {
//         Ok(WasmResult::Reply(data)) => {
//             let decoded_results: Vec<RegisterPublicationResult> = candid::decode_one(&data).unwrap();

//             assert_eq!(
//                 decoded_results.len(),
//                 publications.len(),
//                 "Number of results should match number of publications"
//             );

//             // Collect the publication IDs
//             let mut publication_ids = Vec::new();

//             for (i, result) in decoded_results.iter().enumerate() {
//                 match result {
//                     RegisterPublicationResult::Ok(pub_id) => {
//                         println!(
//                             "Publication {} successfully created, ID: {:?}",
//                             i + 1,
//                             pub_id
//                         );
//                         assert_ne!(
//                             pub_id,
//                             &Nat::from(0u32),
//                             "Publication ID should not be zero"
//                         );

//                         // Convert pub_id to String and collect, for future testing on uniqueness
//                         publication_ids.push(pub_id.to_string());
//                     }
//                     RegisterPublicationResult::Err(err) => {
//                         panic!("Publication registration error: {:?}", err);
//                     }
//                 }
//             }

//             // Check that all IDs are unique
//             let unique_ids: HashSet<String> = publication_ids.iter().cloned().collect();

//             assert_eq!(
//                 unique_ids.len(),
//                 publication_ids.len(),
//                 "Publication IDs should be unique"
//             );
//         }
//         Ok(WasmResult::Reject(err)) => {
//             panic!("Publication registration rejected: {:?}", err);
//         }
//         Err(e) => {
//             panic!("Publication registration call error: {:?}", e);
//         }
//     }
// }


// #[tokio::test]
// async fn test_event_publishing() {
//     let pic = PocketIc::new().await;

//     let subscription_manager_canister_id = pic.create_canister().await;
//     pic.add_cycles(subscription_manager_canister_id, 2_000_000_000_000).await;

//     let subscription_manager_wasm_path = std::env::var("EVM_LOGS_CANISTER_PATH")
//         .expect("EVM_LOGS_CANISTER_PATH must be set");
//     let subscription_manager_wasm_bytes = tokio::fs::read(subscription_manager_wasm_path)
//         .await
//         .expect("Failed to read the subscription manager WASM file");
//     pic.install_canister(
//         subscription_manager_canister_id,
//         subscription_manager_wasm_bytes.to_vec(),
//         vec![],
//         None,
//     )
//     .await;

//     let publisher_principal = Principal::anonymous(); // Use appropriate principal
//     let publication_registration = PublicationRegistration {
//         namespace: "test_namespace".to_string(),
//         config: vec![],
//         memo: None,
//     };
//     let register_publication_result = pic
//         .update_call(
//             subscription_manager_canister_id,
//             publisher_principal,
//             "call_register_publication",
//             candid::encode_one(vec![publication_registration.clone()]).unwrap(),
//         )
//         .await;

//     let publication_id = match register_publication_result {
//         Ok(WasmResult::Reply(data)) => {
//             let decoded_results: Vec<RegisterPublicationResult> = candid::decode_one(&data).unwrap();
//             match &decoded_results[0] {
//                 RegisterPublicationResult::Ok(pub_id) => {
//                     println!("Publication successfully created, ID: {:?}", pub_id);
//                     assert_ne!(
//                         *pub_id,
//                         Nat::from(0u32),
//                         "Publication ID should not be zero"
//                     );
//                     pub_id.clone()
//                 }
//                 RegisterPublicationResult::Err(err) => {
//                     panic!("Publication registration error: {:?}", err);
//                 }
//             }
//         }
//         Ok(WasmResult::Reject(err)) => {
//             panic!("Publication registration rejected: {:?}", err);
//         }
//         Err(e) => {
//             panic!("Publication registration call error: {:?}", e);
//         }
//     };

//     let events = vec![
//         Event {
//             id: Nat::from(0u64), 
//             prev_id: None,
//             timestamp: 0,
//             namespace: "test_namespace".to_string(),
//             data: ICRC16Value::Text("Test event data 1".to_string()),
//             headers: None,
//         },
//         Event {
//             id: Nat::from(0u64),
//             prev_id: None,
//             timestamp: 0,
//             namespace: "test_namespace".to_string(),
//             data: ICRC16Value::Text("Test event data 2".to_string()),
//             headers: None,
//         },
//         Event {
//             id: Nat::from(0u64),
//             prev_id: None,
//             timestamp: 0,
//             namespace: "test_namespace".to_string(),
//             data: ICRC16Value::Text("Test event data 3".to_string()),
//             headers: None,
//         },
//     ];

//     let publish_events_result = pic
//         .update_call(
//             subscription_manager_canister_id,
//             publisher_principal,
//             "icrc72_publish",
//             candid::encode_one(events.clone()).unwrap(),
//         )
//         .await;

//     match publish_events_result {
//         Ok(WasmResult::Reply(data)) => {
//             let decoded_results: Vec<Option<Result<Vec<Nat>, String>>> = candid::decode_one(&data).unwrap();
//             assert_eq!(
//                 decoded_results.len(),
//                 events.len(),
//                 "Number of results should match number of events"
//             );

//             let mut event_ids = Vec::new();

//             for (i, result) in decoded_results.iter().enumerate() {
//                 match result {
//                     Some(Ok(event_id_vec)) => {
//                         assert_eq!(
//                             event_id_vec.len(),
//                             1,
//                             "Expected one event ID per event"
//                         );
//                         let event_id = &event_id_vec[0];
//                         println!(
//                             "Event {} published successfully, ID: {:?}",
//                             i + 1,
//                             event_id
//                         );
//                         assert_ne!(event_id, &Nat::from(0u32), "Event ID should not be zero");
//                         event_ids.push(event_id.to_string());
//                     }
//                     Some(Err(err)) => {
//                         panic!("Event publish error: {:?}", err);
//                     }
//                     None => {
//                         panic!("Event publish returned None for event {}", i + 1);
//                     }
//                 }
//             }

//             let unique_event_ids: HashSet<String> = event_ids.iter().cloned().collect();
//             assert_eq!(
//                 unique_event_ids.len(),
//                 event_ids.len(),
//                 "Event IDs should be unique"
//             );

//         }
//         Ok(WasmResult::Reject(err)) => {
//             panic!("Event publish rejected: {:?}", err);
//         }
//         Err(e) => {
//             panic!("Event publish call error: {:?}", e);
//         }
//     }
// }



// #[tokio::test]
// async fn test_event_publishing_and_notification_delivery() {
//     let pic = PocketIc::new().await;

//     let subscription_manager_canister_id = pic.create_canister().await;
//     pic.add_cycles(subscription_manager_canister_id, 2_000_000_000_000).await;

//     let subscription_manager_wasm_path = std::env::var("EVM_LOGS_CANISTER_PATH")
//         .expect("EVM_LOGS_CANISTER_PATH must be set");
//     let subscription_manager_wasm_bytes = tokio::fs::read(subscription_manager_wasm_path)
//         .await
//         .expect("Failed to read the subscription manager WASM file");
//     pic.install_canister(
//         subscription_manager_canister_id,
//         subscription_manager_wasm_bytes.to_vec(),
//         vec![],
//         None,
//     )
//     .await;

//     // Create the subscriber canister
//     let subscriber_canister_id = pic.create_canister().await;
//     pic.add_cycles(subscriber_canister_id, 2_000_000_000_000).await;

//     // Install the subscriber wasm
//     let subscriber_wasm_path = std::env::var("TEST_CANISTER_WASM_PATH")
//         .expect("TEST_CANISTER_WASM_PATH must be set");
//     let subscriber_wasm_bytes = tokio::fs::read(subscriber_wasm_path)
//         .await
//         .expect("Failed to read the subscriber WASM file");
//     pic.install_canister(
//         subscriber_canister_id,
//         subscriber_wasm_bytes.to_vec(),
//         vec![],
//         None,
//     )
//     .await;

//     // Register a subscription from the subscriber canister
//     let subscription_registration = SubscriptionRegistration {
//         namespace: "test_namespace".to_string(),
//         config: vec![],
//         memo: None,
//     };
//     let register_subscription_result = pic
//         .update_call(
//             subscription_manager_canister_id,
//             subscriber_canister_id, 
//             "icrc72_register_subscription",
//             candid::encode_one(vec![subscription_registration.clone()]).unwrap(),
//         )
//         .await;

//     // Check the subscription registration result
//     match register_subscription_result {
//         Ok(WasmResult::Reply(data)) => {
//             let decoded_result: Vec<RegisterSubscriptionResult> = candid::decode_one(&data).unwrap();
//             match &decoded_result[0] {
//                 RegisterSubscriptionResult::Ok(sub_id) => {
//                     println!("Subscription successfully created, ID: {:?}", sub_id);
//                 }
//                 RegisterSubscriptionResult::Err(err) => {
//                     panic!("Subscription registration error: {:?}", err);
//                 }
//             }
//         }
//         Ok(WasmResult::Reject(err)) => {
//             panic!("Subscription registration rejected: {:?}", err);
//         }
//         Err(e) => {
//             panic!("Subscription registration call error: {:?}", e);
//         }
//     }

//     // Register a publication
//     let publisher_principal = Principal::anonymous(); 
//     let publication_registration = PublicationRegistration {
//         namespace: "test_namespace".to_string(),
//         config: vec![],
//         memo: None,
//     };
//     let register_publication_result = pic
//         .update_call(
//             subscription_manager_canister_id,
//             publisher_principal,
//             "call_register_publication",
//             candid::encode_one(vec![publication_registration.clone()]).unwrap(),
//         )
//         .await;

//     // Check the publication registration result
//     match register_publication_result {
//         Ok(WasmResult::Reply(data)) => {
//             let decoded_result: Vec<RegisterPublicationResult> = candid::decode_one(&data).unwrap();
//             match &decoded_result[0] {
//                 RegisterPublicationResult::Ok(pub_id) => {
//                     println!("Publication successfully created, ID: {:?}", pub_id);
//                 }
//                 RegisterPublicationResult::Err(err) => {
//                     panic!("Publication registration error: {:?}", err);
//                 }
//             }
//         }
//         Ok(WasmResult::Reject(err)) => {
//             panic!("Publication registration rejected: {:?}", err);
//         }
//         Err(e) => {
//             panic!("Publication registration call error: {:?}", e);
//         }
//     }

//     // Publish an event
//     let event = Event {
//         id: Nat::from(0u64), // ID will be assigned by the canister
//         prev_id: None,
//         timestamp: 0,
//         namespace: "test_namespace".to_string(),
//         data: ICRC16Value::Text("Test event data".to_string()),
//         headers: None,
//     };
//     let publish_events_result = pic
//         .update_call(
//             subscription_manager_canister_id,
//             publisher_principal,
//             "icrc72_publish",
//             candid::encode_one(vec![event.clone()]).unwrap(),
//         )
//         .await;

//     // Check the event publishing result
//     match publish_events_result {
//         Ok(WasmResult::Reply(data)) => {
//             let decoded_results: Vec<Option<Result<Vec<Nat>, String>>> = candid::decode_one(&data).unwrap();
//             match &decoded_results[0] {
//                 Some(Ok(event_ids)) => {
//                     println!("Event published successfully, IDs: {:?}", event_ids);
//                     assert_eq!(event_ids.len(), 1, "Expected one event ID");
//                     assert_ne!(event_ids[0], Nat::from(0u32), "Event ID should not be zero");
//                 }
//                 Some(Err(err)) => {
//                     panic!("Event publish error: {:?}", err);
//                 }
//                 None => {
//                     panic!("Event publish returned None");
//                 }
//             }
//         }
//         Ok(WasmResult::Reject(err)) => {
//             panic!("Event publish rejected: {:?}", err);
//         }
//         Err(e) => {
//             panic!("Event publish call error: {:?}", e);
//         }
//     }

//     // Wait for the notification to be sent
//     sleep(Duration::from_millis(500)).await;

//     // Query the subscriber canister to retrieve notifications
//     let get_notifications_result = pic
//         .query_call(
//             subscriber_canister_id,
//             Principal::anonymous(),
//             "get_notifications",
//             candid::encode_args(()).unwrap(),
//         )
//         .await;

//     // Verify that the subscriber received the notification
//     match get_notifications_result {
//         Ok(WasmResult::Reply(data)) => {
//             let notifications: Vec<EventNotification> = candid::decode_one(&data).unwrap();
//             println!("Received notifications: {:?}", notifications);
//             assert_eq!(notifications.len(), 1, "Expected one notification");
//             let notification = &notifications[0];
//             assert_eq!(notification.namespace, "test_namespace", "Incorrect namespace");
//             if let ICRC16Value::Text(ref text) = notification.data {
//                 assert_eq!(text, "Test event data", "Incorrect event data");
//             } else {
//                 panic!("Unexpected data type in notification");
//             }
//         }
//         Ok(WasmResult::Reject(err)) => {
//             panic!("Get notifications rejected: {:?}", err);
//         }
//         Err(e) => {
//             panic!("Get notifications call error: {:?}", e);
//         }
//     }
// }
