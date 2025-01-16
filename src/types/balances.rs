use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use candid::{CandidType, Nat, Principal};

#[derive(Error, Debug)]
pub enum BalanceError {
    #[error("balance already exists")]
    BalanceAlreadyExists,
    #[error("balance does not exist")]
    BalanceDoesNotExist,
    #[error("nonce already used")]
    NonceAlreadyUsed,
    #[error("insufficient balance")]
    InsufficientBalance,
}

#[derive(Error, Debug)]
pub enum DepositError {
    #[error("balance error: {0}")]
    BalanceError(#[from] BalanceError),
    #[error("tx is not finalized")]
    TxNotFinalized,
    #[error("tx has failed")]
    TxFailed,
    #[error("caller is not tx sender")]
    CallerIsNotTxSender,
    #[error("tx without receiver")]
    TxWithoutReceiver,
    #[error("caller is not the sender of the transfer")]
    CallerIsNotTransferSender,
    #[error("token receiver is not the canister eth address")]
    TokenReceiverIsNotCanisterEthAddress,
    #[error("invalid transfer event")]
    InvalidTransferEvent,
    #[error("This chain is not allowed for deposit")]
    ChainNotAllowed,
}


#[derive(Debug, CandidType, Deserialize, Serialize, Default, Clone)]
pub struct BalanceEntry {
    pub amount: Nat,
    pub nonces: Vec<Nat>,
}

#[derive(CandidType, Deserialize, Serialize, Default, Clone, Debug)]
pub struct Balances {
    pub balances: HashMap<Principal, Nat>,
}