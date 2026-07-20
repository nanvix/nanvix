// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! In-memory POSIX unnamed pipes.

//==================================================================================================
// Modules
//==================================================================================================

mod pipe_end;
mod pipe_end_error;
mod pipe_read_outcome;
mod pipe_write_outcome;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use self::{
    pipe_end::{
        PipeEnd,
        PIPE_BUF,
        PIPE_CAPACITY,
    },
    pipe_end_error::PipeEndError,
    pipe_read_outcome::PipeReadOutcome,
    pipe_write_outcome::PipeWriteOutcome,
};
