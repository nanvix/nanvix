// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Terminal-generated signal.

/// A terminal-generated signal recognized while `ISIG` is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalSignal {
    /// `VINTR` (`^C`), mapped to `SIGINT` by the daemon.
    Interrupt,
    /// `VQUIT` (`^\`), mapped to `SIGQUIT` by the daemon.
    Quit,
    /// `VSUSP` (`^Z`), mapped to `SIGTSTP` by the daemon.
    Suspend,
}
