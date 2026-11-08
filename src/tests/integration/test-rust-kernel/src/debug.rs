// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Debug Kernel Call Regression Tests
//!
//! The `debug()` kernel call copies at most [`MAX_DEBUG_MESSAGE_SIZE`] bytes of a message into a
//! kernel buffer. The kernel used to reject a longer message as a whole, logging an error in place
//! of the message, so the information that the message carried was lost. It now writes the leading
//! part of a long message, cut at a character boundary, followed by a truncation marker.
//!
//! The tests below verify that:
//!
//! 1. Messages of exactly [`MAX_DEBUG_MESSAGE_SIZE`] bytes, one byte longer, and several times
//!    longer are accepted.
//! 2. A long message whose cut falls in the middle of a multi-byte character is accepted.
//! 3. Invalid UTF-8 is still rejected: an invalid byte, an incomplete character at the end of a
//!    message that fits in the kernel buffer, and an invalid byte before the cut of a long message.
//! 4. Logging a message longer than [`MAX_DEBUG_MESSAGE_SIZE`] bytes through `syslog` completes.
//!    The logger splits the message across kernel calls, so the kernel log shows it whole.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::debug::{
        self,
        MAX_DEBUG_MESSAGE_SIZE,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Byte that fills test messages.
const FILLER: u8 = b'x';

/// UTF-8 encoding of a three-byte character (U+20AC EURO SIGN).
const EURO_SIGN: [u8; 3] = [0xE2, 0x82, 0xAC];

/// Byte that never appears in UTF-8.
const INVALID_BYTE: u8 = 0xFF;

/// Length of the short test messages (in bytes).
const SHORT_MESSAGE_SIZE: usize = 64;

//==================================================================================================
// Helper Functions
//==================================================================================================

///
/// # Description
///
/// Builds a test message that starts with a label naming the test and is filled with [`FILLER`].
///
/// # Parameters
///
/// - `test`: Name of the test.
///
/// # Returns
///
/// The test message.
///
fn build_message<const N: usize>(test: &str) -> [u8; N] {
    let mut message: [u8; N] = [FILLER; N];
    let label: [&str; 3] = ["test-kernel: debug: ", test, ": "];
    for (dst, src) in message
        .iter_mut()
        .zip(label.iter().flat_map(|part| part.bytes()))
    {
        *dst = src;
    }
    message
}

///
/// # Description
///
/// Writes a message with the `debug()` kernel call, and then ends the line in the kernel log. The
/// line is ended separately because the kernel drops the tail of a long message.
///
/// # Parameters
///
/// - `message`: Message to write.
///
/// # Returns
///
/// The result of the kernel call that writes `message`.
///
fn write(message: &[u8]) -> Result<(), Error> {
    let result: Result<(), Error> = debug::__kcall_debug(message.as_ptr(), message.len());
    let newline: &[u8] = b"\n";
    let _ = debug::__kcall_debug(newline.as_ptr(), newline.len());
    result
}

///
/// # Description
///
/// Checks that the kernel accepts a message.
///
/// # Parameters
///
/// - `test`: Name of the test.
/// - `message`: Message to write.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidArgument`] if the kernel rejects `message`.
///
fn expect_accepted(test: &'static str, message: &[u8]) -> Result<(), Error> {
    if let Err(e) = write(message) {
        ::syslog::error!("test-kernel: debug: {test}: FAIL - message rejected (error={e:?})");
        return Err(Error::new(ErrorCode::InvalidArgument, test));
    }
    Ok(())
}

///
/// # Description
///
/// Checks that the kernel rejects a message as an invalid argument.
///
/// # Parameters
///
/// - `test`: Name of the test.
/// - `message`: Message to write.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidArgument`] if the kernel accepts `message`, or if it rejects it
/// with an unexpected error.
///
fn expect_rejected(test: &'static str, message: &[u8]) -> Result<(), Error> {
    match write(message) {
        Err(e) if e.code == ErrorCode::InvalidArgument => Ok(()),
        Err(e) => {
            ::syslog::error!("test-kernel: debug: {test}: FAIL - unexpected error (error={e:?})");
            Err(Error::new(ErrorCode::InvalidArgument, test))
        },
        Ok(()) => {
            ::syslog::error!("test-kernel: debug: {test}: FAIL - message accepted");
            Err(Error::new(ErrorCode::InvalidArgument, test))
        },
    }
}

//==================================================================================================
// Test Cases
//==================================================================================================

/// Writes a message that fills the kernel buffer exactly.
fn test_message_at_limit() -> Result<(), Error> {
    const TEST: &str = "message_at_limit";
    let message: [u8; MAX_DEBUG_MESSAGE_SIZE] = build_message(TEST);
    expect_accepted(TEST, &message)
}

/// Writes a message that is one byte longer than the kernel buffer.
fn test_message_over_limit() -> Result<(), Error> {
    const TEST: &str = "message_over_limit";
    let message: [u8; MAX_DEBUG_MESSAGE_SIZE + 1] = build_message(TEST);
    expect_accepted(TEST, &message)
}

/// Writes a message that is several times longer than the kernel buffer.
fn test_long_message() -> Result<(), Error> {
    const TEST: &str = "long_message";
    let message: [u8; 4 * MAX_DEBUG_MESSAGE_SIZE] = build_message(TEST);
    expect_accepted(TEST, &message)
}

/// Writes a long message whose cut falls in the middle of a multi-byte character.
fn test_cut_inside_character() -> Result<(), Error> {
    const TEST: &str = "cut_inside_character";
    let mut message: [u8; MAX_DEBUG_MESSAGE_SIZE + 2] = build_message(TEST);
    // Only the first byte of the character fits in the kernel buffer.
    message[MAX_DEBUG_MESSAGE_SIZE - 1..].copy_from_slice(&EURO_SIGN);
    expect_accepted(TEST, &message)
}

/// Writes a message that contains an invalid byte.
fn test_invalid_byte() -> Result<(), Error> {
    const TEST: &str = "invalid_byte";
    let mut message: [u8; SHORT_MESSAGE_SIZE] = build_message(TEST);
    message[SHORT_MESSAGE_SIZE - 1] = INVALID_BYTE;
    expect_rejected(TEST, &message)
}

/// Writes a message that fits in the kernel buffer, but ends in the middle of a character. Only
/// the tail of a long message is dropped, so the kernel must not drop the partial character.
fn test_incomplete_character() -> Result<(), Error> {
    const TEST: &str = "incomplete_character";
    let mut message: [u8; SHORT_MESSAGE_SIZE] = build_message(TEST);
    message[SHORT_MESSAGE_SIZE - 1] = EURO_SIGN[0];
    expect_rejected(TEST, &message)
}

/// Writes a long message that contains an invalid byte before the cut.
fn test_long_message_invalid_byte() -> Result<(), Error> {
    const TEST: &str = "long_message_invalid_byte";
    let mut message: [u8; MAX_DEBUG_MESSAGE_SIZE + 1] = build_message(TEST);
    message[MAX_DEBUG_MESSAGE_SIZE - 1] = INVALID_BYTE;
    expect_rejected(TEST, &message)
}

/// Logs a message that is several times longer than the kernel buffer through `syslog`. The
/// message is made of multi-byte characters, so that the logger cannot split it at the limit.
fn test_syslog_long_message() {
    let text: String = "\u{20AC}".repeat(MAX_DEBUG_MESSAGE_SIZE);
    ::syslog::info!("test-kernel: debug: syslog_long_message: {text}");
}

//==================================================================================================
// Public Interface
//==================================================================================================

///
/// # Description
///
/// Runs all `debug()` kernel call regression tests.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
/// # Errors
///
/// Returns the first error encountered by any test case.
///
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-kernel: debug: starting debug() regression tests");

    test_message_at_limit()?;
    ::syslog::info!("test-kernel: debug: PASS - message_at_limit");

    test_message_over_limit()?;
    ::syslog::info!("test-kernel: debug: PASS - message_over_limit");

    test_long_message()?;
    ::syslog::info!("test-kernel: debug: PASS - long_message");

    test_cut_inside_character()?;
    ::syslog::info!("test-kernel: debug: PASS - cut_inside_character");

    test_invalid_byte()?;
    ::syslog::info!("test-kernel: debug: PASS - invalid_byte");

    test_incomplete_character()?;
    ::syslog::info!("test-kernel: debug: PASS - incomplete_character");

    test_long_message_invalid_byte()?;
    ::syslog::info!("test-kernel: debug: PASS - long_message_invalid_byte");

    test_syslog_long_message();
    ::syslog::info!("test-kernel: debug: PASS - syslog_long_message");

    ::syslog::info!("test-kernel: debug: all tests passed");

    Ok(())
}
