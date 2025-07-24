use candid::Principal;
use crate::state::read_state;

pub fn is_orchestrator() -> Result<(), String> {
    let caller = ic_cdk::caller();
    
    read_state(|state| {
        // Orchestrator is always set now, so we just check if caller matches
        if caller == state.orchestrator {
            Ok(())
        } else {
            Err("Access denied: caller is not orchestrator".to_string())
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
