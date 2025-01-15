use evm_logs_types::ChainName;
use evm_rpc_canister_types::LogEntry;

const MAX_RESPONSE_BYTES: u32 = 1_000_000;
const EVM_EVENT_SIZE_BYTES: u32 = 800;

pub fn convert_log_to_string(chain_name: &ChainName, log: &LogEntry) -> String {
    format!(
        "Chain: {:?}, Address: {}, TxHash: {:?}, Block: {:?}, Topics: {:?}, Data: {}",
        chain_name,
        log.address,
        log.transactionHash,
        log.blockNumber
            .as_ref()
            .map(|n| n.0.clone())
            .unwrap_or_default(),
        log.topics,
        log.data
    )
}

pub fn print_logs(logs: &Vec<String>) {
    for log in logs {
        crate::log!("Log: {:?}", log);
    }
}

pub fn calculate_request_chunk_size(events_num_per_interval: u32, addresses_num: u32) -> usize {
    if addresses_num == 0 {
        return 1;
    }

    let bytes_per_address = events_num_per_interval.saturating_mul(EVM_EVENT_SIZE_BYTES);

    let max_addresses_per_request = if bytes_per_address == 0 {
        u32::MAX 
    } else {
        MAX_RESPONSE_BYTES / bytes_per_address
    };

    usize::max(usize::min(addresses_num as usize, max_addresses_per_request as usize), 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_addresses() {
        let result = calculate_request_chunk_size(10, 0);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_zero_events_per_interval() {
        let result = calculate_request_chunk_size(0, 100);
        assert_eq!(result, 100);
    }

    #[test]
    fn test_small_chunk_size() {
        let expected_chunk_size = (MAX_RESPONSE_BYTES / (EVM_EVENT_SIZE_BYTES * 1)) as usize;
        let result = calculate_request_chunk_size(1, 100);
        assert_eq!(result, expected_chunk_size.min(100).max(1));
    }

    #[test]
    fn test_large_addresses_number() {
        let expected_chunk_size = (MAX_RESPONSE_BYTES / (EVM_EVENT_SIZE_BYTES * 10)) as usize;
        let result = calculate_request_chunk_size(10, 1_000_000);
        assert_eq!(result, expected_chunk_size.min(1_000_000).max(1));
    }

    #[test]
    fn test_large_events_per_interval() {
        let expected_chunk_size = (MAX_RESPONSE_BYTES / (EVM_EVENT_SIZE_BYTES * 1_000_000)) as usize;
        let result = calculate_request_chunk_size(1_000_000, 100);
        assert_eq!(result, expected_chunk_size.min(100).max(1));
    }

    #[test]
    fn test_edge_case_max_bytes_per_request() {
        let bytes_per_address = EVM_EVENT_SIZE_BYTES * 64;
        let expected_chunk_size = (MAX_RESPONSE_BYTES / bytes_per_address) as usize;
        let result = calculate_request_chunk_size(64, 64);
        assert_eq!(result, expected_chunk_size.min(64).max(1));
    }

    #[test]
    fn test_min_result_is_one() {
        let bytes_per_address = EVM_EVENT_SIZE_BYTES * 10;
        let expected_chunk_size = (MAX_RESPONSE_BYTES / bytes_per_address) as usize;
        let result = calculate_request_chunk_size(10, 1);
        assert_eq!(result, expected_chunk_size.min(1).max(1));
    }

    #[test]
    fn test_max_result_respects_addresses_number() {
        let bytes_per_address = EVM_EVENT_SIZE_BYTES * 10;
        let expected_chunk_size = (MAX_RESPONSE_BYTES / bytes_per_address) as usize;
        let result = calculate_request_chunk_size(10, 50);
        assert_eq!(result, expected_chunk_size.min(50).max(1));
    }
}
