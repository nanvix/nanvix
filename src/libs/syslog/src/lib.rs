// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    fmt,
    fmt::Write,
};

//==================================================================================================
// Macros
//==================================================================================================

#[macro_export]
macro_rules! trace{
    ( $($arg:tt)* ) => ({
		if $crate::MAX_LEVEL >= $crate::LogLevel::Trace {
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get(module_path!(), $crate::LogLevel::Trace),
                $($arg)*
            );
        }
    })
}

#[macro_export]
macro_rules! debug{
    ( $($arg:tt)* ) => ({
		if $crate::MAX_LEVEL >= $crate::LogLevel::Debug{
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get(module_path!(), $crate::LogLevel::Debug),
                $($arg)*
            );
        }
    })
}

#[macro_export]
macro_rules! info{
    ( $($arg:tt)* ) => ({
		if $crate::MAX_LEVEL >= $crate::LogLevel::Info {
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get(module_path!(), $crate::LogLevel::Info),
                $($arg)*
            );
        }
    })
}

#[macro_export]
macro_rules! warn{
    ( $($arg:tt)* ) => ({
		if $crate::MAX_LEVEL >= $crate::LogLevel::Warn{
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get(module_path!(), $crate::LogLevel::Warn),
                $($arg)*
            );
        }
    })
}

#[macro_export]
macro_rules! error{
    ( $($arg:tt)* ) => ({
		if $crate::MAX_LEVEL >= $crate::LogLevel::Error{
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get(module_path!(), $crate::LogLevel::Error),
                $($arg)*
            );
        }
    })
}

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
// Trait Implementations
//==================================================================================================

impl fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        #[cfg(feature = "std")]
        println!("{s}");
        #[cfg(not(feature = "std"))]
        let _ = ::sys::kcall::debug::debug(s.as_ptr(), s.len());
        Ok(())
    }
}

impl Logger {
    pub fn get(tag: &str, level: LogLevel) -> Self {
        let mut ret: Self = Self;
        let _ = write!(&mut ret, "[{level:?}][{tag}] ");
        ret
    }
}
