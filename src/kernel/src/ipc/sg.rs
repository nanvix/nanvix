// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        PhysicalAddress,
        VirtualAddress,
    },
    pm::ProcessManager,
};
use ::alloc::vec::Vec;
use ::arch::mem::PAGE_SIZE;
use ::core::cmp;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        GuestSgSegment,
        SG_BULK_MAX_BYTES,
        SG_BULK_MAX_SEGMENTS,
    },
    mm::Address,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds a scatter/gather segment list for a user-space buffer.
///
/// The returned descriptors are fully chained: each non-final descriptor records the guest virtual
/// address of the descriptor that follows it, and the final descriptor terminates the chain.
///
/// # Parameters
///
/// - `pm`: Process manager used to translate user virtual addresses.
/// - `pid`: Identifier of the process that owns the buffer.
/// - `buffer`: Start virtual address of the user-space buffer.
/// - `transfer_len`: Number of bytes to transfer.
///
/// # Returns
///
/// Upon success, the scatter/gather segment list is returned. Upon failure, an error is returned
/// instead.
///
/// # Errors
///
/// This function returns an error if the transfer length is invalid, if a user virtual address
/// cannot be translated, if a segment field does not fit the wire format, or if the segment chain
/// would exceed the scatter/gather limits.
///
pub fn build_user_segments(
    pm: &ProcessManager,
    pid: ProcessIdentifier,
    buffer: VirtualAddress,
    transfer_len: usize,
) -> Result<Vec<GuestSgSegment>, Error> {
    let buffer_raw: usize = buffer.into_raw_value();
    if transfer_len == 0 {
        let reason: &str = "scatter/gather transfer length is zero";
        error!("{reason} (pid={pid:?}, buffer={buffer_raw:#x})");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }
    if transfer_len > SG_BULK_MAX_BYTES {
        let reason: &str = "scatter/gather transfer exceeds maximum byte count";
        error!(
            "{reason} (pid={pid:?}, buffer={buffer_raw:#x}, len={transfer_len}, \
             max={SG_BULK_MAX_BYTES})"
        );
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Bound the allocation up front: a transfer needs at most one segment per page it spans, capped
    // by the scatter/gather segment limit. Reserving that exact capacity keeps the kernel-heap
    // allocation bounded and avoids reallocation while the chain is built. Use fallible reservation
    // so a kernel-heap exhaustion is reported as an error instead of aborting.
    let first_page_offset: usize = buffer_raw & (PAGE_SIZE - 1);
    let span_pages: usize = (first_page_offset + transfer_len).div_ceil(PAGE_SIZE);
    let capacity: usize = cmp::min(span_pages, SG_BULK_MAX_SEGMENTS as usize);
    let mut segments: Vec<GuestSgSegment> = Vec::new();
    segments.try_reserve_exact(capacity).map_err(|_| {
        let reason: &str = "failed to allocate scatter/gather segment list";
        error!("{reason} (pid={pid:?}, buffer={buffer_raw:#x}, capacity={capacity})");
        Error::new(ErrorCode::OutOfMemory, reason)
    })?;

    let mut current: usize = buffer_raw;
    let mut remaining: usize = transfer_len;

    while remaining > 0 {
        let page_offset: usize = current & (PAGE_SIZE - 1);
        let chunk_len: usize = cmp::min(PAGE_SIZE - page_offset, remaining);
        let vaddr: VirtualAddress = VirtualAddress::from_raw_value(current);

        // Translate the user virtual address and keep the result strongly typed so the contiguity
        // check below operates on real physical addresses rather than raw integers.
        let paddr_raw: usize = pm.user_vaddr_to_paddr(pid, vaddr).inspect_err(|error| {
            error!(
                "failed to translate scatter/gather segment (pid={pid:?}, vaddr={current:#x}, \
                 error={error:?})"
            );
        })?;
        let paddr: PhysicalAddress =
            PhysicalAddress::from_raw_value(paddr_raw).inspect_err(|error| {
                error!(
                    "scatter/gather segment is not a valid physical address (pid={pid:?}, \
                     vaddr={current:#x}, paddr={paddr_raw:#x}, error={error:?})"
                );
            })?;
        let paddr_u32: u32 = u32::try_from(paddr.into_raw_value()).map_err(|_| {
            let reason: &str = "guest physical address exceeds u32";
            error!("{reason} (pid={pid:?}, paddr={paddr_raw:#x})");
            Error::new(ErrorCode::InvalidArgument, reason)
        })?;
        let chunk_len_u32: u32 = u32::try_from(chunk_len).map_err(|_| {
            let reason: &str = "scatter/gather segment length exceeds u32";
            error!("{reason} (pid={pid:?}, chunk_len={chunk_len})");
            Error::new(ErrorCode::InvalidArgument, reason)
        })?;

        if let Some(last) = segments.last_mut() {
            let last_gpa: PhysicalAddress =
                PhysicalAddress::from_raw_value(last.data_gpa() as usize).map_err(|error| {
                    let reason: &str = "segment is not a valid physical address";
                    error!(
                        "{reason} (pid={pid:?}, data_gpa={:#x}, error={error:?})",
                        last.data_gpa()
                    );
                    Error::new(ErrorCode::InvalidArgument, reason)
                })?;
            // The segments are contiguous when the previous one ends exactly where this one starts.
            // `checked_add` returns `None` when the previous segment ends at the top of physical
            // memory, in which case the two cannot be contiguous.
            let contiguous: bool = last_gpa.checked_add(last.data_len() as usize) == Some(paddr);
            if contiguous {
                let new_len: u32 = last.data_len().checked_add(chunk_len_u32).ok_or_else(|| {
                    let reason: &str = "segment length overflow";
                    error!(
                        "{reason} (pid={pid:?}, data_len={}, chunk_len={chunk_len_u32})",
                        last.data_len()
                    );
                    Error::new(ErrorCode::InvalidArgument, reason)
                })?;
                last.set_data_len(new_len);
            } else {
                if segments.len() == SG_BULK_MAX_SEGMENTS as usize {
                    let reason: &str = "scatter/gather transfer has too many segments";
                    error!(
                        "{reason} (pid={pid:?}, buffer={buffer_raw:#x}, len={transfer_len}, \
                         max={SG_BULK_MAX_SEGMENTS})"
                    );
                    return Err(Error::new(ErrorCode::InvalidArgument, reason));
                }
                segments.push(GuestSgSegment::new(
                    VirtualAddress::from_raw_value(0),
                    paddr_u32,
                    chunk_len_u32,
                ));
            }
        } else {
            segments.push(GuestSgSegment::new(
                VirtualAddress::from_raw_value(0),
                paddr_u32,
                chunk_len_u32,
            ));
        }

        current = current.checked_add(chunk_len).ok_or_else(|| {
            let reason: &str = "buffer address overflow";
            error!("{reason} (pid={pid:?}, current={current:#x}, chunk_len={chunk_len})");
            Error::new(ErrorCode::InvalidArgument, reason)
        })?;
        remaining -= chunk_len;
    }

    // Now that the segment vector has reached its final size and heap location, link each
    // descriptor to the next. Performing the chaining here, rather than in the stdio transport,
    // keeps all scatter/gather chain construction within the ipc subsystem.
    chain_segments(&mut segments)?;

    Ok(segments)
}

///
/// # Description
///
/// Links scatter/gather descriptors together in place.
///
/// Each non-final descriptor is updated to record the guest virtual address of the descriptor that
/// follows it; the final descriptor is terminated with a zero link. The descriptors must already
/// reside at their final memory location, as their addresses are captured into the chain.
///
/// # Parameters
///
/// - `segments`: Scatter/gather descriptors to link.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
/// # Errors
///
/// This function returns an error if a descriptor address does not fit the guest/host wire format.
///
fn chain_segments(segments: &mut [GuestSgSegment]) -> Result<(), Error> {
    for i in 0..segments.len() {
        let next: VirtualAddress = if i + 1 < segments.len() {
            VirtualAddress::try_from_ptr(
                &segments[i + 1] as *const GuestSgSegment,
                "scatter/gather segment descriptor address exceeds u32",
            )?
        } else {
            VirtualAddress::from_raw_value(0)
        };
        segments[i].set_next(next);
    }
    Ok(())
}
