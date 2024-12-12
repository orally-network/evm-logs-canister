use ic_web3_rs::ethabi::{decode, ParamType};
use super::read_contract::SolidityToken;

pub fn decode_swap_event_data(data: Vec<u8>) -> Result<Vec<SolidityToken>, String> {
    // index_topic_1 address sender, index_topic_2 address recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick
    
    let param_types = vec![
        ParamType::Int(256), // amount0
        ParamType::Int(256), // amount1
        ParamType::Int(256), // sqrtPriceX96
        ParamType::Int(256), // liquidity
        ParamType::Int(256),   // tick
    ];

    let decoded_tokens = decode(&param_types, &data)
        .map_err(|e| format!("Decoding error: {:?}", e))?;

    let result = decoded_tokens.into_iter().map(SolidityToken::from).collect();
    Ok(result)
}