// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// Little-endian byte order.
pub const __LITTLE_ENDIAN: c_int = 1234;
/// Big-endian byte order.
pub const __BIG_ENDIAN: c_int = 4321;
/// PDP-endian byte order.
pub const __PDP_ENDIAN: c_int = 3412;

/// Byte order of the Rust compilation target.
#[cfg(target_endian = "little")]
pub const __BYTE_ORDER: c_int = __LITTLE_ENDIAN;

/// Byte order of the Rust compilation target.
#[cfg(target_endian = "big")]
pub const __BYTE_ORDER: c_int = __BIG_ENDIAN;

/// BSD-style alias for [`__LITTLE_ENDIAN`].
pub const LITTLE_ENDIAN: c_int = __LITTLE_ENDIAN;
/// BSD-style alias for [`__BIG_ENDIAN`].
pub const BIG_ENDIAN: c_int = __BIG_ENDIAN;
/// BSD-style alias for [`__PDP_ENDIAN`].
pub const PDP_ENDIAN: c_int = __PDP_ENDIAN;
/// BSD-style alias for [`__BYTE_ORDER`].
pub const BYTE_ORDER: c_int = __BYTE_ORDER;
