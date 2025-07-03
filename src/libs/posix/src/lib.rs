// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(never_type)] // pthread requires this.
#![feature(c_variadic)] // fcntl requires this.
#![feature(btree_extract_if)] // dlfcn requires this.
#![feature(strict_overflow_ops)]

//==================================================================================================
// Modules
//==================================================================================================

extern crate nvx;

extern crate alloc;

// Address and routing parameter area.
pub mod arpa;

/// Format of directory entries
pub mod dirent;

/// Dynamic linking.
pub mod dlfcn;

/// System error numbers.
pub mod errno;

/// Time types.
pub mod time;

/// Virtual environments.
pub mod venv;

/// File control operations.
pub mod fcntl;

/// Definitions for network database operations.
pub mod netdb;

/// Definitions for the poll() function.
pub mod poll;

/// Posix threads.
pub mod pthread;

/// Password structure.
pub mod pwd;

/// Standard symbolic constants and types.
pub mod unistd;

/// File last access and modification times.
pub mod utime;

/// Execution scheduling.
pub mod sched;

/// Standard library definitions.
pub mod stdlib;

/// Signals
pub mod signal;

/// System-specific headers.
pub mod sys;
