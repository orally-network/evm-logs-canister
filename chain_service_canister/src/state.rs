use std::cell::RefCell;
use ic_stable_structures::{Cell, DefaultMemoryImpl};
use crate::types::{ChainServiceState, ChainConfig};

type Memory = DefaultMemoryImpl;

thread_local! {
    static STATE: RefCell<Cell<ChainServiceState, Memory>> = RefCell::new(
        Cell::init(Memory::default(), ChainServiceState::default())
            .expect("Failed to initialize state")
    );
}

pub fn init_state(config: ChainConfig) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let mut chain_service_state = ChainServiceState::default();
        chain_service_state.config = config;
        
        state.set(chain_service_state).expect("Failed to set initial state");
    });
}

pub fn read_state<R>(f: impl FnOnce(&ChainServiceState) -> R) -> R {
    STATE.with(|state| {
        let state = state.borrow();
        let current_state = state.get();
        f(&current_state)
    })
}

pub fn mutate_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut ChainServiceState) -> R
{
    STATE.with(|state| {
        let mut borrowed = state.borrow_mut();
        let mut current_state = borrowed.get().clone();
        let result = f(&mut current_state);
        borrowed.set(current_state).expect("Failed to update state");
        result
    })
}
