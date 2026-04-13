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
use ::hyperlight_guest::guest_handle::handle::GuestHandle;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Global Variables
//==================================================================================================

pub(crate) static mut GUEST_HANDLE: GuestHandle = GuestHandle::new();

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
            Ok(())
        }
    }

    ///
    /// # Description
    ///
    /// Writes a string to the guest's standard debug output.
    ///
    /// # Note
    ///
    /// `debug_print()` is infallible (it uses inline `out` instructions), so this method always
    /// returns `Ok(())`. The `Result` return type is preserved for API compatibility.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it uses raw asm under the hood.
    ///
    pub unsafe fn puts(message: &str) -> Result<(), Error> {
        ::hyperlight_guest::exit::debug_print(message);
        Ok(())
    }

    /// Gets the number of credits available to the guest.
    ///
    /// # Safety
    /// This function is unsafe because it dereferences a raw pointer.
    pub unsafe fn get_credits() -> Result<u64, Error> {
        use ::hyperlight_common::layout::{
            MAX_GVA,
            SCRATCH_TOP_GUEST_COUNTER_OFFSET,
        };
        let credits_gva: usize = MAX_GVA - SCRATCH_TOP_GUEST_COUNTER_OFFSET as usize + 1;
        let credits_ptr: *const u64 = credits_gva as *const u64;
        let val = ::core::ptr::read_volatile(credits_ptr);
        Ok(val)
    }

    /// Writes a message to the guest's standard output.
    ///
    /// # Safety
    /// This function is unsafe because it uses a static mutable variable.
    pub unsafe fn vmbus_write(data: &[u8]) -> Result<(), Error> {
        let failure_reason: &'static str = "vmbus_write: failed to write data";
        let count: i32 = GUEST_HANDLE
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

    /// Writes bulk data (header + payload) via the VmbusBulkWrite host function.
    ///
    /// # Safety
    /// This function is unsafe because it uses a static mutable variable.
    pub unsafe fn vmbus_bulk_write(data: &[u8]) -> Result<(), Error> {
        let failure_reason: &'static str = "vmbus_bulk_write: failed to write data";
        let count: i32 = GUEST_HANDLE
            .call_host_function::<i32>(
                "VmbusBulkWrite",
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

    /// Reads the next chunk of bulk data from the host via the VmbusBulkRead host function.
    ///
    /// Returns an empty Vec when all bulk data has been consumed.
    ///
    /// # Safety
    /// This function is unsafe because it uses a static mutable variable.
    pub unsafe fn vmbus_bulk_read() -> Result<Vec<u8>, Error> {
        let failure_reason: &'static str = "vmbus_bulk_read: failed to read data";
        GUEST_HANDLE
            .call_host_function::<Vec<u8>>("VmbusBulkRead", None, ReturnType::VecBytes)
            .map_err(|_| Error::new(ErrorCode::IoErr, failure_reason))
    }
}
