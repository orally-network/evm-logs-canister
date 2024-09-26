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

### Build and Deploy
