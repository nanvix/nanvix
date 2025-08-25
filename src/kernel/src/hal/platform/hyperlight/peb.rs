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

use ::alloc::vec::Vec;
use ::hyperlight_common::{
    flatbuffer_wrappers::function_types::{
        ParameterValue,
        ReturnType,
    },
    mem::HyperlightPEB,
};
use ::hyperlight_guest::{
    exit::debug_print,
    guest_handle::handle::GuestHandle,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug)]
pub struct ProcessEnvironmentBlock;

//==================================================================================================
// Global Variables
//==================================================================================================

pub(crate) static mut GUEST_HANDLE: GuestHandle = GuestHandle::new();

//==================================================================================================
// Implementations
//==================================================================================================

impl ProcessEnvironmentBlock {
    /// Initializes the process environment block.
    ///
    /// # Safety
    /// This function is unsafe because it handles a static mutable variable.
    pub unsafe fn init(peb_base: *mut HyperlightPEB) -> Result<(), Error> {
        if GUEST_HANDLE.peb().is_some() {
            let reason: &'static str = "init: peb already initialized";
            error!("{}", reason);
            Err(Error::new(ErrorCode::ResourceBusy, reason))
        } else {
            GUEST_HANDLE = GuestHandle::init(peb_base);
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
                error!("{}", reason);
                Err(Error::new(ErrorCode::NoSuchDevice, reason))
            },
        }
    }

    /// Writes a string to the guest's standard output.
    ///
    /// # Safety
    /// This function is unsafe because it uses raw asm under the hood.
    pub unsafe fn puts(message: &str) -> Result<(), Error> {
        debug_print(message);
        Ok(())
    }

    /// Gets the number of credits available to the guest.
    ///
    /// # Safety
    /// This function is unsafe because it dereferences a raw pointer.
    pub unsafe fn get_credits() -> Result<u64, Error> {
        match GUEST_HANDLE.peb() {
            Some(peb_ptr) => Ok((*peb_ptr).credits_value),
            None => {
                let reason: &'static str = "get_credits: peb not initialized";
                error!("{}", reason);
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
}
