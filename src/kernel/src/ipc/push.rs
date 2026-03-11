// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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
/// When the destination is the kernel (linuxd), data is transferred via the vmbus data chunk transfer
/// path. In this mode, the user buffer must reside entirely within a single physical page because
/// the vmbus translates only the first page's virtual address to a guest physical address.
/// Callers must split larger transfers into page-aligned chunks at the syscall library layer.
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
        caller_tid, caller_pid, destination_tid, destination_pid, buffer_raw, transfer_len
    );

    // When the destination is the kernel (linuxd), use the vmbus for data chunk transfer instead of the
    // rendezvous cross-process copy. The user buffer virtual address is translated to a guest
    // physical address so the VMM can directly read the data without an intermediate kernel buffer
    // copy.
    #[cfg(feature = "stdio")]
    if destination_pid == ProcessIdentifier::KERNEL {
        cfg_if::cfg_if! {
            if #[cfg(all(feature = "microvm", feature = "ring-buffer"))] {
                let segment_count: usize = crate::ring::fixed_buffer_count_for_len(transfer_len);
                let reservation: crate::ring::FixedBufferReservation =
                    crate::ring::get_or_alloc_thread_fixed_buffers(caller_tid, segment_count)
                        .map_err(SleepError::Generic)?;
                let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
                let mut copied: usize = 0;

                for &buffer_id in reservation.ids() {
                    if copied >= transfer_len {
                        break;
                    }
                    let chunk_len: usize = core::cmp::min(
                        transfer_len - copied,
                        ::nvx_ring::FIXED_BUF_SIZE,
                    );
                    let fixed_buffer_vaddr: usize =
                        crate::ring::fixed_buffer_vaddr(buffer_id).map_err(SleepError::Generic)?;
                    let dst: crate::hal::mem::VirtualAddress =
                        crate::hal::mem::VirtualAddress::from_raw_value(fixed_buffer_vaddr);
                    let src: crate::hal::mem::VirtualAddress =
                        crate::hal::mem::VirtualAddress::from_raw_value(buffer_raw + copied);
                    pm.vmcopy_from_user(caller_pid, dst, src, chunk_len)
                        .map_err(SleepError::Generic)?;
                    copied += chunk_len;
                }

                let mut sent: usize = 0;
                for &buffer_id in reservation.ids() {
                    if sent >= transfer_len {
                        break;
                    }
                    let chunk_len: usize = core::cmp::min(
                        transfer_len - sent,
                        ::nvx_ring::FIXED_BUF_SIZE,
                    );

                    trace!(
                        "push(): fixed-buffer transfer via ring (caller_pid={caller_pid:?}, \
                         caller_tid={caller_tid:?}, buffer_id={buffer_id}, offset={sent}, \
                         len={chunk_len})"
                    );

                    crate::stdio::write_fixed_bulk(
                        caller_pid,
                        caller_tid,
                        destination_pid,
                        destination_tid,
                        buffer_id,
                        chunk_len as u32,
                    )
                    .map_err(SleepError::Generic)?;
                    sent += chunk_len;
                }

                return Ok(());
            } else {
                // Reject transfers that cross a page boundary. The vmbus data chunk transfer path translates
                // only the first page's virtual address to a guest physical address, so the entire buffer
                // must reside within a single physical page.
                if transfer_len > 0 {
                    let page_offset: usize = buffer_raw & (::arch::mem::PAGE_SIZE - 1);
                    if page_offset.saturating_add(transfer_len) > ::arch::mem::PAGE_SIZE {
                        let reason: &str = "bulk push buffer crosses a page boundary";
                        error!(
                            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                             buffer={buffer_raw:#x}, len={transfer_len}, page_offset={page_offset})"
                        );
                        return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
                    }
                }

                trace!(
                    "push(): data chunk transfer via vmbus (caller_pid={caller_pid:?}, \
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
        }
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
