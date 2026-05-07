use std::thread::sleep;
use std::time::Duration;

use hitrace_macro::trace_fn;

#[trace_fn]
fn hitrace_xtask_macro_target() {
    sleep(Duration::from_millis(20));
}

#[test]
fn macro_instruments_function() {
    println!("hitrace macro pid={}", std::process::id());
    hitrace_xtask_macro_target();
    println!("hitrace macro ok");
}
