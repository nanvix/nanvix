// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Non-blocking pipe write outcome.

/// Outcome of a non-blocking pipe write attempt.
#[derive(Debug)]
pub enum PipeWriteOutcome {
    /// Appended `N` bytes.
    Wrote(usize),
    /// Insufficient space to make progress.
    WouldBlock,
    /// No readers remain.
    BrokenPipe,
}
