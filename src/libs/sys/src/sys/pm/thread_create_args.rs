// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mm::VirtualAddress;

//==================================================================================================
// Structures
//==================================================================================================

/// Argument structure used with the `create_thread()` kernel call.
#[derive(Debug, Copy, Clone)]
pub struct ThreadCreateArgs {
    /// User wrapper function to be executed by the thread.
    pub user_wrapper_fn: VirtualAddress,

    /// User function to be executed by the thread.
    pub user_fn: VirtualAddress,

    /// Argument to be passed to the user function.
    pub user_fn_arg: usize,
}

impl Default for ThreadCreateArgs {
    fn default() -> Self {
        Self {
            user_wrapper_fn: VirtualAddress::from_raw_value(0),
            user_fn: VirtualAddress::from_raw_value(0),
            user_fn_arg: 0,
        }
    }
}
