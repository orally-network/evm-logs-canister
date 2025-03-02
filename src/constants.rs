pub const ETHEREUM_CHAIN_ID: u32 = 1;
pub const BSC_CHAIN_ID: u32 = 56;
pub const POLYGON_CHAIN_ID: u32 = 137;
pub const BASE_CHAIN_ID: u32 = 8453;
pub const OPTIMISM_CHAIN_ID: u32 = 10;
pub const ARBITRUM_CHAIN_ID: u32 = 42161;

// Base cycle cost for an inter-canister call
pub const BASE_CALL_CYCLES: u64 = 260_000;

// Cost per byte for sending and receiving messages
pub const CYCLES_PER_BYTE_SEND: u64 = 1000; // Cycles per byte sent
pub const CYCLES_PER_BYTE_RECEIVE: u64 = 1000; // Cycles per byte received

// Size estimation for Ethereum log event and related data
pub const EVM_EVENT_SIZE_BYTES: u32 = 800; // Bytes for one received EVM event
pub const ETH_ADDRESS_SIZE: u32 = 20; // Size of one Ethereum address
pub const ETH_TOPIC_SIZE: u32 = 32; // Size of one Ethereum topic
