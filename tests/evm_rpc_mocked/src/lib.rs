use std::{cell::RefCell, str::FromStr};

use candid::{CandidType, Deserialize, Nat, Principal};
use evm_logs_types::{Filter, SubscriptionInfo};
use evm_rpc_types::{
    BlockTag, GetLogsArgs, Hex, Hex20, Hex32, Hex256, LogEntry, MultiRpcResult, Nat256, RpcConfig, RpcServices,
};
use ic_cdk::api::call::call;
use ic_cdk_macros::*;

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct Config {
    pub evm_logs_canister_id: Principal, // Example principal to be passed
}

thread_local! {
    static CONFIG: RefCell<Config> = RefCell::new(Config {
        evm_logs_canister_id: Principal::anonymous(),
    });
    static ETH_GET_LOGS_COUNTER: RefCell<u64> = RefCell::new(0);
}

#[init]
async fn init(config: Config) {
    CONFIG.with(|c| *c.borrow_mut() = config);
}

#[query(name = "get_eth_get_logs_count")]
fn get_eth_get_logs_count() -> u64 {
    ETH_GET_LOGS_COUNTER.with(|counter| *counter.borrow())
}

#[query]
fn get_candid_pointer() -> String {
    __export_service()
}

candid::export_service!();

#[update(name = "eth_getLogs")]
pub async fn eth_get_logs(
    _source: RpcServices,
    _config: Option<RpcConfig>,
    _args: GetLogsArgs,
) -> MultiRpcResult<Vec<LogEntry>> {
    ic_cdk::println!("CALLING eth_getLogs");
    ETH_GET_LOGS_COUNTER.with(|counter| *counter.borrow_mut() += 1);

    let log_entries = get_same_logs_as_sub_filters().await;

    MultiRpcResult::Consistent(Ok(log_entries))
}

#[update(name = "eth_getBlockByNumber")]
pub async fn eth_get_block_by_number(
    _source: RpcServices,
    _config: Option<RpcConfig>,
    _args: BlockTag,
) -> MultiRpcResult<evm_rpc_types::Block> {
    ic_cdk::println!("CALLING eth_getBlockByNumber");

    let block = evm_rpc_types::Block {
        base_fee_per_gas: Some(Nat256::from(10u32)),
        number: Nat256::from(123456u32),
        difficulty: Some(Nat256::from(5000000u32)),
        extra_data: Hex::from(vec![0x01, 0x02, 0x03]),
        gas_limit: Nat256::from(30000000u32),
        gas_used: Nat256::from(15000000u32),
        hash: Hex32::from([0xaa; 32]),
        logs_bloom: Hex256::from([0xbb; 256]),
        miner: Hex20::from([0xcc; 20]),
        mix_hash: Hex32::from([0xdd; 32]),
        nonce: Nat256::from(1234u32),
        parent_hash: Hex32::from([0xee; 32]),
        receipts_root: Hex32::from([0xff; 32]),
        sha3_uncles: Hex32::from([0x11; 32]),
        size: Nat256::from(1000u32),
        state_root: Hex32::from([0x22; 32]),
        timestamp: Nat256::from(1707000000u32),
        total_difficulty: Some(Nat256::from(8000000u32)),
        transactions: vec![Hex32::from([0x33; 32])],
        transactions_root: Some(Hex32::from([0x44; 32])),
        uncles: vec![Hex32::from([0x55; 32])],
    };

    MultiRpcResult::Consistent(Ok(block))
}

async fn get_same_logs_as_sub_filters() -> Vec<LogEntry> {
    // Call get_subscriptions from the evm_logs_canister

    let evm_logs_canister = CONFIG.with(|config| config.borrow().evm_logs_canister_id);
    let result: Result<(Vec<SubscriptionInfo>,), _> = call(
        evm_logs_canister,
        "get_subscriptions",
        (None::<u32>, None::<Nat>, None::<Vec<Filter>>),
    )
    .await;

    let subscriptions = match result {
        Ok((subscriptions,)) => subscriptions,
        Err(err) => {
            ic_cdk::print(format!("Error fetching subscriptions: {:?}", err));
            return vec![];
        }
    };

    let mut log_entries = vec![];

    // Iterate over subscriptions and generate logs based on filters
    for subscription in subscriptions {
        let filter = subscription.filter;
        let filter_address = filter.address;
        let filter_topics = filter.topics.unwrap_or_default();

        // Create a sample log entry matching the filter
        let log_entry = LogEntry {
            address: filter_address,
            topics: filter_topics
                .iter()
                .flat_map(|topic_list| topic_list.iter().cloned())
                .collect(),
            transaction_hash: Some(
                Hex32::from_str("0xd9cf780ea5308e53d5339512353367e9975e936c2fe94ac63b3da2d4b298b891").unwrap(),
            ),
            block_number: None,
            data: Hex::from(vec![]),
            block_hash: None,
            transaction_index: None,
            log_index: None,
            removed: false,
        };

        log_entries.push(log_entry);
    }

    log_entries
}
