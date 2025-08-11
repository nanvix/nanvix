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
    pub unsafe fn init(peb_base: *mut HyperlightPEB) -> Result<(), Error> {
        if GUEST_HANDLE.peb().is_some() {
            error!("peb already initialized");
            Err(Error::new(ErrorCode::ResourceBusy, "peb already initialized"))
        } else {
            GUEST_HANDLE = GuestHandle::init(peb_base);
            Ok(())
        }
    }

    pub unsafe fn set_guest_function_dispatch_ptr(ptr: u64) -> Result<(), Error> {
        match GUEST_HANDLE.peb() {
            Some(peb_ptr) => {
                (*peb_ptr).guest_function_dispatch_ptr = ptr;
                Ok(())
            },
            None => {
                error!("peb not initialized");
                Err(Error::new(ErrorCode::NoSuchDevice, "peb not initialized"))
            },
        }
    }

    pub unsafe fn puts(message: &str) -> Result<(), Error> {
        debug_print(message);
        Ok(())
    }

    pub unsafe fn vmbus_write(data: &[u8]) -> Result<(), Error> {
        let count = GUEST_HANDLE
            .call_host_function::<i32>(
                "VmbusWrite",
                Some(Vec::from(&[ParameterValue::VecBytes(Vec::from(data))])),
                ReturnType::Int,
            )
            .map_err(|_| Error::new(ErrorCode::IoErr, "failed to write data"))?;

        if count != data.len() as i32 {
            Err(Error::new(ErrorCode::IoErr, "failed to write data"))
        } else {
            Ok(())
        }
    }

    pub unsafe fn vmbus_read() -> Result<Vec<u8>, Error> {
        GUEST_HANDLE
            .call_host_function::<Vec<u8>>("VmbusRead", None, ReturnType::VecBytes)
            .map_err(|_| Error::new(ErrorCode::IoErr, "failed to read data"))
    }
}
