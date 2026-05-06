use std::ffi::CString;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn emits_hitrace_markers() {
    println!("hitrace smoke pid={}", std::process::id());

    let span = CString::new("hitrace_xtask_sync_span").expect("valid trace name");

    hitrace::start_trace(&span);
    sleep(Duration::from_millis(50));
    hitrace::finish_trace();

    #[cfg(all(feature = "api-19", target_env = "ohos"))]
    {
        let ex_name = CString::new("hitrace_xtask_ex_span").expect("valid trace name");
        let custom_args = CString::new("phase=hitrace_xtask,result=ok").expect("valid trace args");
        hitrace::start_trace_ex(
            hitrace::api_19::HiTraceOutputLevel::Info,
            &ex_name,
            &custom_args,
        );
        sleep(Duration::from_millis(50));
        hitrace::finish_trace_ex(hitrace::api_19::HiTraceOutputLevel::Info);
    }

    hitrace::trace_metric_str("hitrace_xtask_counter", 7_i32);

    println!("hitrace smoke ok");
}
