use std::fmt::Debug;

use hitrace_sys::HiTrace_Output_Level;

#[cfg(feature = "api-19")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
/// Represents HiTrace output levels for API level 19+.
/// Mirrors `HiTrace_Output_Level` from hitrace-sys.
/// 
/// But the actual tracing-core-0.1.36 goes Trace=0, Debug=1, Info=2, Warn=3, Error=4
pub enum HiTraceOutputLevel {
    /// Output level only for debug usage.
    Debug = 0,
    /// Output level for log version usage.
    Info = 1,
    /// Output level for log version usage, with higher priority than Info.
    Critical = 2,
    /// Output level for nolog version usage.
    Commercial = 3,
    // /// Output level for range limit.
    Max = 4, // the OG one is also 3???
}

impl From<HiTrace_Output_Level> for HiTraceOutputLevel {
    fn from(level: HiTrace_Output_Level) -> Self{
        match level {
            HiTrace_Output_Level::HITRACE_LEVEL_DEBUG => HiTraceOutputLevel::Debug,
            HiTrace_Output_Level::HITRACE_LEVEL_INFO => HiTraceOutputLevel::Info,
            HiTrace_Output_Level::HITRACE_LEVEL_CRITICAL => HiTraceOutputLevel::Critical,
            HiTrace_Output_Level::HITRACE_LEVEL_COMMERCIAL => HiTraceOutputLevel::Commercial,
            _ => HiTraceOutputLevel::Max
        }
    }
}

impl From<HiTraceOutputLevel> for HiTrace_Output_Level {
    fn from(value: HiTraceOutputLevel) -> Self {
        match value {
            HiTraceOutputLevel::Info => HiTrace_Output_Level::HITRACE_LEVEL_INFO,
            HiTraceOutputLevel::Debug => HiTrace_Output_Level::HITRACE_LEVEL_DEBUG,
            HiTraceOutputLevel::Critical => HiTrace_Output_Level::HITRACE_LEVEL_CRITICAL,
            HiTraceOutputLevel::Commercial => HiTrace_Output_Level::HITRACE_LEVEL_COMMERCIAL,
            HiTraceOutputLevel::Max => HiTrace_Output_Level::HITRACE_LEVEL_MAX,
        }
    }
}

#[cfg(feature = "tracing-level-conversion")]
impl From<tracing_core::Level> for HiTraceOutputLevel {
    fn from(level: tracing_core::Level) -> Self{
        match level {
            tracing_core::Level::TRACE /* 0 */ => HiTraceOutputLevel::Debug,      //0
            tracing_core::Level::DEBUG /* 1 */ => HiTraceOutputLevel::Info,       //1
            tracing_core::Level::INFO  /* 2 */ => HiTraceOutputLevel::Critical,   //2
            tracing_core::Level::WARN  /* 3 */ => HiTraceOutputLevel::Commercial, //3
            tracing_core::Level::ERROR /* 4 */ => HiTraceOutputLevel::Max,        //4
        }
    }
}