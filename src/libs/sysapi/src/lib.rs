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

/// Byte-order constants.
pub mod endian;

/// System Error Numbers
pub mod errno;

/// Format of directory entries
pub mod dirent;

/// Dynamic linking.
pub mod dlfcn;

/// Foreign Function Interface
pub mod ffi;

/// File control operations.
pub mod fcntl;

/// Floating-point environment.
pub mod fenv;

/// File-tree-walk constants.
pub mod ftw;

/// Command-line option parsing.
pub mod getopt;

/// Pathname pattern matching.
pub mod glob;

/// Group structure.
pub mod grp;

/// Codeset conversion.
pub mod iconv;

/// Implementation-defined constants.
pub mod limits;

/// Definitions for Network Database Operations
pub mod netdb;

/// Network interface definitions.
pub mod net_if;

/// Internet Address Family
pub mod netinet_in;

/// Definitions for the Internet Transmission Control Protocol (TCP)
pub mod netinet_tcp;

/// Message catalog types.
pub mod nl_types;

/// Default system paths.
pub mod paths;

/// Definitions for I/O polling.
pub mod poll;

/// Posix threads.
pub mod pthread;

/// Password structure.
pub mod pwd;

/// Execution scheduling.
pub mod sched;

/// Signal handling.
pub mod signal;

/// Standard type definitions.
pub mod stddef;

/// Device control operations.
pub mod sys_ioctl;

/// Legacy system parameters.
pub mod sys_param;

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

/// File-system information.
pub mod sys_statvfs;

/// Time-of-day types.
pub mod sys_time;

/// File access and modification times structure.
pub mod sys_times;

/// System Types
pub mod sys_types;

/// Definitions for vector I/O operations.
pub mod sys_uio;

/// Definitions for UNIX Domain Sockets
pub mod sys_un;

/// System identification.
pub mod sys_utsname;

/// Process termination status.
pub mod sys_wait;

/// System logging interface.
pub mod syslog;

/// General terminal interface.
pub mod termios;

/// Time types.
pub mod time;

/// File last access and modification times.
pub mod utime;

/// Standard symbolic constants and types.
pub mod unistd;
