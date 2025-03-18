#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {{
        ic_cdk::println!($($arg)*);
    }};
}
