use candid::Principal;
use ic_cdk::api::management_canister::main::{canister_status, CanisterIdRecord};
use crate::state::read_state;

pub async fn is_controller() -> Result<(), String> {
    let caller = ic_cdk::caller();
    let canister_id = ic_cdk::api::id();
    
    // Get canister status to check controllers
    let status_result = canister_status(CanisterIdRecord { canister_id }).await;
    
    match status_result {
        Ok((status,)) => {
            if status.settings.controllers.contains(&caller) {
                Ok(())
            } else {
                Err("Access denied: caller is not a controller of this canister".to_string())
            }
        }
        Err(e) => Err(format!("Failed to check canister controllers: {:?}", e)),
    }
}

pub fn is_admin() -> Result<(), String> {
    let caller = ic_cdk::caller();
    
    read_state(|state| {
        if state.admin == caller {
            Ok(())
        } else {
            Err("Access denied: caller is not admin".to_string())
        }
    })
}

pub fn is_not_anonymous() -> Result<(), String> {
    let caller = ic_cdk::caller();
    
    if caller == Principal::anonymous() {
        Err("Access denied: anonymous caller not allowed".to_string())
    } else {
        Ok(())
    }
}
