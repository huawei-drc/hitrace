use std::sync::atomic::Ordering::Relaxed;

use hitrace_macro::trace_fn;

mod hitrace {
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering::Relaxed;

    pub struct ScopedTrace {}

    pub static TEST_STATE: AtomicUsize = AtomicUsize::new(0);

    impl ScopedTrace {
        pub unsafe fn _start_trace_str_with_null(name_with_null: &str) -> Self {
            let last_byte = name_with_null.as_bytes().last().expect("empty name");
            assert_eq!(*last_byte, 0, "Last byte must be null");
            TEST_STATE
                .compare_exchange(0, 1, Relaxed, Relaxed)
                .expect("Test already started??");
            Self {}
        }
    }

    impl Drop for ScopedTrace {
        fn drop(&mut self) {
            TEST_STATE
                .compare_exchange(1, 2, Relaxed, Relaxed)
                .expect("Test should not have ended yet");
        }
    }
}

#[trace_fn]
fn do_something_and_measure() {
    assert_eq!(
        hitrace::TEST_STATE.load(Relaxed),
        1,
        "Test should have started and not ended yet"
    )
}

#[test]
fn check_instrumentation() {
    assert_eq!(
        hitrace::TEST_STATE.load(Relaxed),
        0,
        "Tracing code should not have run yet"
    );
    do_something_and_measure();
    assert_eq!(
        hitrace::TEST_STATE.load(Relaxed),
        2,
        "Tracing span should have ended"
    );
}
