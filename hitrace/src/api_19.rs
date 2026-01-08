#![cfg(target_env = "ohos")]
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents HiTrace output levels for API level 19+.
///
/// Mirrors `HiTrace_Output_Level` from hitrace-sys.
/// https://docs.rs/hitrace-sys/latest/hitrace_sys/struct.HiTrace_Output_Level.html
pub enum HiTraceOutputLevel {
    /// Output level only for debug usage.
    Debug = 0,
    /// Output level for log version usage.
    Info = 1,
    /// Output level for log version usage, with higher priority than Info.
    Critical = 2,
    /// Output level for nolog version usage.
    Commercial = 3,
}

impl From<HiTraceOutputLevel> for hitrace_sys::HiTrace_Output_Level {
    fn from(value: HiTraceOutputLevel) -> Self {
        match value {
            HiTraceOutputLevel::Info => hitrace_sys::HiTrace_Output_Level::HITRACE_LEVEL_INFO,
            HiTraceOutputLevel::Debug => hitrace_sys::HiTrace_Output_Level::HITRACE_LEVEL_DEBUG,
            HiTraceOutputLevel::Critical => {
                hitrace_sys::HiTrace_Output_Level::HITRACE_LEVEL_CRITICAL
            }
            HiTraceOutputLevel::Commercial => {
                hitrace_sys::HiTrace_Output_Level::HITRACE_LEVEL_COMMERCIAL
            }
        }
    }
}

#[cfg(feature = "tracing-rs")]
impl From<tracing_core::Level> for HiTraceOutputLevel {
    fn from(level: tracing_core::Level) -> Self {
        use tracing_core::Level;
        match level {
            Level::TRACE | Level::DEBUG => HiTraceOutputLevel::Debug,
            Level::INFO => HiTraceOutputLevel::Info,
            Level::WARN => HiTraceOutputLevel::Critical,
            Level::ERROR => HiTraceOutputLevel::Commercial,
        }
    }
}
