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

use std::ffi::{CStr, CString};
use std::marker::PhantomData;

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

pub struct ScopedTrace {
    // Remove Send / Sync, since the trace needs to be finished on the same thread.
    phantom_data: PhantomData<*mut u8>,
}

impl ScopedTrace {
    /// Starts a new ScopedTrace, which ends when the returned object is dropped.
    ///
    /// Keep in mind the general limitations of HiTrace, where a call to
    /// finish_trace will end the span of the most recently started trace.
    /// Users should try not to mix `ScopedTrace` with manual calls to `finish_trace()`,
    /// and should avoid passing the `ScopedTrace` object around.
    #[must_use]
    pub fn start_trace<T: AsRef<CStr>>(name: &T) -> Self {
        start_trace(name);
        Self {
            phantom_data: PhantomData,
        }
    }

    /// Like `start_trace()` but accepts a `&str`.
    ///
    /// # Panic
    ///
    /// Panics if the provided name can't be converted into a CString.
    #[must_use]
    pub fn start_trace_str(name: &str) -> Self {
        Self::start_trace(&CString::new(name).expect("Contained null-byte"))
    }

    // A hidden function, which should only be used by hitrace-macro,
    // until `c""` syntax is available.
    #[doc(hidden)]
    pub unsafe fn _start_trace_str_with_null(name_with_null: &str) -> Self {
        #[cfg(not(target_env = "ohos"))]
        let _ = name_with_null;
        // SAFETY: The User promises that the `str` slice is a valid null-terminated C-style string.
        #[cfg(target_env = "ohos")]
        unsafe {
            hitrace_sys::OH_HiTrace_StartTrace(name_with_null.as_ptr());
        }
        Self {
            phantom_data: PhantomData,
        }
    }
}

impl Drop for ScopedTrace {
    fn drop(&mut self) {
        finish_trace()
    }
}

#[cfg(test)]
mod test {
    use static_assertions::assert_not_impl_any;

    use crate::ScopedTrace;

    assert_not_impl_any!(ScopedTrace: Send, Sync);
}
