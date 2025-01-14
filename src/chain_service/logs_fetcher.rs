use candid::Nat;
use evm_rpc_canister_types::{
    BlockTag, EvmRpcCanister, GetLogsArgs, GetLogsResult, LogEntry, MultiGetLogsResult, RpcServices,
};

pub async fn fetch_logs(
    evm_rpc: &EvmRpcCanister,
    rpc_providers: &RpcServices,
    from_block: u64,
    addresses: Option<Vec<String>>,
    topics: Option<Vec<Vec<String>>>,
) -> Result<Vec<LogEntry>, String> {
    let get_logs_args = GetLogsArgs {
        fromBlock: Some(BlockTag::Number(Nat::from(from_block))),
        toBlock: Some(BlockTag::Latest),
        addresses: addresses.unwrap_or_default(),
        topics,
    };
    // TODO figure out how many requests we need to fit the max response size(divide addresses array in smaller chunks) 
    let cycles = 10_000_000_000;
    let (result,) = evm_rpc
        .eth_get_logs(rpc_providers.clone(), None, get_logs_args, cycles)
        .await
        .map_err(|e| format!("Call failed: {:?}", e))?;

    match result {
        MultiGetLogsResult::Consistent(r) => match r {
            GetLogsResult::Ok(logs) => Ok(logs),
            GetLogsResult::Err(err) => Err(format!("{:?}", err)),
        },
        MultiGetLogsResult::Inconsistent(_) => {
            Err("RPC providers gave inconsistent results".to_string())
        }
    }
}
