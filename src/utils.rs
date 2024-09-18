use std::cell::RefCell;
use ic_cdk::api::time;
use candid::Nat;

thread_local! {
    static SUB_ID_COUNTER: RefCell<Nat> = RefCell::new(Nat::from(0u32));
}

pub fn current_timestamp() -> u64 {
    time()
}

pub fn generate_sub_id() -> Nat {
    SUB_ID_COUNTER.with(|counter| {
        let mut cnt = counter.borrow_mut();
        *cnt += Nat::from(1u32);
        cnt.clone()
    })
}
