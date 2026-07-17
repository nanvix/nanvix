// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! In-guest terminal line discipline for the vfsd console backend.

//==================================================================================================
// Modules
//==================================================================================================

mod console_read_outcome;
mod discipline;
mod terminal_signal;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use self::{
    console_read_outcome::ConsoleReadOutcome,
    discipline::LineDiscipline,
    terminal_signal::TerminalSignal,
};
