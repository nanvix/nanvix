// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "stdio")]
use crate::hal::mem::VirtualAddress;
use crate::pm::{
    ProcessManager,
    SleepError,
};
#[cfg(feature = "stdio")]
use ::sys::error::{
    Error,
    ErrorCode,
};
#[cfg(feature = "stdio")]
use ::sys::ipc::{
    GuestSgBulkKind,
    GuestSgSegment,
    SG_BULK_MAX_BYTES,
};
use ::sys::{
    ipc::PullArgs,
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
/// When the sender is the host I/O backend, data is transferred via the vmbus scatter/gather data
/// chunk transfer path.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `caller_tid`: Identifier of the calling thread.
/// - `args_ptr`: User-space pointer to the [`PullArgs`] descriptor.
///
/// # Returns
///
/// On successful completion, this function returns the number of bytes pulled from the sender
/// process.  On failure, an error code is returned instead.
///
pub fn pull(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    args_ptr: u32,
) -> Result<usize, SleepError> {
    // Copy the argument descriptor from user space into kernel space.
    let mut args: PullArgs = PullArgs::zeroed();
    {
        // SAFETY: the process manager is initialized and access is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        crate::pm::copy_from_user(pm, caller_pid, &mut args, args_ptr as *const PullArgs)
            .map_err(SleepError::Generic)?;
    }

    let sender_pid: ProcessIdentifier = args.src_pid;
    let sender_tid: ThreadIdentifier = args.src_tid;
    let buffer_raw: usize = args.buffer as usize;
    let transfer_len: usize = args.len as usize;

    // Validate the sender identifiers copied from user space. The by-pointer ABI lets a caller place
    // an arbitrary raw value in the descriptor, so reject the negative/sentinel identifiers that the
    // previous register-based ABI rejected while converting an unsigned register value into an
    // identifier. Converting the identifier back to a `u32` fails for exactly those negative values,
    // so invalid identifiers keep failing early and predictably with `InvalidArgument`.
    u32::try_from(sender_pid).map_err(|error| {
        let reason: &str = "invalid sender process identifier";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
             sender_pid={sender_pid:?}, error={error:?})"
        );
        SleepError::Generic(error)
    })?;
    u32::try_from(sender_tid).map_err(|error| {
        let reason: &str = "invalid sender thread identifier";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
             sender_tid={sender_tid:?}, error={error:?})"
        );
        SleepError::Generic(error)
    })?;

    let timeout: super::rendezvous::RendezvousTimeout =
        super::rendezvous::RendezvousTimeout::resolve(args.timeout).map_err(SleepError::Generic)?;

    trace!(
        "tid={:?}, pid={:?}, src_tid={:?}, src_pid={:?}, buffer={:#x}, len={}",
        caller_tid,
        caller_pid,
        sender_tid,
        sender_pid,
        buffer_raw,
        transfer_len
    );

    // When the source is the host I/O backend, use the vmbus for data chunk transfer instead of the
    // rendezvous cross-process copy. The user buffer virtual address is translated to a guest
    // physical address so the VMM can write data directly into guest physical memory without an
    // intermediate kernel buffer copy. After the transfer completes the data is already in place.
    #[cfg(feature = "stdio")]
    if sender_pid == ProcessIdentifier::KERNEL {
        if transfer_len == 0 || transfer_len > SG_BULK_MAX_BYTES {
            let reason: &str = "invalid scatter/gather bulk pull length";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 buffer={buffer_raw:#x}, len={transfer_len})"
            );
            return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
        }

        trace!(
            "pull(): data chunk transfer via vmbus (caller_pid={caller_pid:?}, \
             caller_tid={caller_tid:?}, len={transfer_len})"
        );

        // SAFETY: IPC runs after the process manager is initialized, and the process manager
        // provides the synchronization needed for address translation.
        let pm: &ProcessManager = unsafe { ProcessManager::get() };
        let segments: ::alloc::vec::Vec<GuestSgSegment> = super::sg::build_user_segments(
            pm,
            caller_pid,
            VirtualAddress::from_raw_value(buffer_raw),
            transfer_len,
        )
        .map_err(SleepError::Generic)?;

        crate::stdio::write_bulk(
            caller_pid,
            caller_tid,
            sender_pid,
            sender_tid,
            GuestSgBulkKind::Pull,
            &segments,
            args.len,
        )
        .map_err(SleepError::Generic)?;

        // Register a pending bulk pull and sleep until the completion arrives. UserVM scatters the
        // response into the guest physical segments before waking this thread. The deadline bounds
        // the wait so a slow or wedged host cannot block the guest thread forever.
        //
        // NOTE: a finite deadline does not cancel the in-flight host transfer, so it is safe only
        // against a non-responding host — a slow host that replies after the deadline still scatters
        // into the caller's (possibly freed or reused) buffer. See the caveat on
        // `bulk_pull::register_and_sleep` and issue #2908
        // (https://github.com/nanvix/nanvix/issues/2908). All current callers use the infinite
        // variant.
        return super::bulk_pull::register_and_sleep(caller_tid, timeout.deadline());
    }

    super::rendezvous::do_pull(
        caller_pid,
        caller_tid,
        sender_pid,
        sender_tid,
        buffer_raw,
        transfer_len,
        timeout,
    )
}
