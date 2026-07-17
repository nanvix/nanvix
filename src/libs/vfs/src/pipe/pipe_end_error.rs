// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Pipe-end direction error.

/// Error returned when a pipe end is used in the wrong direction.
#[derive(Debug, PartialEq, Eq)]
pub enum PipeEndError {
    /// Read attempted on the write end, or write attempted on the read end.
    WrongDirection,
}
