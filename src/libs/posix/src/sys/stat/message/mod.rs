// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fstat;
mod fstatat;
mod futimens;
mod utimensat;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    fstat::FileStatRequest,
    fstatat::{
        FileStatAtRequest,
        FileStatAtResponse,
    },
    futimens::{
        UpdateFileAccessTimeRequest,
        UpdateFileAccessTimeResponse,
    },
    utimensat::{
        UpdateFileAccessTimeAtRequest,
        UpdateFileAccessTimeAtResponse,
    },
};
