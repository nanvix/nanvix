// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! System API Library

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

/// Definitions for Internet Operations
pub mod arpa_inet;

/// System Error Numbers
pub mod errno;

/// Format of directory entries
pub mod dirent;

/// Foreign Function Interface
pub mod ffi;

/// File control operations.
pub mod fcntl;

/// Implementation-defined constants.
pub mod limits;

/// Definitions for Network Database Operations
pub mod netdb;

/// Internet Address Family
pub mod netinet_in;

/// Definitions for the Internet Transmission Control Protocol (TCP)
pub mod netinet_tcp;

/// Definitions for I/O polling.
pub mod poll;

/// Posix threads.
pub mod pthread;

/// Password structure.
pub mod pwd;

/// Execution scheduling.
pub mod sched;

/// Standard type definitions.
pub mod stddef;

/// Memory management operations.
pub mod sys_mman;

/// Definitions for resource operations.
pub mod sys_resource;

/// Synchronous I/O multiplexing.
pub mod sys_select;

/// Sockets Library
pub mod sys_socket;

/// File status.
pub mod sys_stat;

/// File access and modification times structure.
pub mod sys_times;

/// System Types
pub mod sys_types;

/// Definitions for vector I/O operations.
pub mod sys_uio;

/// Definitions for UNIX Domain Sockets
pub mod sys_un;

/// Time types.
pub mod time;

/// File last access and modification times.
pub mod utime;

/// Standard symbolic constants and types.
pub mod unistd;
