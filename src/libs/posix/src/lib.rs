// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(never_type)] // pthread requires this.
#![feature(c_variadic)] // fcntl requires this.

//==================================================================================================
// Modules
//==================================================================================================

extern crate nvx;

extern crate alloc;

#[cfg(feature = "syscall")]
extern crate syslog;

// Address and routing parameter area.
pub mod arpa;

/// Dynamic linking.
pub mod dlfcn;

/// Dummy implementations.
pub mod dummy;

/// System error numbers.
pub mod errno;

/// Virtual environments.
pub mod venv;

/// Definitions for network database operations.
pub mod netdb;

/// Definitions for the poll() function.
pub mod poll;

/// Posix threads.
pub mod pthread;

/// Password structure.
pub mod pwd;

/// File last access and modification times.
pub mod utime;

/// System-specific headers.
pub mod sys;
