// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Console descriptor handle.

//==================================================================================================
// Imports
//==================================================================================================

use super::ConsoleStream;
use crate::line_discipline::LineDiscipline;
use ::alloc::sync::Arc;
use ::spin::Mutex;

//==================================================================================================
// Structures
//==================================================================================================

/// Routing token for a console-backed descriptor.
///
/// A console handle records which standard stream the descriptor represents and a reference to the
/// shared line discipline (terminal attributes plus cooked input), but performs no I/O of its own.
/// It exists so that vfsd can own the descriptor slot, its per-descriptor flags, and the terminal
/// device state while the actual console I/O is driven from the daemon.
pub struct ConsoleHandle {
    /// Which standard stream this descriptor represents.
    stream: ConsoleStream,
    /// Shared line discipline (terminal attributes and cooked input), referenced by every console
    /// descriptor of the device.
    terminal: Arc<Mutex<LineDiscipline>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ConsoleHandle {
    /// Creates a console handle for the given standard stream with its own fresh line discipline.
    ///
    /// Use [`ConsoleHandle::with_terminal`] when several handles must share one terminal device, as
    /// the standard streams do.
    pub fn new(stream: ConsoleStream) -> Self {
        Self::with_terminal(stream, Arc::new(Mutex::new(LineDiscipline::default())))
    }

    /// Creates a console handle for the given standard stream that references `terminal`.
    ///
    /// The three standard streams are created this way from a single shared line discipline so that
    /// they observe one consistent `termios`/`winsize` and one shared input buffer.
    pub fn with_terminal(stream: ConsoleStream, terminal: Arc<Mutex<LineDiscipline>>) -> Self {
        Self { stream, terminal }
    }

    /// Returns the standard stream this handle represents.
    pub fn stream(&self) -> ConsoleStream {
        self.stream
    }

    /// Returns a reference to the shared line discipline backing this console descriptor.
    pub fn terminal(&self) -> &Arc<Mutex<LineDiscipline>> {
        &self.terminal
    }
}
