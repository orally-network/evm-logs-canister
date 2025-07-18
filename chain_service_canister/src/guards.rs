use candid::Principal;
use crate::state::read_state;

pub fn can_set_orchestrator() -> Result<(), String> {
    let caller = ic_cdk::caller();
    
    if caller == Principal::anonymous() {
        return Err("Access denied: anonymous caller not allowed".to_string());
    }
    
    read_state(|state| {
        // Allow setting orchestrator if none is set, or if caller is current orchestrator
        match state.orchestrator {
            None => Ok(()),
            Some(orchestrator) => {
                if caller == orchestrator {
                    Ok(())
                } else {
                    Err("Access denied: only orchestrator can change orchestrator".to_string())
                }
            }
        }
    })
}

pub fn is_orchestrator() -> Result<(), String> {
    let caller = ic_cdk::caller();
    
    read_state(|state| {
        match state.orchestrator {
            Some(orchestrator) => {
                if caller == orchestrator {
                    Ok(())
                } else {
                    Err("Access denied: caller is not orchestrator".to_string())
                }
            }
            None => Err("Access denied: no orchestrator set".to_string()),
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
