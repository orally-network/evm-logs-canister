use evm_logs_types::EventNotification;
use ic_web3_rs::ethabi::{ParamType, decode};

use super::{read_contract::SolidityToken, utils::extract_data_bytes};

/// Decodes swap event data.
/// It expects the data to represent:
/// int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick.
pub fn swap_event_data_decoder(notification: &EventNotification) -> Result<Vec<SolidityToken>, String> {
  let data = extract_data_bytes(notification)?;

  let param_types = vec![
    ParamType::Int(256), // amount0
    ParamType::Int(256), // amount1
    ParamType::Int(256), // sqrtPriceX96
    ParamType::Int(256), // liquidity
    ParamType::Int(256), // tick
  ];

  let decoded_tokens = decode(&param_types, &data).map_err(|e| format!("Decoding error: {:?}", e))?;

  let result = decoded_tokens.into_iter().map(SolidityToken::from).collect();
  Ok(result)
}

/// Decodes Ethereum synchronization event data.
pub fn ethereum_sync_decoder(notification: &EventNotification) -> Result<Vec<SolidityToken>, String> {
  let data = extract_data_bytes(notification)?;

  let param_types = vec![ParamType::Address, ParamType::Address];

  let decoded_tokens = decode(&param_types, &data).map_err(|e| format!("Decoding error: {:?}", e))?;

  let result = decoded_tokens.into_iter().map(SolidityToken::from).collect();
  Ok(result)
}

pub fn primex_deposit_decoder(notification: &EventNotification) -> Result<Vec<SolidityToken>, String> {
  let data = extract_data_bytes(notification)?;

  let param_types = vec![
    ParamType::Int(256), // amount
  ];

  let decoded_tokens = decode(&param_types, &data).map_err(|e| format!("Decoding error: {:?}", e))?;

  let result = decoded_tokens.into_iter().map(SolidityToken::from).collect();
  Ok(result)
}

pub fn chainfusion_deposit_decoder(notification: &EventNotification) -> Result<Vec<SolidityToken>, String> {
  let data = extract_data_bytes(notification)?;

  let param_types = vec![
    ParamType::Int(256), // amount
  ];

  let decoded_tokens = decode(&param_types, &data).map_err(|e| format!("Decoding error: {:?}", e))?;

  let result = decoded_tokens.into_iter().map(SolidityToken::from).collect();
  Ok(result)
}

pub fn mainnet_uniswap_exchange_1(notification: &EventNotification) -> Result<Vec<SolidityToken>, String> {
  let data = extract_data_bytes(notification)?;

  let param_types = vec![
    ParamType::Address,  //index_topic_1
    ParamType::Address,  // index_topic_2
    ParamType::Int(256), // amount0
    ParamType::Int(256), // amount1
    ParamType::Int(160), // sqrtPriceX96
    ParamType::Int(128), // liquidity
    ParamType::Int(24),  // tick
  ];

  let decoded_tokens = decode(&param_types, &data).map_err(|e| format!("Decoding error: {:?}", e))?;

  let result = decoded_tokens.into_iter().map(SolidityToken::from).collect();
  Ok(result)
}

pub fn mainnet_fantom_token(notification: &EventNotification) -> Result<Vec<SolidityToken>, String> {
  let data = extract_data_bytes(notification)?;

  let param_types = vec![
    ParamType::Address,  // index_topic_1
    ParamType::Address,  // index_topic_2
    ParamType::Int(256), // value
  ];

  let decoded_tokens = decode(&param_types, &data).map_err(|e| format!("Decoding error: {:?}", e))?;

  let result = decoded_tokens.into_iter().map(SolidityToken::from).collect();
  Ok(result)
}

pub fn curve_token_exchange_decoder(notification: &EventNotification) -> Result<Vec<SolidityToken>, String> {
  let data = extract_data_bytes(notification)?;

  let param_types = vec![
    ParamType::Int(256), // sold_id
    ParamType::Int(256), // tokens_sold
    ParamType::Int(256), // bought_id
    ParamType::Int(256), // tokens_bought
  ];

  let decoded_tokens = decode(&param_types, &data).map_err(|e| format!("Decoding error: {:?}", e))?;

  let result = decoded_tokens.into_iter().map(SolidityToken::from).collect();
  Ok(result)
}
