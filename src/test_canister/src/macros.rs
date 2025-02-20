#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        ic_cdk::println!($($arg)*);
    }};
}
