use ic_cdk::api::call::call;
use candid::Principal; // Імпорт для Principal
use std::cell::RefCell;

pub struct ChainService {
    canister_id: String, // ID каністри EVM RPC
}

impl ChainService {
    pub fn new(canister_id: String) -> Self {
        ChainService { canister_id }
    }

    pub async fn fetch_logs(&self, from_block: u64, to_block: u64, address: Option<String>) -> Result<Vec<String>, String> {
        let method = "eth_getLogs";
        let params = if let Some(addr) = address {
            format!(
                "[{{\"fromBlock\": \"0x{:x}\", \"toBlock\": \"0x{:x}\", \"address\": \"{}\"}}]",
                from_block, to_block, addr
            )
        } else {
            format!(
                "[{{\"fromBlock\": \"0x{:x}\", \"toBlock\": \"0x{:x}\"}}]",
                from_block, to_block
            )
        };

        let json_rpc_request = format!(
            "{{\"jsonrpc\": \"2.0\", \"method\": \"{}\", \"params\": {}, \"id\": 1}}",
            method, params
        );

        // String => Principal
        let principal_id = Principal::from_text(&self.canister_id).map_err(|e| format!("Invalid canister ID: {:?}", e))?;

        // Call EVM RPC canister
        let result: Result<(String,), _> = call(
            principal_id,
            "request",
            (("variant {Chain=0x1}".to_string(), json_rpc_request, 1000),),
        )
        .await;

        // Process result
        match result {
            Ok((response,)) => {
                Ok(vec![response])
            }
            Err(err) => Err(format!("Error fetching logs: {:?}", err)),
        }

    }
}
