use std::cell::RefCell;
use ic_stable_structures::{Cell, DefaultMemoryImpl};
use crate::types::{OrchestratorState, OrchestratorInitArg};

type Memory = DefaultMemoryImpl;

thread_local! {
    static STATE: RefCell<Cell<OrchestratorState, Memory>> = RefCell::new(
        Cell::init(Memory::default(), OrchestratorState::default())
            .expect("Failed to initialize state")
    );
}

pub fn init_state(arg: OrchestratorInitArg) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let mut orchestrator_state = OrchestratorState::default();
        orchestrator_state.admin = arg.admin;
        orchestrator_state.version = arg.version;
        
        state.set(orchestrator_state).expect("Failed to set initial state");
    });
}

pub fn read_state<R>(f: impl FnOnce(&OrchestratorState) -> R) -> R {
    STATE.with(|state| {
        let state = state.borrow();
        let current_state = state.get();
        f(&current_state)
    })
}

pub fn mutate_state<F, R>(f: F) -> R
where
F: FnOnce(&mut OrchestratorState) -> R
{
    STATE.with(|state| {
        let mut borrowed = state.borrow_mut();
        let mut current_state = borrowed.get().clone();
        let result = f(&mut current_state);
        borrowed.set(current_state).expect("Failed to update state");
        result
    })
}
