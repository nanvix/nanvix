// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(all(feature = "stdio", not(all(feature = "microvm", feature = "ring-buffer"))))]
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
/// When the sender is the kernel (linuxd), data is transferred via the vmbus data chunk transfer path.
/// In this mode, the user buffer must reside entirely within a single physical page because the
/// vmbus translates only the first page's virtual address to a guest physical address. Callers
/// must split larger transfers into page-aligned chunks at the syscall library layer.
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
        cfg_if::cfg_if! {
            if #[cfg(all(feature = "microvm", feature = "ring-buffer"))] {
                let buffer_id: u32 =
                    crate::ring::get_or_alloc_thread_fixed_buffer(caller_tid).map_err(SleepError::Generic)?;

                trace!(
                    "pull(): fixed-buffer transfer via ring (caller_pid={caller_pid:?}, \
                     caller_tid={caller_tid:?}, buffer_id={buffer_id}, len={transfer_len})"
                );

                crate::stdio::write_fixed_bulk(
                    caller_pid,
                    caller_tid,
                    sender_pid,
                    sender_tid,
                    buffer_id,
                    transfer_len_raw,
                )
                .map_err(SleepError::Generic)?;

                return super::fixed_pull::register_and_sleep(
                    caller_pid,
                    caller_tid,
                    buffer_raw,
                    transfer_len,
                );
            } else {
                // Reject transfers that cross a page boundary. The vmbus data chunk transfer path translates
                // only the first page's virtual address to a guest physical address, so the entire buffer
                // must reside within a single physical page.
                if transfer_len > 0 {
                    let page_offset: usize = buffer_raw & (::arch::mem::PAGE_SIZE - 1);
                    if page_offset.saturating_add(transfer_len) > ::arch::mem::PAGE_SIZE {
                        let reason: &str = "bulk pull buffer crosses a page boundary";
                        error!(
                            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                             buffer={buffer_raw:#x}, len={transfer_len}, page_offset={page_offset})"
                        );
                        return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
                    }
                }

                trace!(
                    "pull(): data chunk transfer via vmbus (caller_pid={caller_pid:?}, \
                     caller_tid={caller_tid:?}, len={transfer_len})"
                );

                // Translate user virtual address to guest physical address.
                let pm: &ProcessManager = unsafe { ProcessManager::get() };
                let vaddr: crate::hal::mem::VirtualAddress =
                    crate::hal::mem::VirtualAddress::from_raw_value(buffer_raw);
                let paddr: usize = pm
                    .user_vaddr_to_paddr(caller_pid, vaddr)
                    .map_err(SleepError::Generic)?;
                let gpa: u32 = u32::try_from(paddr).map_err(|_| {
                    let reason: &str = "guest physical address exceeds u32";
                    error!(
                        "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, paddr={paddr:#x})"
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

                // Register a pending bulk pull and sleep until the completion arrives. The VMM writes
                // data directly into the guest physical page backing the user buffer, so no post-wake
                // copy is needed.
                return super::bulk_pull::register_and_sleep(caller_tid);
            }
        }
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
