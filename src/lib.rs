//! HiTrace
//!
//! Safe bindings for the [`HiTrace`] tracing system on OpenHarmony.
//! This crate does nothing if not compiled for OpenHarmony (`target_env = ohos`).
//!
//!
//! # Usage
//!
//! `HiTrace` allows tracing Spans in a synchronous and stack based fashion.
//!
//! ## Examples
//!
//! ##
//! ```
//! # fn OH_HiTrace_StartTrace(_: * const core::ffi::c_char) {}
//! # fn OH_HiTrace_FinishTrace() {}
//! # fn step1() {}
//! # fn step2() {}
//! # use hitrace::{start_trace, finish_trace};
//! # use std::ffi::CString;
//! fn load_website() {
//!     start_trace(&c"step1");
//!     step1();
//!     finish_trace();
//!     start_trace(&CString::new("step2").unwrap());
//!     step2();
//!     finish_trace();
//! }
//! start_trace(&c"LoadingWebsite");
//! load_website();
//! finish_trace();
//! ```
//!
//!
//! [`HiTrace`]: <https://gitee.com/openharmony/hiviewdfx_hitrace>

use std::ffi::CStr;

pub fn start_trace<T: AsRef<CStr>>(name: &T) {
    start_trace_cstr(name.as_ref())
}

#[cfg(target_env = "ohos")]
fn start_trace_cstr(name: &CStr) {
    // SAFETY: We have a valid CStr, which is copied by `OH_HiTrace_StartTrace`.
    unsafe {
        hitrace_sys::OH_HiTrace_StartTrace(name.as_ptr());
    }
}

#[cfg(not(target_env = "ohos"))]
fn start_trace_cstr(_: &CStr) {}

/// Finishes the most recently started trace span
pub fn finish_trace() {
    #[cfg(target_env = "ohos")]
    fn finish_trace_() {
        // Todo: We should check in the OpenHarmony code to make sure that
        // `OH_HiTrace_FinishTrace` does not cause Memory Safety issues, if called
        // without a corresponding previous `OH_HiTrace_StartTrace`.
        unsafe {
            hitrace_sys::OH_HiTrace_FinishTrace();
        }
    }

    #[cfg(not(target_env = "ohos"))]
    fn finish_trace_() {}

    finish_trace_()
}
