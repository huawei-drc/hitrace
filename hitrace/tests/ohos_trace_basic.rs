use std::thread::sleep;
use std::time::Duration;

#[test]
fn emits_basic_markers() {
    println!("hitrace basic pid={}", std::process::id());

    hitrace::start_trace(&c"hitrace_xtask_basic_sync");
    sleep(Duration::from_millis(20));
    hitrace::finish_trace();

    hitrace::trace_metric_str("hitrace_xtask_basic_counter", 7_i32);
    hitrace::trace_metric_saturating_str("hitrace_xtask_basic_saturating", u64::MAX);

    println!("hitrace basic ok");
}
