use std::str::FromStr;

use candid::{CandidType, Deserialize, Nat, Principal};
use evm_logs_types::Filter;
use evm_rpc_types::{Hex20, Hex32};
use getrandom::getrandom;

pub const DEFAULT_CYCLES_VALUE: u128 = 4_000_000_000_000;

#[derive(CandidType, Deserialize)]
pub struct EvmLogsInitArgs {
  pub evm_rpc_canister: Principal,
  pub proxy_canister: Principal,
  pub estimate_events_num: u32,
  pub max_response_bytes: u32,
}

#[derive(CandidType, Deserialize)]
pub struct WalletCall128Args {
  pub canister: Principal,
  pub method_name: String,
  pub args: Vec<u8>,
  pub cycles: Nat,
}

#[derive(CandidType, Deserialize)]
pub struct EvmRpcMockedConfig {
  pub evm_logs_canister_id: Principal,
}

pub fn generate_random_filter() -> Filter {
  let mut address_bytes = [0u8; 20]; // Ethereum addresses are 20 bytes long
  let mut topic_bytes = [0u8; 32]; // Topics are 32 bytes long

  getrandom(&mut address_bytes).expect("Failed to generate random address bytes");
  getrandom(&mut topic_bytes).expect("Failed to generate random topic bytes");

  let address = format!("0x{}", hex::encode(address_bytes)); // Convert address to hex string
  let topic = format!("0x{}", hex::encode(topic_bytes)); // Convert topic to hex string

  Filter {
    address: Hex20::from_str(&address).unwrap(),
    topics: Some(vec![vec![Hex32::from_str(&topic).unwrap()]]),
  }
}
