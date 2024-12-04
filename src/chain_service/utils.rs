use evm_rpc_canister_types::LogEntry;
use evm_logs_types::ChainName;

pub fn convert_log_to_string(chain_name: &ChainName, log: &LogEntry) -> String {
    format!(
        "Chain: {:?}, Address: {}, TxHash: {:?}, Block: {:?}, Topics: {:?}, Data: {}",
        chain_name,
        log.address,
        log.transactionHash,
        log
            .blockNumber
            .as_ref()
            .map(|n| n.0.clone())
            .unwrap_or_default(),
        log.topics,
        log.data
    )
}

pub fn print_logs(logs: &Vec<String>) {
    for log in logs {
        ic_cdk::println!("Log: {:?}", log);
    }
}
