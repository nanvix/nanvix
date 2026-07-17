// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Non-blocking pipe read outcome.

/// Outcome of a non-blocking pipe read attempt.
#[derive(Debug)]
pub enum PipeReadOutcome {
    /// Copied `N` bytes out of the buffer.
    Read(usize),
    /// Buffer empty with writers still open.
    WouldBlock,
    /// Buffer empty and no writers remain.
    Eof,
}
