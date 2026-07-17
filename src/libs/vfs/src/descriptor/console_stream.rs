// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Console stream identity.

//==================================================================================================
// Enumerations
//==================================================================================================

/// Identifies which standard console stream a [`super::ConsoleHandle`] represents.
///
/// A console-backed descriptor performs no I/O of its own; the stream identity is the only state it
/// needs so that `fstat` can synthesize a stable device identity in a later plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleStream {
    /// Standard input (descriptor 0).
    Stdin,
    /// Standard output (descriptor 1).
    Stdout,
    /// Standard error (descriptor 2).
    Stderr,
}
