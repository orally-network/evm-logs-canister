use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::STATE;

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
    pub balances: HashMap<Principal, BalanceEntry>,
}

impl Balances {
    pub fn top_up(caller: Principal, amount: Nat) -> Result<(), String> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let balances = &mut state.user_balances.balances;
            let entry = balances.entry(caller).or_insert_with(|| BalanceEntry {
                amount: Nat::from(0u32),
                nonces: vec![],
            });

            entry.amount += amount.clone();
            Ok(())
        })
    }

    pub fn contains(address: &Principal) -> bool {
        STATE.with(|state| state.borrow().user_balances.balances.contains_key(address))
    }

    pub fn is_sufficient(address: Principal, amount: Nat) -> Result<bool, BalanceError> {
        STATE.with(|state| {
            let state = state.borrow();
            let balance_entry = state
                .user_balances
                .balances
                .get(&address)
                .ok_or(BalanceError::BalanceDoesNotExist)?;

            Ok(balance_entry.amount >= amount)
        })
    }

    pub fn reduce(address: &Principal, amount: Nat) -> Result<(), BalanceError> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let balance_entry = state
                .user_balances
                .balances
                .get_mut(address)
                .ok_or(BalanceError::BalanceDoesNotExist)?;

            if balance_entry.amount < amount {
                return Err(BalanceError::InsufficientBalance);
            }

            balance_entry.amount -= amount.clone();

            Ok(())
        })
    }

    pub fn get_balance(principal: &Principal) -> Result<Nat, BalanceError> {
        STATE.with(|state| {
            let state = state.borrow();
            let balance_entry = state
                .user_balances
                .balances
                .get(principal);
    
            Ok(balance_entry.map_or_else(|| Nat::from(0u32), |entry| entry.amount.clone()))
        })
    }     
}