# evm-logs-canister

## Overview

The `evm-logs-canister` is a decentralized solution on the Internet Computer Protocol (ICP) designed to streamline the process of subscribing to and handling event logs from Ethereum Virtual Machine (EVM) compatible blockchains.

## Problem

Developers needing real-time information from blockchain events often have to deploy and maintain individual listeners for each application, leading to increased complexity and cost. This process, known as "chain fusion," requires each developer's canister to continuously listen to all block events, which is inefficient and resource-intensive.

## Solution

Our solution introduces a publish-subscribe (pub/sub) proxy model that allows developers to subscribe their canisters to specific logs on a chosen chain, thereby sharing the operational costs among all subscribers. This model not only reduces the redundancy of data fetching but also optimizes resource utilization across the network.

## How It Works

### Components
- **Developer**: The user who subscribes to and interacts with the canister.
- **SubscriptionManager**: Manages all subscriptions, including creation, updating, and deletion.
- **ChainService**: A collection of services, one for each supported blockchain. Each service manages its connection, fetching logs, and notifying subscribed canisters.

### Subscription Process: Implementation following [Pub-Sub ICRC72 standard](https://github.com/icdevs/ICEventsWG/blob/main/Meetings/20240529/icrc72draft.md)
1. **Subscription Creation**:
   - Developers call the `subscribe(filter)` method to initiate a subscription.
   - The `filter` includes `chain`, `contract_address`, and `topics`.
   - Cycles are attached to the call for subscription fees and are deducted as logs are fetched and delivered. The base fee for listening logs will be shared between all subscribers, and the individual fee will be deducted after the subscriber gets the log. 

2. **Subscription Management**:
   - `check_subscription(sub_id)`: Retrieves the status and the remaining balance of a subscription.
   - `unsubscribe(sub_id)`: Cancels the subscription and refunds any remaining cycles.

3. **Log Handling**:
   - Each `ChainService` runs an `EventListener` based on a set interval, querying `eth_getLogs` with all current filters.
   - The `EventEmitter` processes and routes the fetched logs to the correct subscribers through the `publish(sub_id, LogEntry)` method of the `SubscriptionManager`.

Notes: 
- To avoid [DoS issues](https://internetcomputer.org/docs/current/developer-docs/security/security-best-practices/inter-canister-calls#be-aware-of-the-risks-involved-in-calling-untrustworthy-canisters) with callback mechanics (publish to subscriber), you need to use a proxy canister.
- Future improvement (when it will be in mainnet) use (best-effort messages)[https://forum.dfinity.org/t/scalable-messaging-model/26920] for callbacks.
- Careful cycles calculation (link)[https://internetcomputer.org/docs/current/developer-docs/gas-cost], (link)[https://internetcomputer.org/docs/current/developer-docs/cost-estimations-and-examples] 

### Sequence Diagram
![evm-logs-canister-sequence](https://github.com/user-attachments/assets/5e1460ba-e8ff-4416-831c-4e0eb2b57617)

## Running the project locally

1. **Start the DFX environment**:
   
   ```
   dfx start --clean
   ```
   
2. **Build and install code into the canister:**:
   ```
   make local_deploy
   ```
   
#### After these steps you can use evm-logs-canister functionality by calling implemented candid methods.


## Canister methods

### Subscription

You can subscribe on the evm_logs_canister from another canister by specifying your filter in the code. 
All you need is just initialize *SubscriptionRegistration* struct with your custom filter:

```
pub struct SubscriptionRegistration {
    pub namespace: String,
    pub filter: Filter,
    pub memo: Option<Vec<u8>>, // Blob
}
```
In the filter parameter you should set contract address and event topics you want to subscribe on

Example of [subscription struct creation](https://github.com/orally-network/evm-logs-canister/blob/7c042a5b776eb8037891c084f936aaa730694545/src/canister_testing/test_canister1/src/utils.rs#L93):

```
pub fn create_chainfusion_deposit_config() -> SubscriptionRegistration {
    let address = "0x7574eb42ca208a4f6960eccafdf186d627dcc175".to_string();
    let topics = Some(vec![vec![
        "0x257e057bb61920d8d0ed2cb7b720ac7f9c513cd1110bc9fa543079154f45f435".to_string(),
    ]]);

    let filter = Filter { address, topics };

    SubscriptionRegistration {
        namespace: "com.events.Ethereum".to_string(),
        filter,
        memo: None,
    }
}
```
This subscription config will be listening [chain fusion ckUSDT minter deposit events](https://etherscan.io/tx/0xb358914900ae855dc283bdba8c2de1f2a02f1b9610c35f0ec49f71c60465f104#eventlog#165).
After *SubscriptionRegistration* is initialized you are free to subscribe to evm-logs-canister with specified filter:
```
let result: Result<(RegisterSubscriptionResult,), _> =
        call(canister_id, "subscribe", (subscription_params,)).await;
```
Where canister_id - evm_logs_canister ID

####  EVM Topics Passing
When sending a filter to the EVM node, you can specify which log topics should match specific positions in the event. Here’s how the topic filters work:

- [] (empty): Matches any transaction, as no specific topics are required.
- [A]: Matches if the first topic of the transaction is A, with no restrictions on the following topics.
- [null, B]: Matches any transaction with B in the second position, regardless of the first topic.
- [A, B]: Matches transactions where A is in the first position and B is in the second.
- [[A, B], [A, B]]: Matches if the first topic is either A or B, and the second topic is also either A or B. This creates an "OR" condition for each position.

This strategy provides flexibility in filtering specific transactions based on topic order and values.

More info: [eth_getLogs RPC method](https://docs.alchemy.com/docs/deep-dive-into-eth_getlogs) 
### Events handling and decoding
After subscribing, you will receive EVM events at certain intervals via the **handle_notification** callback:
```
#[update]
async fn handle_notification(notification: EventNotification) {
    log!("Received notification: {:?}", notification);

    // decode each notification in corresponding way and save decoded data
    ...
}
```
You can implement your own decoder 
to decode event data. A common use case would be to map a specific decoder to each subscription filter
creation, since each evm event has its own data format, a special decoding approach must be applied.
Common use cases and its implementations can be checked in [Test Canister source code](https://github.com/orally-network/evm-logs-canister/tree/7c042a5b776eb8037891c084f936aaa730694545/src/canister_testing/test_canister1) of this repo. 

Impementation of your own events decoder: [ethereum_sync_decoder](https://github.com/orally-network/evm-logs-canister/blob/7c042a5b776eb8037891c084f936aaa730694545/src/canister_testing/test_canister1/src/decoders.rs#L33)

### Get your active subscriptions with IDs

```
dfx canister call test_canister1 get_subscriptions '(
    principal "bkyz2-fmaaa-aaaaa-qaaaq-cai"
)' 
```

### Cancel subscription(unsubscribe)


```
dfx canister call test_canister1 unsubscribe '(
    principal "bkyz2-fmaaa-aaaaa-qaaaq-cai",
    <SUB_ID>:nat
)'
```