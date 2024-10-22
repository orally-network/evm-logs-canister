# evm-logs-cainster

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
- **ChainService[]**: A collection of services, one for each supported blockchain. Each service manages its connection, fetching logs, and notifying subscribed canisters.
  - **EventListener**: Periodically fetches logs based on active filters.
  - **EventEmitter**: Distributes fetched logs to the appropriate subscribers.

### Subscription Process: Implementation following [Pub-Sub ICRC72 standard](https://github.com/icdevs/ICEventsWG/blob/main/Meetings/20240529/icrc72draft.md)
1. **Subscription Creation**:
   - Developers call the `subscribe(filter, config)` method to initiate a subscription.
   - The `filter` includes `chain_id`, `contract_address`, and `topics`.
   - The `config` can optionally include `fulfill_func_name` to specify the method name where logs should be delivered within the subscriber canister. If not provided, logs are sent to the default `fulfill_log(sub_id, LogEntry)` method.
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
   
2. **Build:**:
   ```
   make build
   ```
3. **Deploy canisters:**
   ```
   make deploy
   ```
   
### After these steps you can use evm-logs-canister functionality by calling implemented candid method.


## Canister methods

### Subscription

You can subscribe on the evm_logs_canister from another canister by specifying your filter(test_canister already built and deployed by the build scipt for demonstration):

```
  dfx canister call test_canister1 register_subscription '(
      principal "bkyz2-fmaaa-aaaaa-qaaaq-cai",
      vec {
          record {
              namespace = "com.events.Ethereum";
              filters = vec {
                  record {
                      addresses = vec { "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852" };
                      topics = opt vec {
                        vec { "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822" };
                        vec { "0x0000000000000000000000003fc91a3afd70395cd496c647d5a6cc9d4b2b7fad" };
                      };
                  };
              };
              
              memo = null;
          };
      }
  )'
```

#### Strategy of EVM Topics Passing:
When sending a filter to the EVM node, you can specify which log topics should match specific positions in the event. Here’s how the topic filters work:

- [] (empty): Matches any transaction, as no specific topics are required.
- [A]: Matches if the first topic of the transaction is A, with no restrictions on the following topics.
- [null, B]: Matches any transaction with B in the second position, regardless of the first topic.
- [A, B]: Matches transactions where A is in the first position and B is in the second.
- [[A, B], [A, B]]: Matches if the first topic is either A or B, and the second topic is also either A or B. This creates an "OR" condition for each position.

This strategy provides flexibility in filtering specific transactions based on topic order and values.

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