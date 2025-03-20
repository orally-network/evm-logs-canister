use std::path::PathBuf;

use anyhow::{Result, bail};
use candid::Deserialize;
use config::Config;
use serde::Serialize;

const DEFAULT_TEST_CONFIG_PATH: &str = "test_configuration/test_config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct TestConfig {
  pub evm_logs_canister_wasm_path: String,
  pub test_canister_wasm_path: String,
  pub cycles_wallet_wasm_path: String,
  pub proxy_canister_wasm_path: String,
  pub evm_rpc_mocked_wasm_path: String,
}
impl TestConfig {
  pub fn new() -> Result<Self> {
    let path_to_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path_to_workspace_src = path_to_manifest.parent().unwrap();
    let path_to_test_config = PathBuf::from(&path_to_workspace_src).join(DEFAULT_TEST_CONFIG_PATH);
    let path_to_append = path_to_manifest.clone();
    match Config::builder()
      .add_source(config::File::from(path_to_test_config))
      .build()
    {
      Ok(x) => match x.try_deserialize::<TestConfig>() {
        Ok(config) => Ok(TestConfig {
          evm_logs_canister_wasm_path: format!(
            "{}/{}",
            path_to_append.to_str().unwrap(),
            config.evm_logs_canister_wasm_path
          ),
          test_canister_wasm_path: format!(
            "{}/{}",
            path_to_append.to_str().unwrap(),
            config.test_canister_wasm_path
          ),
          cycles_wallet_wasm_path: format!(
            "{}/{}",
            path_to_append.to_str().unwrap(),
            config.cycles_wallet_wasm_path
          ),
          proxy_canister_wasm_path: format!(
            "{}/{}",
            path_to_append.to_str().unwrap(),
            config.proxy_canister_wasm_path
          ),
          evm_rpc_mocked_wasm_path: format!(
            "{}/{}",
            path_to_append.to_str().unwrap(),
            config.evm_rpc_mocked_wasm_path
          ),
        }),
        Err(e) => {
          bail!(
            "Deserialization error in obtaining \
             config, try to check '{path_to_append:?}', err: {e}"
          )
        }
      },
      Err(e) => bail!(
        "Error in obtaining configuration \
         from source '{path_to_append:?}', err: {e}"
      ),
    }
  }
}
