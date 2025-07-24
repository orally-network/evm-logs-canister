use candid::{Principal};
use pocket_ic::{PocketIc};

fn get_evm_rpc_wasm() -> Vec<u8> {
    std::fs::read("assets/evm_rpc.wasm.gz")
        .expect("Failed to read EVM RPC WASM file from assets/evm_rpc.wasm.gz")
}

pub fn setup_evm_rpc_canister(pic: &PocketIc) -> Principal {
    // Use the mainnet EVM RPC canister ID as suggested
    let _evm_rpc_id = Principal::from_text("7hfb6-caaaa-aaaar-qadga-cai").unwrap();

    // Create canister (PocketIC doesn't support specific IDs, so we create a new one)
    let canister_id = pic.create_canister();

    pic.add_cycles(canister_id, 10_000_000_000_000); // 10T cycles for EVM RPC

    // Install EVM RPC canister with proper InstallArgs
    let evm_rpc_wasm = get_evm_rpc_wasm();

    // Create InstallArgs for EVM RPC canister
    #[derive(candid::CandidType, serde::Deserialize)]
    struct InstallArgs {
        demo: Option<bool>,
        manage_api_keys: Option<Vec<Principal>>,
        log_filter: Option<LogFilter>,
        override_provider: Option<OverrideProvider>,
        nodes_in_subnet: Option<u32>,
    }

    #[derive(candid::CandidType, serde::Deserialize)]
    enum LogFilter {
        ShowAll,
        HideAll,
        ShowPattern(String),
        HidePattern(String),
    }

    #[derive(candid::CandidType, serde::Deserialize)]
    struct OverrideProvider {
        override_url: Option<RegexSubstitution>,
    }

    #[derive(candid::CandidType, serde::Deserialize)]
    struct RegexSubstitution {
        pattern: String,
        replacement: String,
    }

    let install_args = InstallArgs {
        demo: Some(true), // Enable demo mode for testing
        manage_api_keys: None,
        log_filter: Some(LogFilter::ShowAll),
        override_provider: None,
        nodes_in_subnet: Some(1),
    };

    let init_arg = candid::encode_one(install_args).unwrap();

    pic.install_canister(canister_id, evm_rpc_wasm, init_arg, None);

    canister_id
}