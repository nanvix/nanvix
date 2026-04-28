// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub(crate) mod constants;
mod fast_memcpy;
mod fast_memset;
mod hooks;

pub(crate) use self::{
    fast_memcpy::fast_memcpy,
    fast_memset::fast_memset,
};
