use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::STATE;

use candid::{CandidType, Nat, Principal};

#[derive(Error, Debug)]
pub enum BalanceError {
    #[error("balance does not exist")]
    BalanceDoesNotExist,
    #[error("insufficient balance")]
    InsufficientBalance,
}

#[derive(Debug, CandidType, Deserialize, Serialize, Default, Clone)]
pub struct BalanceEntry {
    pub amount: Nat,
}

#[derive(CandidType, Deserialize, Serialize, Default, Clone)]
pub struct Balances {
    pub balances: HashMap<Principal, Nat>,
}

impl Balances {
    pub fn top_up(caller: Principal, amount: Nat) -> Result<(), String> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let balances = &mut state.user_balances.balances;
            let entry = balances.entry(caller).or_insert_with(|| Nat::from(0u32));

            *entry += amount.clone();
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

            Ok(*balance_entry >= amount)
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

            if *balance_entry < amount {
                return Err(BalanceError::InsufficientBalance);
            }

            *balance_entry -= amount.clone();

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
    
            Ok(balance_entry.map_or_else(|| Nat::from(0u32), |entry| entry.clone()))
        })
    }     
}


impl std::fmt::Debug for Balances {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for it in &self.balances {
            ic_cdk::println!("{:?} : {:?}", it.0.to_text(), it.1);
        }        
        Ok(())
    }
}