// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;
use ::arch::mem::PAGE_SIZE;
use ::core::{
    cmp,
    fmt::Write,
    ptr,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        mm,
        pm,
    },
    mm::{
        AccessPermission,
        Address,
        MmioRegionInfo,
    },
    pm::Capability,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Encoded 8-byte "RAMFS   " tag exposed by the MicroVM RAMFS MMIO region.
const RAMFS_MMIO_TAG: u64 = u64::from_be_bytes(*b"RAMFS   ");

/// Maximum number of bytes dumped per run to avoid overwhelming the logger.
const RAMFS_DUMP_MAX_BYTES: usize = 512;

/// Number of bytes rendered per hexdump line.
const RAMFS_DUMP_LINE_BYTES: usize = 16;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs the RAMFS `mmio_info()` regression test.
///
/// # Description
///
/// Allocates the RAMFS MMIO mapping, queries `mmio_info()`, and validates the metadata reported by
/// the kernel. This test requires the `-ramfs` option to be passed to nanvixd.
///
/// # Returns
///
/// `Ok(())` on success, or an error if the RAMFS region is not available or any invariant fails.
///
/// # Errors
///
/// Returns an error if the RAMFS region is not registered or if any validation fails.
pub fn run() -> Result<(), Error> {
    let mut cap_acquired: bool = false;
    let mut region_attached: bool = false;

    let mut result: Result<(), Error> = (|| {
        pm::__kcall_capctl(Capability::IoManagement, true)?;
        cap_acquired = true;

        match mm::__kcall_mmio_alloc(RAMFS_MMIO_TAG) {
            Ok(()) => region_attached = true,
            Err(err) if err.code == ErrorCode::NoSuchEntry => {
                let reason: &'static str =
                    "RAMFS MMIO region not found; ensure -ramfs option is passed to nanvixd";
                ::syslog::error!("test-kernel: {}", reason);
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            },
            Err(err) => return Err(err),
        }

        let info: MmioRegionInfo = mm::__kcall_mmio_info(RAMFS_MMIO_TAG)?;
        validate_info(&info)?;
        dump_ramfs_contents(&info)?;

        Ok(())
    })();

    if region_attached {
        if let Err(err) = mm::__kcall_mmio_free(RAMFS_MMIO_TAG) {
            ::syslog::error!("test-kernel: failed to free RAMFS region (error={:?})", err);
            if result.is_ok() {
                result = Err(err);
            }
        }
    }

    if cap_acquired {
        if let Err(err) = pm::__kcall_capctl(Capability::IoManagement, false) {
            ::syslog::error!(
                "test-kernel: failed to drop IoManagement capability (error={:?})",
                err
            );
            if result.is_ok() {
                result = Err(err);
            }
        }
    }

    result
}

///
/// # Description
///
/// Validates that the MMIO region metadata conforms to expected invariants.
///
/// # Parameters
///
/// - `info`: Reference to the MMIO region info structure returned by the kernel.
///
/// # Returns
///
/// `Ok(())` if all invariants hold, or an error describing which invariant failed.
///
fn validate_info(info: &MmioRegionInfo) -> Result<(), Error> {
    let permissions: AccessPermission = info.permissions()?;
    if permissions != AccessPermission::RDWR {
        return Err(Error::new(ErrorCode::InvalidArgument, "ramfs permissions mismatch"));
    }

    let base_raw: usize = info.base().into_raw_value();
    if base_raw == 0 || !base_raw.is_multiple_of(PAGE_SIZE) {
        return Err(Error::new(ErrorCode::InvalidArgument, "ramfs base is not page aligned"));
    }

    let size: usize = info.size();
    if size == 0 || !size.is_multiple_of(PAGE_SIZE) {
        return Err(Error::new(ErrorCode::InvalidArgument, "ramfs size is not page aligned"));
    }

    Ok(())
}

///
/// # Description
///
/// Dumps the initial bytes of the RAMFS region to the kernel log for diagnostic purposes.
///
/// # Parameters
///
/// - `info`: Reference to the MMIO region info structure containing base address and size.
///
/// # Returns
///
/// `Ok(())` on success, or an error if the base pointer is null.
///
fn dump_ramfs_contents(info: &MmioRegionInfo) -> Result<(), Error> {
    let total_size: usize = info.size();
    if total_size == 0 {
        ::syslog::warn!("test-kernel: skipped ramfs dump because region is empty");
        return Ok(());
    }

    let base_raw: usize = info.base().into_raw_value();
    if base_raw == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument, "ramfs base pointer is null"));
    }

    let dump_size: usize = cmp::min(total_size, RAMFS_DUMP_MAX_BYTES);

    ::syslog::info!(
        "test-kernel: ramfs dump begin (base={:#010x}, total_size_bytes={}, dump_size_bytes={})",
        base_raw,
        total_size,
        dump_size,
    );

    // SAFETY: The base pointer was validated to be non-null and the kernel guarantees the MMIO
    // region is mapped into the process address space for at least `dump_size` bytes. We use
    // volatile reads to prevent the compiler from optimizing away accesses to device memory.
    unsafe {
        let base_ptr: *const u8 = info.base().as_ptr();
        dump_ramfs_bytes(base_ptr, dump_size);
    }

    if total_size > dump_size {
        ::syslog::info!(
            "test-kernel: ramfs dump truncated (total_size_bytes={}, dumped_bytes={})",
            total_size,
            dump_size,
        );
    }

    ::syslog::info!("test-kernel: ramfs dump end");

    Ok(())
}

///
/// # Description
///
/// Reads bytes from the RAMFS region using volatile reads and logs them in hexdump format.
///
/// # Parameters
///
/// - `base_ptr`: Pointer to the start of the MMIO region.
/// - `dump_size`: Number of bytes to read and log.
///
/// # Safety
///
/// The caller must ensure that `base_ptr` is valid for reads of `dump_size` bytes and that the
/// memory region remains mapped for the duration of this function.
///
unsafe fn dump_ramfs_bytes(base_ptr: *const u8, dump_size: usize) {
    let mut offset: usize = 0;

    while offset < dump_size {
        let remaining: usize = dump_size - offset;
        let chunk_len: usize = cmp::min(remaining, RAMFS_DUMP_LINE_BYTES);
        let mut chunk_buf: [u8; RAMFS_DUMP_LINE_BYTES] = [0; RAMFS_DUMP_LINE_BYTES];

        for (idx, byte_ref) in chunk_buf.iter_mut().enumerate().take(chunk_len) {
            *byte_ref = ptr::read_volatile(base_ptr.add(offset + idx));
        }

        log_ramfs_line(offset, &chunk_buf[..chunk_len]);
        offset += chunk_len;
    }
}

///
/// # Description
///
/// Formats a single line of hexdump output and logs it via syslog.
///
/// # Parameters
///
/// - `offset`: Byte offset within the region for this line.
/// - `chunk`: Slice of bytes to render in hex and ASCII.
///
fn log_ramfs_line(offset: usize, chunk: &[u8]) {
    let mut line: String = String::with_capacity(64);
    let _ = line.write_fmt(format_args!("test-kernel: ramfs[{offset:#06x}]:"));

    for byte in chunk {
        let _ = line.write_fmt(format_args!(" {byte:02x}"));
    }

    line.push(' ');

    for &byte in chunk {
        let printable: char = match byte {
            0x20..=0x7e => char::from(byte),
            _ => '.',
        };
        line.push(printable);
    }

    ::syslog::info!("{}", line);
}
