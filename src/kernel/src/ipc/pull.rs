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
/// Pulls data from a sender process using rendezvous synchronization.
///
/// When the sender is the kernel (linuxd), data is transferred via the vmbus data chunk transfer
/// path. The vmbus translates a single virtual address into one guest physical address and treats
/// the payload as physically contiguous. A buffer that lies within a single page is therefore
/// handed to the VMM directly (fast path), while a buffer that crosses a page boundary is only
/// *virtually* contiguous and is staged through a physically-contiguous kernel bounce page (slow
/// path). Either way the payload must fit within a single page.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `caller_tid`: Identifier of the calling thread.
/// - `sender_raw`: Raw identifier of the sender process.
/// - `sender_tid_raw`: Raw identifier of the sender thread.
/// - `buffer_raw`: Raw pointer to the buffer where data will be stored.
/// - `transfer_len_raw`: Raw length of data to be transferred.
///
/// # Returns
///
/// On successful completion, this function returns the number of bytes pulled from the sender
/// process.  On failure, an error code is returned instead.
///
pub fn pull(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    sender_raw: u32,
    sender_tid_raw: u32,
    buffer_raw: usize,
    transfer_len_raw: u32,
) -> Result<usize, SleepError> {
    // Convert sender process identifier.
    let sender_pid: ProcessIdentifier =
        ProcessIdentifier::try_from(sender_raw).map_err(|error| {
            let reason: &str = "invalid sender process identifier";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 sender_raw={sender_raw}, error={error:?})"
            );
            SleepError::Generic(error)
        })?;

    // Convert sender thread identifier.
    let sender_tid: ThreadIdentifier =
        ThreadIdentifier::try_from(sender_tid_raw).map_err(|error| {
            let reason: &str = "invalid sender thread identifier";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 sender_tid_raw={sender_tid_raw}, error={error:?})"
            );
            SleepError::Generic(error)
        })?;

    // Convert transfer length.
    let transfer_len: usize = usize::try_from(transfer_len_raw).map_err(|_| {
        let reason: &str = "transfer length is too large";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
             sender_pid={sender_pid:?}, sender_tid={sender_tid:?}, \
             transfer_len_raw={transfer_len_raw})"
        );
        SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason))
    })?;

    trace!(
        "tid={:?}, pid={:?}, src_tid={:?}, src_pid={:?}, buffer={:#x}, len={}",
        caller_tid,
        caller_pid,
        sender_tid,
        sender_pid,
        buffer_raw,
        transfer_len
    );

    // When the source is the kernel (linuxd), use the vmbus for data chunk transfer instead of the
    // rendezvous cross-process copy. The user buffer virtual address is translated to a guest
    // physical address so the VMM can write data directly into guest physical memory without an
    // intermediate kernel buffer copy. After the transfer completes the data is already in place.
    #[cfg(feature = "stdio")]
    if sender_pid == ProcessIdentifier::KERNEL {
        trace!(
            "pull(): data chunk transfer via vmbus (caller_pid={caller_pid:?}, \
             caller_tid={caller_tid:?}, len={transfer_len})"
        );

        // The vmbus data chunk transfer path writes `transfer_len` physically-contiguous bytes
        // starting at a single translated guest-physical address. A user buffer is only
        // *virtually* contiguous, so one whose bytes spill past its first page cannot be handed to
        // the VMM directly: only the first page would be translated and the VMM would write into an
        // unrelated physical frame. Detect that case up front.
        let page_offset: usize = buffer_raw & (::arch::mem::PAGE_SIZE - 1);
        let crosses_page: bool =
            transfer_len > 0 && page_offset.saturating_add(transfer_len) > ::arch::mem::PAGE_SIZE;

        if !crosses_page {
            // Fast path: the buffer lies within a single page, so the VMM writes directly into the
            // guest physical page backing it and no post-wake copy is needed.
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

            crate::stdio::write_bulk(
                caller_pid,
                caller_tid,
                sender_pid,
                sender_tid,
                gpa,
                transfer_len_raw,
            )
            .map_err(SleepError::Generic)?;

            // Register a pending bulk pull and sleep until the completion arrives.
            return super::bulk_pull::register_and_sleep(caller_tid);
        }

        // Slow path: the buffer straddles a page boundary. Receive into a physically-contiguous
        // kernel bounce page so the VMM still sees a single contiguous region, then scatter the
        // delivered bytes back into the page-straddling user buffer. The payload itself must fit
        // within one page (every kernel-peer caller transfers at most a page of data).
        if transfer_len > ::arch::mem::PAGE_SIZE {
            let reason: &str = "bulk pull payload exceeds one page";
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

        crate::stdio::write_bulk(
            caller_pid,
            caller_tid,
            sender_pid,
            sender_tid,
            gpa,
            transfer_len_raw,
        )
        .map_err(SleepError::Generic)?;

        // Sleep until the VMM has written the payload into the bounce page. `bounce` lives on this
        // thread's kernel stack across the sleep, so it is still valid on wake.
        // Clamp the completion length to what the caller actually requested: never copy out or
        // report more bytes than the user buffer holds, even if the completion claims a larger
        // transfer (e.g. a buggy or malicious peer).
        let actual_len: usize = super::bulk_pull::register_and_sleep(caller_tid)?.min(transfer_len);

        // Scatter the delivered bytes from the contiguous bounce page back into the user buffer.
        if actual_len > 0 {
            unsafe { ProcessManager::get_mut() }
                .vmcopy_to_user(
                    caller_pid,
                    crate::hal::mem::VirtualAddress::from_raw_value(buffer_raw),
                    bounce_vaddr,
                    actual_len,
                )
                .map_err(SleepError::Generic)?;
        }

        // `bounce` is dropped here, freeing the kernel frame.
        return Ok(actual_len);
    }

    super::rendezvous::do_pull(
        caller_pid,
        caller_tid,
        sender_pid,
        sender_tid,
        buffer_raw,
        transfer_len,
    )
}
