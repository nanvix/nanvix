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
    pub user_fn: VirtualAddress,

    /// First argument to be passed to the user function.
    pub user_fn_arg0: usize,

    /// Second argument to be passed to the user function.
    pub user_fn_arg1: usize,

    /// Base address of the user stack.
    pub user_stack_base: VirtualAddress,

    /// Size of the user stack.
    pub user_stack_size: usize,

    /// Optional base address for the user-space thread data area.
    pub user_tda: Option<VirtualAddress>,
}

impl ThreadCreateArgs {
    /// Null user function.
    pub const NULL_USER_FN: VirtualAddress = VirtualAddress::new(0);
}

impl Default for ThreadCreateArgs {
    fn default() -> Self {
        Self {
            user_fn: VirtualAddress::from_raw_value(0),
            user_fn_arg0: 0,
            user_fn_arg1: 0,
            user_stack_base: VirtualAddress::from_raw_value(0),
            user_stack_size: 0,
            user_tda: None,
        }
    }
}
