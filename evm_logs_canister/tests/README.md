# evm-logs-canister tests overview

This documentation provides an understanding of the different test cases ensuring the and reliability of the different workflows within the EVM logs canister.

All integration tests use simulated Internet Computer environment - PocketIc.

### Key constants:
- `max_response_bytes`: Defines the maximum number of bytes expected from the evm-rpc canister.
This value is intentionally set lower than in the basic evm-logs workflow to effectively test the batch request fetching logic and ensure proper handling of large data responses.
- `estimate_events_num`: Specifies the estimated number of events expected per contract address from the evm-rpc canister.
This value helps optimize the event fetching process by providing an approximation of the event volume for each subscription.


## Main Workflow Test

This test verifies the main workflow of the EVM logs canister with multiple subscribers.

### Overview:
- Deploy and initialize multiple canisters: `evm-logs-canister`, `evm-rpc-mocked`, `proxy`, `cycles-wallet`, 
  and multiple subscriber canisters (as defined by the constant value in the code).
- Each subscriber canister subscribes to the `evm-logs-canister` with a randomly generated filter.
- The test ensures that all subscriptions are correctly registered (subscription count matches).
- Finally, it verifies that each subscriber received expected event notification, accordingly to the filter.

### Key Assertions:
- The number of registered subscriptions matches the expected count.
- Each subscriber receives exactly notification after the logs are fetched and processed. Notification must correspond to the subscriber filter. 

---

## Batch Request Test

This test simulates multiple subscribers registering event filters and ensures that event notifications are correctly processed and delivered using the batch requests feature. To achieve this we pass smaller `max_response_bytes` into the `evm_logs_canister`

### Overview:
- Deploy and initialize multiple canisters: `evm-logs-canister`, `evm-rpc-mocked`, `proxy`, `cycles-wallet`, 
  and multiple subscriber canisters (as defined by the constant value in the code).
- Each subscriber canister subscribes to the `evm-logs-canister` with a randomly generated filter.
- The test ensures that all subscriptions are correctly registered (subscription count matches).
- Verify amount of the batches for `eth_getLogs` fetching function. **Pay attention to `max_response_bytes` and `estimate_events_num` here. Because number of batch calls mostly depends on these constants. For deeper understanding check [chunks calculation logic](link)**
- Finally, it verifies that each subscriber received expected event notification, accordingly to the filter (as in `main-workflow` test)


### Key Assertions:
- The subscriber should receive exactly `num_filters` notifications.
- Each received notification should match one of the originally subscribed filters.
- The `evm-logs-canister` should batch requests efficiently, assertion calculates and comparing with expected amount of batch calls.


---

## Subscribe-Publish-Receive Test

This simle test validates the flow of event subscriptions and notification publication in the `evm-logs-canister`. It ensures the integrity of the event delivery mechanism from subscription to notification.

### Overview:
- Deploy and initialize multiple canisters: `evm-logs-canister`, `evm-rpc-mocked`, `proxy`, `cycles-wallet`, 
  and multiple subscriber canisters (as defined by the constant value in the code).
- Singular subscriber canister subscribes to the `evm-logs-canister` with a randomly generated filter.
- An event is constructed with a predefined `chain_id`, `address`, and `topics` which corresponds to the subscribers filter and being published manually by the call from environment. 

  - The subscriber canister is queried for received notifications.
  - The response is validated to ensure:
    - One notification was received.
    - The event metadata (`chain_id`, `event_id`, `data`) matches the published event.

### Key Assertions:
- The subscription registration process completes successfully.
- Published events are correctly delivered as notifications to the subscriber.
- The notification data matches the original event content.


---

