// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Non-blocking console read outcome.

/// Outcome of a non-blocking console read attempt.
#[derive(Debug, PartialEq, Eq)]
pub enum ConsoleReadOutcome {
    /// Copied `N` bytes out of the cooked queue.
    Read(usize),
    /// No cooked input is available.
    WouldBlock,
    /// An end-of-file marker is ready.
    Eof,
}
