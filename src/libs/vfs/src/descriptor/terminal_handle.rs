// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Opened terminal-device handle.

//==================================================================================================
// Imports
//==================================================================================================

use super::AccessMode;
use crate::line_discipline::LineDiscipline;
use ::alloc::sync::Arc;
use ::spin::Mutex;

//==================================================================================================
// Enumerations
//==================================================================================================

/// Identity of a named terminal device.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalDevice {
    /// The calling process's controlling terminal.
    Tty,
    /// The system console.
    Console,
}

//==================================================================================================
// Structures
//==================================================================================================

/// An open description of a named terminal device.
pub struct TerminalHandle {
    /// Named device opened by the caller.
    device: TerminalDevice,
    /// Access permitted by this open description.
    access_mode: AccessMode,
    /// Shared console line discipline.
    terminal: Arc<Mutex<LineDiscipline>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl TerminalHandle {
    /// Creates an opened terminal handle.
    pub fn new(
        device: TerminalDevice,
        access_mode: AccessMode,
        terminal: Arc<Mutex<LineDiscipline>>,
    ) -> Self {
        Self {
            device,
            access_mode,
            terminal,
        }
    }

    /// Returns the named terminal device.
    pub fn device(&self) -> TerminalDevice {
        self.device
    }

    /// Returns whether reads are permitted.
    pub fn readable(&self) -> bool {
        self.access_mode.readable()
    }

    /// Returns whether writes are permitted.
    pub fn writable(&self) -> bool {
        self.access_mode.writable()
    }

    /// Returns the shared line discipline.
    pub fn terminal(&self) -> &Arc<Mutex<LineDiscipline>> {
        &self.terminal
    }
}
