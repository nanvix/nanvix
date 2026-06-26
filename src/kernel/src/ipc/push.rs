// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "stdio")]
use crate::mm::{
    KernelPage,
    VirtMemoryManager,
};
#[cfg(feature = "stdio")]
use crate::pm::ProcessManager;
use crate::pm::SleepError;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Pushes data to a destination process using rendezvous synchronization.
///
/// When the destination is the kernel (linuxd), data is transferred via the vmbus data chunk
/// transfer path. The vmbus translates a single virtual address into one guest physical address and
/// treats the payload as physically contiguous. A buffer that lies within a single page is
/// therefore handed to the VMM directly (fast path), while a buffer that crosses a page boundary is
/// only *virtually* contiguous and is staged through a physically-contiguous kernel bounce page
/// (slow path). Either way the payload must fit within a single page.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `caller_tid`: Identifier of the calling thread.
/// - `destination_raw`: Raw identifier of the destination process.
/// - `destination_tid_raw`: Raw identifier of the destination thread.
/// - `buffer_raw`: Raw pointer to the buffer whose contents will be sent.
/// - `transfer_len_raw`: Raw length of data to be transferred.
///
/// # Returns
///
/// On successful completion, this function returns `()` after delivering the payload to the
/// destination.  On failure, an error code is returned instead.
///
pub fn push(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    destination_raw: u32,
    destination_tid_raw: u32,
    buffer_raw: usize,
    transfer_len_raw: u32,
) -> Result<(), SleepError> {
    let destination_pid: ProcessIdentifier =
        ProcessIdentifier::try_from(destination_raw).map_err(|error| {
            let reason: &str = "invalid destination process identifier";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 destination_raw={destination_raw}, error={error:?})"
            );
            SleepError::Generic(error)
        })?;

    let destination_tid: ThreadIdentifier = ThreadIdentifier::try_from(destination_tid_raw)
        .map_err(|error| {
            let reason: &str = "invalid destination thread identifier";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 destination_tid_raw={destination_tid_raw}, error={error:?})"
            );
            SleepError::Generic(error)
        })?;

    let transfer_len: usize = usize::try_from(transfer_len_raw).map_err(|_| {
        let reason: &str = "transfer length is too large";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
             destination_pid={destination_pid:?}, destination_tid={destination_tid:?}, \
             transfer_len_raw={transfer_len_raw})"
        );
        SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason))
    })?;

    trace!(
        "tid={:?}, pid={:?}, dst_tid={:?}, dst_pid={:?}, buffer={:#x}, len={}",
        caller_tid,
        caller_pid,
        destination_tid,
        destination_pid,
        buffer_raw,
        transfer_len
    );

    // When the destination is the kernel (linuxd), use the vmbus for data chunk transfer instead
    // of the rendezvous cross-process copy. The user buffer virtual address is translated to a
    // guest physical address so the VMM can directly read the data without an intermediate kernel
    // buffer copy.
    #[cfg(feature = "stdio")]
    if destination_pid == ProcessIdentifier::KERNEL {
        trace!(
            "push(): data chunk transfer via vmbus (caller_pid={caller_pid:?}, \
             caller_tid={caller_tid:?}, len={transfer_len})"
        );

        // The vmbus data chunk transfer path reads `transfer_len` physically-contiguous bytes
        // starting at a single translated guest-physical address. A user buffer is only
        // *virtually* contiguous, so one whose bytes spill past its first page cannot be handed to
        // the VMM directly: only the first page would be translated and the VMM would read into an
        // unrelated physical frame. Detect that case up front.
        let page_offset: usize = buffer_raw & (::arch::mem::PAGE_SIZE - 1);
        let crosses_page: bool =
            transfer_len > 0 && page_offset.saturating_add(transfer_len) > ::arch::mem::PAGE_SIZE;

        if !crosses_page {
            // Fast path: the buffer lies within a single page, so translate its address and let the
            // VMM read it in place with no intermediate copy.
            let pm: &ProcessManager = unsafe { ProcessManager::get() };
            let vaddr: crate::hal::mem::VirtualAddress =
                crate::hal::mem::VirtualAddress::from_raw_value(buffer_raw);
            let paddr: usize = pm
                .user_vaddr_to_paddr(caller_pid, vaddr)
                .map_err(SleepError::Generic)?;
            let gpa: u32 = u32::try_from(paddr).map_err(|_| {
                let reason: &str = "guest physical address exceeds u32";
                error!(
                    "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                     paddr={paddr:#x})"
                );
                SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason))
            })?;

            return crate::stdio::write_bulk(
                caller_pid,
                caller_tid,
                destination_pid,
                destination_tid,
                gpa,
                transfer_len_raw,
            )
            .map_err(SleepError::Generic);
        }

        // Slow path: the buffer straddles a page boundary. Stage it through a physically-contiguous
        // kernel bounce page so the VMM still sees a single contiguous region. The payload itself
        // must fit within one page (every kernel-peer caller transfers at most a page of data).
        if transfer_len > ::arch::mem::PAGE_SIZE {
            let reason: &str = "bulk push payload exceeds one page";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 len={transfer_len})"
            );
            return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
        }

        // Allocate the bounce page. A kernel frame is a single physical frame: contiguous and
        // identity-mapped, so its guest-physical address can be handed to the VMM directly.
        let bounce: KernelPage = unsafe { VirtMemoryManager::get_mut() }
            .alloc_kpage(true)
            .map_err(SleepError::Generic)?;
        let bounce_vaddr: crate::hal::mem::VirtualAddress =
            crate::hal::mem::VirtualAddress::from_raw_value(bounce.base().into_raw_value());
        let gpa: u32 = u32::try_from(bounce.frame_address().into_raw_value()).map_err(|_| {
            let reason: &str = "bounce page guest physical address exceeds u32";
            error!("{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?})");
            SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason))
        })?;

        // Copy the page-straddling user buffer into the contiguous bounce page.
        unsafe { ProcessManager::get_mut() }
            .vmcopy_from_user(
                caller_pid,
                bounce_vaddr,
                crate::hal::mem::VirtualAddress::from_raw_value(buffer_raw),
                transfer_len,
            )
            .map_err(SleepError::Generic)?;

        // Hand the bounce page to the VMM. The port write traps synchronously into the VMM, which
        // reads the page before this call returns, so the bounce page may be released right after.
        crate::stdio::write_bulk(
            caller_pid,
            caller_tid,
            destination_pid,
            destination_tid,
            gpa,
            transfer_len_raw,
        )
        .map_err(SleepError::Generic)?;

        // `bounce` is dropped here, freeing the kernel frame.
        return Ok(());
    }

    super::rendezvous::do_push(
        caller_pid,
        caller_tid,
        destination_pid,
        destination_tid,
        buffer_raw,
        transfer_len,
    )
}
