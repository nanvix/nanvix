// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wasi::{
    types::{
        Errno,
        Fd,
        FdFlags,
    },
    WasiCtxInner,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl WasiCtxInner {
    /// Accepts a new connection on a socket.
    pub(super) fn sock_accept(&self, _sockfd: Fd, _fdflags: FdFlags) -> Result<Fd, Errno> {
        // TODO: implement this operation.
        Err(Errno::Nosys)
    }
}
