// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Resources reclaimed when process descriptor state is released.

//==================================================================================================
// Imports
//==================================================================================================

use super::PipeClosure;
use ::alloc::vec::Vec;

//==================================================================================================
// Structures
//==================================================================================================

/// Resources that the daemon must reclaim after descriptors are released.
pub struct ProcessExitReclaim {
    /// Remote host filesystem descriptors that must be closed.
    pub orphaned_hostfs_fds: Vec<i32>,
    /// Remote network descriptors that must be closed.
    pub orphaned_socket_fds: Vec<i32>,
    /// Pipe ends whose reference count reached zero.
    pub pipe_closures: Vec<PipeClosure>,
}
