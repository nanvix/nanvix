// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(non_snake_case)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::{
    string::ToString,
    vec::Vec,
};
use ::hyperlight_common::{
    flatbuffer_wrappers::function_types::{
        ParameterValue,
        ReturnType,
    },
    mem::HyperlightPEB,
};
use ::hyperlight_guest::guest_handle::handle::GuestHandle;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Global Variables
//==================================================================================================

pub(crate) static mut GUEST_HANDLE: GuestHandle = GuestHandle::new();

/// Buffer used to store messages before the PEB is initialized.
static mut OUTPUT_BUFFER: Buffer = Buffer {
    data: [0; Buffer::CAPACITY],
    len: 0,
};

//==================================================================================================
// Process Environment Block
//==================================================================================================

#[derive(Debug)]
pub struct ProcessEnvironmentBlock;

impl ProcessEnvironmentBlock {
    /// Initializes the process environment block.
    ///
    /// # Safety
    /// This function is unsafe because it handles a static mutable variable.
    pub unsafe fn init(peb_base: *mut HyperlightPEB) -> Result<(), Error> {
        if GUEST_HANDLE.peb().is_some() {
            let reason: &'static str = "init: peb already initialized";
            error!("{reason}");
            Err(Error::new(ErrorCode::ResourceBusy, reason))
        } else {
            GUEST_HANDLE = GuestHandle::init(peb_base);
            OUTPUT_BUFFER.flush(&mut |s: &str| Self::host_print(s))?;
            Ok(())
        }
    }

    /// Sets the guest function dispatch pointer.
    ///
    /// # Safety
    /// This function is unsafe because it dereferences a raw pointer.
    pub unsafe fn set_guest_function_dispatch_ptr(ptr: u64) -> Result<(), Error> {
        match GUEST_HANDLE.peb() {
            Some(peb_ptr) => {
                (*peb_ptr).guest_function_dispatch_ptr = ptr;
                Ok(())
            },
            None => {
                let reason: &'static str = "set_guest_function_dispatch_ptr: peb not initialized";
                error!("{reason}");
                Err(Error::new(ErrorCode::NoSuchDevice, reason))
            },
        }
    }

    /// Writes a string to the guest's standard output.
    ///
    /// # Safety
    /// This function is unsafe because it uses raw asm under the hood.
    pub unsafe fn puts(message: &str) -> Result<(), Error> {
        // Check if PEB is not initialized to decide whether to buffer the message.
        if GUEST_HANDLE.peb().is_none() {
            return OUTPUT_BUFFER.append(message);
        }

        Self::host_print(message);

        Ok(())
    }

    /// Gets the number of credits available to the guest.
    ///
    /// # Safety
    /// This function is unsafe because it dereferences a raw pointer.
    pub unsafe fn get_credits() -> Result<u64, Error> {
        match GUEST_HANDLE.peb() {
            Some(peb_ptr) => {
                // Credits_value is updated asynchronously by the host;
                // so we use a volatile read to avoid reading stale data.
                Ok(::core::ptr::read_volatile::<u64>(::core::ptr::addr_of!(
                    (*peb_ptr).credits_value
                )))
            },
            None => {
                let reason: &'static str = "get_credits: peb not initialized";
                error!("{reason}");
                Err(Error::new(ErrorCode::NoSuchDevice, reason))
            },
        }
    }

    /// Writes a message to the guest's standard output.
    ///
    /// # Safety
    /// This function is unsafe because it uses a static mutable variable.
    pub unsafe fn vmbus_write(data: &[u8]) -> Result<(), Error> {
        let failure_reason: &'static str = "vmbus_write: failed to write data";
        let count = GUEST_HANDLE
            .call_host_function::<i32>(
                "VmbusWrite",
                Some(Vec::from(&[ParameterValue::VecBytes(Vec::from(data))])),
                ReturnType::Int,
            )
            .map_err(|_| Error::new(ErrorCode::IoErr, failure_reason))?;

        if count != data.len() as i32 {
            Err(Error::new(ErrorCode::IoErr, failure_reason))
        } else {
            Ok(())
        }
    }

    /// Reads a message from the guest's standard input.
    ///
    /// # Safety
    /// This function is unsafe because it uses a static mutable variable.
    pub unsafe fn vmbus_read() -> Result<Vec<u8>, Error> {
        let failure_reason: &'static str = "vmbus_read: failed to read data";
        GUEST_HANDLE
            .call_host_function::<Vec<u8>>("VmbusRead", None, ReturnType::VecBytes)
            .map_err(|_| Error::new(ErrorCode::IoErr, failure_reason))
    }

    /// Writes a string to the host's standard output.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it uses a static mutable variable.
    unsafe fn host_print(message: &str) {
        let _ = GUEST_HANDLE.call_host_function::<i32>(
            "HostPrint",
            Some(Vec::from(&[ParameterValue::String(message.to_string())])),
            ReturnType::Int,
        );
    }
}

//==================================================================================================
// Buffer
//==================================================================================================

/// A fixed-size buffer for storing output messages.
struct Buffer {
    /// Underlying data.
    data: [u8; Self::CAPACITY],
    /// Current length.
    len: usize,
}

impl Buffer {
    /// Buffer capacity.
    const CAPACITY: usize = 512;

    ///
    /// # Description
    ///
    /// Appends a message to the buffer.
    ///
    /// # Parameters
    ///
    /// - `message`: Message to append.
    ///
    /// # Return Value
    ///
    /// On success, this function returns an empty tuple. On failure, it returns an object that
    /// describes the error.
    ///
    /// # Notes
    ///
    /// - This function intentionally does not write to the log to avoid recursive calls.
    ///
    fn append(&mut self, message: &str) -> Result<(), Error> {
        let message_bytes: &[u8] = message.as_bytes();

        // Check if the message does not fit in the buffer.
        if message_bytes.len() > Self::CAPACITY {
            return Err(Error::new(ErrorCode::MessageTooLong, "message too long"));
        }

        // Check if there is not enough space in the buffer.
        if self.len + message_bytes.len() > Self::CAPACITY {
            return Err(Error::new(ErrorCode::NoSpaceOnDevice, "buffer full"));
        }

        self.data[self.len..self.len + message_bytes.len()].copy_from_slice(message_bytes);
        self.len += message_bytes.len();

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Flushes the buffer using the provided write function.
    ///
    /// # Parameters
    ///
    /// - `write_fn`: Function used to write the buffered data.
    ///
    /// # Return Value
    ///
    /// On success, this function returns an empty tuple. On failure, it returns an object that
    /// describes the error.
    ///
    /// # Notes
    ///
    /// - This function intentionally does not write to the log to avoid recursive calls.
    ///
    fn flush(&mut self, write_fn: &mut dyn FnMut(&str)) -> Result<(), Error> {
        // Check if there is nothing to flush.
        if self.len == 0 {
            return Ok(());
        }

        // Convert the buffered data to a string.
        let buffered_message: &str =
            ::core::str::from_utf8(&self.data[..self.len]).map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, "puts: buffered data is not valid UTF-8")
            })?;

        // Write the buffered message using the provided function.
        write_fn(buffered_message);
        self.len = 0;

        Ok(())
    }
}
