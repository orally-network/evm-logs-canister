use candid::{CandidType, Deserialize, Nat};
use ic_cdk::api::call::{call, call_with_payment128};
use candid::Principal; // Імпорт для Principal
use std::cell::RefCell;
use std::io::{self, Write};
use candid::Encode;
// use evm_rpc_types::{self, Nat256};


use evm_rpc_canister_types::{
    BlockTag, EthMainnetService, GetLogsArgs, EvmRpcCanister, GetBlockByNumberResult, GetLogsResult, HttpOutcallError, MultiGetBlockByNumberResult, MultiGetLogsResult, RejectionCode, RpcError, RpcServices, EVM_RPC
};


pub struct ChainService {
    canister_id: String, 
    evm_rpc: EvmRpcCanister,
}

impl ChainService {
    pub fn new(canister_id: String) -> Self {
        let principal = Principal::from_text("bd3sg-teaaa-aaaaa-qaaba-cai").unwrap();
        let evm_rpc = EvmRpcCanister(principal);

        ChainService { canister_id, evm_rpc }
    }
    pub async fn fetch_logs(&self, from_block: u64, to_block: u64, address: Option<String>) -> Result<Vec<String>, String> {


        let rpc_providers = RpcServices::EthMainnet(Some(vec![EthMainnetService::Alchemy]));


        let get_logs_args = GetLogsArgs {
            fromBlock: Some(BlockTag::Number(Nat::from(from_block))),
            toBlock: Some(BlockTag::Number(Nat::from(to_block))),
            addresses: address
            .into_iter()
            // .map(|addr| addr.parse::<Hex20>().expect("Invalid address format"))     // TODO
            .collect(),

            topics: None,
        };

        let cycles = 10_000_000_000;
        let (result,) = self.evm_rpc
            .eth_get_logs(rpc_providers, None, get_logs_args, cycles)
            .await
            .expect("Call failed");

        match result {
            MultiGetLogsResult::Consistent(r) => match r {
                GetLogsResult::Ok(block) => {
                    let log_strings: Vec<String> = block.into_iter().map(|log_entry| {
                        format!(
                            "Address: {}, TxHash: {:?}, Block: {:?}, Data: {}",
                            log_entry.address,
                            log_entry.transactionHash,
                            log_entry.blockNumber,
                            log_entry.data
                        )
                    }).collect();
                    Ok(log_strings)
                },
                GetLogsResult::Err(err) => Err(format!("{:?}", err)),
            },
            MultiGetLogsResult::Inconsistent(_) => {
                Err("RPC providers gave inconsistent results".to_string())
            }
        }
        
    }
}


