#[macro_export]
macro_rules! get_state_value {
  ($field:ident) => {{ $crate::STATE.with(|state| state.borrow().$field.clone()) }};
}

#[macro_export]
macro_rules! update_state {
  ($field:ident, $value:expr) => {{
    $crate::STATE.with(|state| {
      state.borrow_mut().$field = $value;
    })
  }};
}

#[macro_export]
macro_rules! log_with_metrics {
    ($($arg:tt)*) => {{
        use $crate::metrics;
        ic_cdk::println!($($arg)*);
        ic_utils::logger::log_message(format!($($arg)*));
        ic_utils::monitor::collect_metrics();

        metrics!(set CYCLES, ic_cdk::api::canister_balance() as u128);
    }};
}
