use crate::STATE;
use candid::{Nat, Principal};


// Internal function to top up balance
pub fn _top_up_balance(caller: Principal, amount: Nat) -> Result<(), String> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let balances = &mut state.user_balances.balances;
        let entry = balances.entry(caller).or_insert_with(|| Nat::from(0u32));
        *entry += amount.clone();
        Ok(())
    })
}