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
    ///
    /// This function is unsafe because it writes to the global `GUEST_HANDLE` static mutable.
    ///
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
    ///
    /// This function is unsafe because it dereferences a raw pointer at a fixed GVA.
    ///
    pub unsafe fn get_credits() -> Result<u64, Error> {
        // Credits are stored at a fixed offset from the top of scratch memory.
        // The host writes here via GuestCounter; we read via volatile read.
        // GVA = MAX_GVA - SCRATCH_TOP_GUEST_COUNTER_OFFSET + 1
        use ::hyperlight_common::layout::{
            MAX_GVA,
            SCRATCH_TOP_GUEST_COUNTER_OFFSET,
        };
        let credits_gva: usize = MAX_GVA - SCRATCH_TOP_GUEST_COUNTER_OFFSET as usize + 1;
        let credits_ptr: *const u64 = credits_gva as *const u64;
        Ok(::core::ptr::read_volatile(credits_ptr))
    }

    /// Writes a message to the guest's standard output.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it accesses the global `GUEST_HANDLE` static mutable.
    ///
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
    ///
    /// This function is unsafe because it accesses the global `GUEST_HANDLE` static mutable.
    ///
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
    ///
    /// This function is unsafe because it accesses the global `GUEST_HANDLE` static mutable.
    ///
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
    ///
    /// This function is unsafe because it accesses the global `GUEST_HANDLE` static mutable.
    ///
    pub unsafe fn vmbus_bulk_read() -> Result<Vec<u8>, Error> {
        let failure_reason: &'static str = "vmbus_bulk_read: failed to read data";
        GUEST_HANDLE
            .call_host_function::<Vec<u8>>("VmbusBulkRead", None, ReturnType::VecBytes)
            .map_err(|_| Error::new(ErrorCode::IoErr, failure_reason))
    }

    ///
    /// # Description
    ///
    /// Queries the host for the authoritative physical memory layout.
    ///
    /// Returns `(snapshot_budget_size, pt_overhead, ramfs_base, ramfs_size, scratch_size)` as
    /// reported by the VMM. All values are in bytes. RAMFS fields are zero when no RAMFS
    /// is present. `snapshot_budget_size` is the deterministic snapshot size.
    /// `pt_overhead` is always 0 with `nanvix-unstable` because Hyperlight skips
    /// guest page-table generation; the field is reserved for forward-compatibility.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it accesses the global `GUEST_HANDLE` static mutable.
    ///
    pub unsafe fn get_memory_layout() -> Result<(usize, usize, usize, usize, usize), Error> {
        let failure_reason: &'static str = "get_memory_layout: failed to query host";
        let bytes: Vec<u8> = GUEST_HANDLE
            .call_host_function::<Vec<u8>>("GetMemoryLayout", None, ReturnType::VecBytes)
            .map_err(|_| Error::new(ErrorCode::IoErr, failure_reason))?;

        const EXPECTED_LEN: usize = 20;
        if bytes.len() < EXPECTED_LEN {
            let reason: &'static str = "get_memory_layout: response too short";
            error!("{reason} (got {} bytes, expected {EXPECTED_LEN})", bytes.len());
            return Err(Error::new(ErrorCode::InvalidMessage, reason));
        }

        let snapshot_budget_size: usize =
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let pt_overhead: usize =
            u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        let ramfs_base: usize =
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let ramfs_size: usize =
            u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let scratch_size: usize =
            u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;

        Ok((snapshot_budget_size, pt_overhead, ramfs_base, ramfs_size, scratch_size))
    }
}
