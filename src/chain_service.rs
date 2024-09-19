use candid::{CandidType, Deserialize, Nat};
use ic_cdk::api::call::call;
use candid::Principal; // Імпорт для Principal
use std::cell::RefCell;
use std::io::{self, Write};
use candid::Encode;


// All there structs are to be removed after exporting evm-epc-types crate to the project 
#[derive(CandidType, Deserialize, Debug)]
enum EthMainnetService {
    Cloudflare,
}

#[derive(CandidType, Deserialize, Debug)]
enum BlockTag {
    Number(Nat),
}

#[derive(CandidType, Deserialize, Debug)]
struct GetLogsArgs {
    from_block: Option<BlockTag>,
    to_block: Option<BlockTag>,
    addresses: Vec<String>,
    topics: Option<Vec<Vec<String>>>,
}

#[derive(CandidType, Deserialize, Debug)]
struct RpcConfig {
    responseSizeEstimate: Option<Nat>,
}

pub struct ChainService {
    canister_id: String, 
}

#[derive(CandidType, Deserialize, Debug)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(CandidType, Deserialize, Debug)]
struct LogEntry {
    transactionHash: Option<String>,
    blockNumber: Option<Nat>,
    data: String,
    topics: Vec<String>,
    address: String,
    logIndex: Option<Nat>,
}

#[derive(CandidType, Deserialize, Debug)]
enum GetLogsResult {
    Ok(Vec<LogEntry>),
    Err(RpcError),
}

#[derive(CandidType, Deserialize, Debug)]
enum RpcServices {
    EthMainnet(Option<Vec<EthMainnetService>>),
}

#[derive(CandidType, Deserialize, Debug)]
enum MultiGetLogsResult {
    Consistent(GetLogsResult),
    Inconsistent(Vec<(RpcServices, GetLogsResult)>),
}


impl ChainService {
    pub fn new(canister_id: String) -> Self {
        ChainService { canister_id }
    }

    pub async fn fetch_logs(&self, from_block: u64, to_block: u64, address: Option<String>) -> Result<Vec<String>, String> {
        let canister_id = Principal::from_text(&self.canister_id)
            .map_err(|e| format!("Invalid canister ID: {:?}", e))?;
    
        // RpcServices: EthMainnet з Cloudflare
        let rpc_services = RpcServices::EthMainnet(Some(vec![EthMainnetService::Cloudflare]));
    
        let get_logs_args = GetLogsArgs {
            from_block: Some(BlockTag::Number(Nat::from(from_block))),
            to_block: Some(BlockTag::Number(Nat::from(to_block))),
            addresses: match address {
                Some(addr) => vec![addr],
                None => vec![],
            },
            topics: None,
        };

        let rpc_config: Option<RpcConfig> = None; 

        let result: Result<(MultiGetLogsResult,), _> = call(
            canister_id, 
            "eth_getLogs", 
            (rpc_services, rpc_config, get_logs_args)
        ).await;

        match result {
            Ok((multi_get_logs_result,)) => {
                match multi_get_logs_result {
                    MultiGetLogsResult::Consistent(GetLogsResult::Ok(log_entries)) => {
                        let logs: Vec<String> = log_entries.into_iter().map(|entry| entry.data).collect();
                        Ok(logs)
                    },
                    MultiGetLogsResult::Consistent(GetLogsResult::Err(rpc_error)) => {
                        Err(format!("RPC Error: Code: {}, Message: {}", rpc_error.code, rpc_error.message))
                    },
                    MultiGetLogsResult::Inconsistent(_) => {
                        Err("Inconsistent logs found.".to_string())
                    }
                }
            },
            Err(err) => Err(format!("Error calling canister: {:?}", err)),
        }
    }
    
}
