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

#[cfg(feature = "api-19")]
pub mod api_19;

pub fn start_trace<T: AsRef<CStr>>(name: &T) {
    start_trace_cstr(name.as_ref())
}

#[cfg(all(
    feature = "api-19",
    target_env = "ohos",
    not(feature = "max_level_off")
))]
pub fn start_trace_ex<T: AsRef<CStr>, U: AsRef<CStr>>(
    level: api_19::HiTraceOutputLevel,
    name: &T,
    custom_args: &U,
) {
    start_trace_ex_cstr(level, name.as_ref(), custom_args.as_ref())
}

#[cfg(all(target_env = "ohos", not(feature = "max_level_off")))]
fn start_trace_cstr(name: &CStr) {
    // SAFETY: We have a valid CStr, which is copied by `OH_HiTrace_StartTrace`.
    unsafe {
        hitrace_sys::OH_HiTrace_StartTrace(name.as_ptr());
    }
}

#[cfg(all(
    feature = "api-19",
    target_env = "ohos",
    not(feature = "max_level_off")
))]
fn start_trace_ex_cstr(level: api_19::HiTraceOutputLevel, name: &CStr, custom_args: &CStr) {
    // SAFETY: We have a valid CStr, which is copied by `OH_HiTrace_StartTrace`.
    // The custom_args is a single String of comma separated key=value pairs.
    unsafe {
        hitrace_sys::OH_HiTrace_StartTraceEx(level.into(), name.as_ptr(), custom_args.as_ptr());
    }
}

#[cfg(any(not(target_env = "ohos"), feature = "max_level_off"))]
fn start_trace_cstr(_: &CStr) {}

/// Finishes the most recently started trace span
pub fn finish_trace() {
    #[cfg(all(target_env = "ohos", not(feature = "max_level_off")))]
    fn finish_trace_() {
        // Todo: We should check in the OpenHarmony code to make sure that
        // `OH_HiTrace_FinishTrace` does not cause Memory Safety issues, if called
        // without a corresponding previous `OH_HiTrace_StartTrace`.
        unsafe {
            hitrace_sys::OH_HiTrace_FinishTrace();
        }
    }

    #[cfg(any(not(target_env = "ohos"), feature = "max_level_off"))]
    fn finish_trace_() {}

    finish_trace_()
}

/// Wrapper function for `OH_HiTrace_CountTrace` with a CStr name parameter
#[cfg(all(target_env = "ohos", not(feature = "max_level_off")))]
fn trace_metric_cstr(name: &CStr, count: i64) {
    unsafe {
        hitrace_sys::OH_HiTrace_CountTrace(name.as_ptr(), count);
    }
}

#[cfg(any(not(target_env = "ohos"), feature = "max_level_off"))]
fn trace_metric_cstr(_: &CStr, _: i64) {}

/// A function to log a count trace event with a name and any integer count that implements `Into<i64>`.
fn trace_metric<T: AsRef<CStr>, C: Into<i64>>(name: &T, count: C) {
    trace_metric_cstr(name.as_ref(), count.into());
}

/// Logs a count trace event with a `&str` name and an integer count that can be converted to `i64`.
///
/// # Arguments
/// * `name` - The name of the event.
/// * `count` - The integer count, convertible to `i64`.
///
/// # Panics
/// Panics if `name` contains a null byte.
pub fn trace_metric_str<C: Into<i64>>(name: &str, count: C) {
    let c_string = CString::new(name).expect("Failed to convert to CString");
    trace_metric(&c_string, count.into());
}

/// Logs a count trace event with a name and an integer count, using saturating conversion to `i64`.
///
/// # Arguments
/// * `name` - The name of the event (as `CStr` or `CString`).
/// * `count` - The integer count, implementing `SaturatingIntoI64`.
pub fn trace_metric_saturating<T: AsRef<CStr>>(name: &T, count: impl SaturatingIntoI64) {
    trace_metric_cstr(name.as_ref(), count.saturating_into());
}

/// Logs a count trace event with a `&str` name and an integer count, using saturating conversion to `i64`.
///
/// # Arguments
/// * `name` - The name of the event.
/// * `count` - The integer count, implementing `SaturatingIntoI64`.
///
/// # Panics
/// Panics if `name` contains a null byte.
pub fn trace_metric_saturating_str(name: &str, count: impl SaturatingIntoI64) {
    let c_string = CString::new(name).expect("Failed to convert to CString");
    trace_metric_saturating(&c_string, count);
}

pub trait SaturatingIntoI64 {
    fn saturating_into(self) -> i64;
}

impl SaturatingIntoI64 for u64 {
    fn saturating_into(self) -> i64 {
        self.min(i64::MAX as u64) as i64
    }
}

impl SaturatingIntoI64 for i128 {
    fn saturating_into(self) -> i64 {
        if self > i64::MAX as i128 {
            i64::MAX
        } else if self < i64::MIN as i128 {
            i64::MIN
        } else {
            self as i64
        }
    }
}

impl SaturatingIntoI64 for u128 {
    fn saturating_into(self) -> i64 {
        self.min(i64::MAX as u128) as i64
    }
}

impl SaturatingIntoI64 for usize {
    fn saturating_into(self) -> i64 {
        self.min(i64::MAX as usize) as i64
    }
}

impl SaturatingIntoI64 for isize {
    fn saturating_into(self) -> i64 {
        if self > i64::MAX as isize {
            i64::MAX
        } else if self < i64::MIN as isize {
            i64::MIN
        } else {
            self as i64
        }
    }
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
        #[cfg(any(not(target_env = "ohos"), feature = "max_level_off"))]
        let _ = name_with_null;
        // SAFETY: The User promises that the `str` slice is a valid null-terminated C-style string.
        #[cfg(all(target_env = "ohos", not(feature = "max_level_off")))]
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
