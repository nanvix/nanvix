// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// I/O operations.
pub mod ioctl;

/// Memory management declarations.
pub mod mman;

/// Definitions for resource operations.
pub mod resource;

/// Synchronous I/O multiplexing.
pub mod select;

/// Sockets.
pub mod socket;

/// File status.
pub mod stat;

/// Time types.
pub mod time;

/// File access and modification times structure.
pub mod times;

/// Definitions for vector I/O operations.
pub mod uio;

/// Definitions for UNIX domain sockets.
pub mod un;

/// System name structure.
pub mod utsname;
