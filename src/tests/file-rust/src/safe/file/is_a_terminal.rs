// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::safe::{
    StandardError,
    StandardInput,
    StandardOutput,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests whether we can check if a file is a terminal.
pub fn test() {
    // Check if STDIN is a terminal.
    match StandardInput::get().is_terminal() {
        Ok(true) => {},
        Ok(false) => {
            panic!("expected STDERR to be a terminal, but it is not");
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Check if STDOUT is a terminal.
    match StandardOutput::get().is_terminal() {
        Ok(true) => {},
        Ok(false) => {
            panic!("expected STDERR to be a terminal, but it is not");
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }

    // Check if STDERR is a terminal.
    match StandardError::get().is_terminal() {
        Ok(true) => {},
        Ok(false) => {
            panic!("expected STDERR to be a terminal, but it is not");
        },
        Err(error) => {
            panic!("{error:?}");
        },
    }
}
