### What is the project structure?

```
.
├── canister_utils 
├── evm_logs_canister
├── evm_logs_types
├── evm_rpc_mocked
├── proxy_canister
├── scripts
├── test_canister
└── test_configuration
```
* `canister_utils` - utils that is used inside canisters 
* `evm_logs_canister` - main canister for processing logs
* `evm_logs_types` - auxiliary library for storing interface types outside canister 
* `evm_rpc_mocked` - mocked evm_rpc canister, created for testing purposes
* `proxy_canister` - canister that is responsible for triggering user method `handle_notification(...)`
* `scripts` - contains additional scripts for fetching binary file for testing
* `test_canister` - canister that can be used for subscription in testing purposes
* `test_configuration` - contains additional info to simplify testing


### What is the purpose of using proxy canister? 

To avoid possibility of exploiting DoS attacks ([DoS issues][1]) to the canister with main logic, we need to use a proxy canister.

Or in other words, avoid blocking of `evm_logs_canister` by invoking user defined method. 
In its code can be any malicious function that can stop or slow down execution of main canister. 
In case of problems with proxy canister it can be simply redeployed without causing any issues to `evm_logs_canister`. 

### How does `evm-logs-canister` receive logs under the hood?

`evm-logs-canister` receive logs by using [batches](#what-is-batch-logic-calculation), only in this way we can bypass the limits of [ingress message payload][3].

To sum up the overall algorithm, we can say that it conforms to this steps:
1. Divide available addresses by batch size.
2. For each of them create asynchronous task that would ask for appropriate events for addresses in batch. 
3. Merge received events into one, to get to know how much bytes we have received.
4. [Charge](#cycles-withdrawal-per-event) appropriate amount of cycles from subscribers.

### What is batch logic calculation?

Batch logic comes up here, because canister can't fetch logs for all addresses at one time. Due to the limitation of [ingress message payload ][3] events for all users just can't be fit into one message. That's why we have to divide events receiving in so-called "batches".

Batch amount of addresses is calculated in this way:
$$ \text{max}  \Biggl( \text{min} \Biggl( \dfrac{\text{max_response_bytes}}{\text{bytes_per_address}}, \text{addresses_num} \Biggl) , 1 \Biggl)  $$
where
- $\text{bytes_per_address} = (\text{estimate_events_num} * \text{EVM_EVENT_SIZE_BYTES})$
- $\text{EVM_EVENT_SIZE_BYTES}$ = 800
- $\text{addresses_num}$ - number of addresses for which we have to send events 

This formula estimates possible value of addresses that would fit in $\text{max_response_bytes}$ (by now it is limited to 1mb out of 2mb).

### Cycles withdrawal per event

TODO write description 

$$  \text{BASE_CALL_CYCLES} + \text{cycles_for_request} + \text{cycles_for_response} + (\text{cycles_for_request} + \text{cycles_for_response}) $$
where
* $\text{request_size_bytes} = \text{BASE_STRUCT_SIZE} + (\text{ETH_ADDRESS_SIZE} * \text{addresses_count}) + \text{ETH_TOPIC_SIZE} * \text{count of all topics for specific user}$
* $\text{BASE_STRUCT_SIZE} = 8$
* $\text{ETH_ADDRESS_SIZE} = 20$
* $\text{ETH_TOPIC_SIZE} = 32$
* $\text{response_size_bytes} = \sum ( \text{bytes len from encoded LogEntry} ) $
* $\text{cycles_for_request} = \text{request_size_bytes} * \text{CYCLES_PER_BYTE_SEND}$
* $\text{cycles_for_response} = \text{response_size_bytes} * \text{CYCLES_PER_BYTE_RECEIVE}$


### Where we can take topics?

Topics are Hex32 value that consist from 32 sequenced bytes. Topics are used to encode some event . Generally you can find them on [etherscan][2] 


### How to create filters?

The most common way to search only for first topic and then to clarify subscription



[1]: https://internetcomputer.org/docs/current/developer-docs/security/security-best-practices/inter-canister-calls#be-aware-of-the-risks-involved-in-calling-untrustworthy-canisters
[2]: https://etherscan.io/
[3]: https://internetcomputer.org/docs/building-apps/canister-management/resource-limits