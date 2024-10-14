use std::cell::RefCell;
use ic_cdk::api::time;
use candid::Nat;
use num_traits::ToPrimitive;

use evm_rpc_canister_types::{
    BlockTag, EvmRpcCanister, GetBlockByNumberResult, MultiGetBlockByNumberResult, RpcServices,
};

thread_local! {
    static SUB_ID_COUNTER: RefCell<Nat> = RefCell::new(Nat::from(0u32));
}

pub fn current_timestamp() -> u64 {
    time()
}

pub async fn get_latest_block_number(
    evm_rpc: &EvmRpcCanister,
    rpc_providers: RpcServices,
) -> Result<u64, String> {
    let cycles = 10_000_000_000;

    let block_tag = BlockTag::Latest;

    let (result,) = evm_rpc
        .eth_get_block_by_number(rpc_providers.clone(), None, block_tag, cycles)
        .await
        .map_err(|e| format!("Call failed: {:?}", e))?;

    match result {
        MultiGetBlockByNumberResult::Consistent(res) => match res {
            GetBlockByNumberResult::Ok(block) => {
                let block_number = block.number.0.to_u64().ok_or("Failed to convert block number to u64")?;
                Ok(block_number)
            }
            GetBlockByNumberResult::Err(err) => Err(format!("RPC error: {:?}", err)),
        },
        MultiGetBlockByNumberResult::Inconsistent(_) => {
            Err("RPC providers gave inconsistent results".to_string())
        }
    }
}
