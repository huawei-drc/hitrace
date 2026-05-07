#![cfg(all(feature = "api-19", target_env = "ohos"))]

use std::thread::sleep;
use std::time::Duration;

#[test]
fn emits_api19_markers() {
    println!("hitrace api19 pid={}", std::process::id());

    hitrace::start_trace_ex(
        hitrace::api_19::HiTraceOutputLevel::Info,
        &c"hitrace_xtask_api19_span",
        &c"phase=hitrace_xtask,result=ok",
    );
    sleep(Duration::from_millis(20));
    hitrace::finish_trace_ex(hitrace::api_19::HiTraceOutputLevel::Info);

    println!("hitrace api19 ok");
}
