// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::{
    kcall::pm,
    pm::MutexAddress,
};
use ::core::{
    fmt,
    fmt::Write,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum log level.
pub const MAX_LEVEL: LogLevel = if cfg!(feature = "trace") {
    LogLevel::Trace
} else if cfg!(feature = "debug") {
    LogLevel::Debug
} else if cfg!(feature = "info") {
    LogLevel::Info
} else if cfg!(feature = "warn") {
    LogLevel::Warn
} else if cfg!(feature = "error") {
    LogLevel::Error
} else {
    LogLevel::Panic
};

//==================================================================================================
// Structures
//==================================================================================================

/// A formatter object
pub struct Logger;

/// Log levels.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Panic,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl core::fmt::Debug for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Panic => write!(f, "PANIC"),
        }
    }
}

//==================================================================================================
// Global Variables
//==================================================================================================

static MUTEX: usize = 0;

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = ::sys::kcall::debug::debug(s.as_ptr(), s.len());
        Ok(())
    }
}

impl Logger {
    pub fn get(tag: &str, level: LogLevel) -> Self {
        pm::lock_mutex(MutexAddress::from(&MUTEX as *const usize as usize)).unwrap();
        let mut ret: Self = Self;
        let _ = write!(&mut ret, "[{:?}][{}] ", level, tag);
        ret
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        pm::unlock_mutex(MutexAddress::from(&MUTEX as *const usize as usize)).unwrap();
    }
}
