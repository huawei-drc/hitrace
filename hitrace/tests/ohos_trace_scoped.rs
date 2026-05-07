use std::thread::sleep;
use std::time::Duration;

#[test]
fn scoped_trace_emits_markers() {
    println!("hitrace scoped pid={}", std::process::id());

    {
        let _g = hitrace::ScopedTrace::start_trace_str("hitrace_xtask_scoped_default");
        sleep(Duration::from_millis(20));
    }

    #[cfg(all(feature = "api-19", target_env = "ohos"))]
    {
        let _g = hitrace::ScopedTrace::start_trace_ex_str(
            hitrace::api_19::HiTraceOutputLevel::Critical,
            "hitrace_xtask_scoped_critical",
            "scoped=true",
        );
        sleep(Duration::from_millis(20));
    }

    println!("hitrace scoped ok");
}
