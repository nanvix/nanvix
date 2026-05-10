// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Platform Selection
//==================================================================================================

//! Platform abstraction layer for the networking backend.
//!
//! This module conditionally compiles the correct platform-specific module and re-exports its
//! contents. Platform `cfg` attributes outside of this module should be limited to test-only
//! code (e.g., `#[cfg(windows)]` on test functions that exercise platform-specific behavior).

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(crate) use self::unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::*;
