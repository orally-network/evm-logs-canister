use candid::{CandidType, Deserialize, Nat};
use ic_cdk::api::call::{call, call_with_payment128};
use candid::Principal; // Імпорт для Principal
use std::cell::RefCell;
use std::io::{self, Write};
use candid::Encode;
use evm_rpc_types::{self, Nat256};

pub struct ChainService {
    canister_id: String, 
}

impl ChainService {
    pub fn new(canister_id: String) -> Self {
        ChainService { canister_id }
    }

    pub async fn fetch_logs(&self, from_block: u64, to_block: u64, address: Option<String>) -> Result<Vec<String>, String> {
        let canister_id = Principal::from_text(&self.canister_id)
            .map_err(|e| format!("Invalid canister ID: {:?}", e))?;
    
        let rpc_services = evm_rpc_types::RpcServices::EthMainnet(Some(vec![evm_rpc_types::EthMainnetService::Cloudflare]));
    
        let get_logs_args = evm_rpc_types::GetLogsArgs {
            from_block: Some(evm_rpc_types::BlockTag::Number(Nat256::from(from_block))),
            to_block: Some(evm_rpc_types::BlockTag::Number(Nat256::from(to_block))),
            addresses: address
            .into_iter()
            .map(|addr| addr.parse::<evm_rpc_types::Hex20>().expect("Invalid address format"))
            .collect(),

            topics: None,
        };

        let rpc_config: Option<evm_rpc_types::RpcConfig> = None; 

        let result: Result<(evm_rpc_types::MultiRpcResult<Vec<evm_rpc_types::LogEntry>>,), _> = call_with_payment128(
            canister_id, 
            "eth_getLogs", 
            (rpc_services, rpc_config, get_logs_args),
            1000000000,
        ).await;

        match result {
            Ok((multi_get_logs_result,)) => {
                match multi_get_logs_result {
                    evm_rpc_types::MultiRpcResult::Consistent(evm_rpc_types::RpcResult::Ok(log_entries)) => {
                        let logs: Vec<String> = log_entries.into_iter().map(|log_entry| {
                            format!(
                                "Address: {}, TxHash: {:?}, Block: {:?}, Data: {}",
                                log_entry.address,
                                log_entry.transaction_hash,
                                log_entry.block_number,
                                log_entry.data
                            )
                        }).collect();
                        
                        Ok(logs)  
                    },
                    evm_rpc_types::MultiRpcResult::Consistent(evm_rpc_types::RpcResult::Err(rpc_error)) => {
                        Err(format!("RPC Error: {:?}", rpc_error))
                    },
                    evm_rpc_types::MultiRpcResult::Inconsistent(inconsistent_logs) => {
                        let inconsistent_log_data: Vec<String> = inconsistent_logs.into_iter()
                            .map(|(_service, result)| match result {
                                Ok(log_entries) => log_entries.into_iter().map(|log_entry| {
                                    format!(
                                        "Address: {}, TxHash: {:?}, Block: {:?}, Data: {}",
                                        log_entry.address,
                                        log_entry.transaction_hash,
                                        log_entry.block_number,
                                        log_entry.data
                                    )
                                }).collect::<Vec<String>>().join("\n"),  
                                Err(_) => "Error fetching log".to_string(),
                            })
                            .collect();
                        
                        Ok(inconsistent_log_data)  
                    }
                }
            },
            Err(err) => Err(format!("Error calling canister: {:?}", err)),
        }
    }
    
}
