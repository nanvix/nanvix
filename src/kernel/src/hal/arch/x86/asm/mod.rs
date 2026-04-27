// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fast_memcpy;
mod fast_memset;
mod hooks;

#[cfg(not(feature = "hyperlight"))]
pub(crate) use self::fast_memcpy::fast_memcpy;
pub(crate) use self::fast_memset::fast_memset;
