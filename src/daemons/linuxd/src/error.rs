// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use ::nix::libc;
use ::std::io;
use ::syscomm::SocketError;

#[derive(Debug)]
pub enum WorkerThreadError {
    Error(io::Error),
    Interrupted,
}

impl From<io::Error> for WorkerThreadError {
    fn from(e: io::Error) -> Self {
        if e.raw_os_error() == Some(libc::EINTR) {
            WorkerThreadError::Interrupted
        } else {
            WorkerThreadError::Error(e)
        }
    }
}

impl From<SocketError> for WorkerThreadError {
    fn from(e: SocketError) -> Self {
        if e.raw_os_error() == Some(libc::EINTR) {
            WorkerThreadError::Interrupted
        } else {
            WorkerThreadError::Error(e.into())
        }
    }
}
