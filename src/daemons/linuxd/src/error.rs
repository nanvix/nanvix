// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

#[derive(Debug)]
pub enum WorkerThreadError {
    Error(sys::error::Error),
    Interrupted,
}

impl From<sys::error::Error> for WorkerThreadError {
    fn from(err: sys::error::Error) -> Self {
        WorkerThreadError::Error(err)
    }
}
