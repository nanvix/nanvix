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
    ipc::PushArgs,
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
/// When the destination is the host I/O backend, data is transferred via the vmbus scatter/gather
/// data chunk transfer path.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `caller_tid`: Identifier of the calling thread.
/// - `args_ptr`: User-space pointer to the [`PushArgs`] descriptor.
///
/// # Returns
///
/// On successful completion, this function returns `()` after delivering the payload to the
/// destination.  On failure, an error code is returned instead.
///
pub fn push(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    args_ptr: u32,
) -> Result<(), SleepError> {
    // Copy the argument descriptor from user space into kernel space.
    let mut args: PushArgs = PushArgs::zeroed();
    {
        // SAFETY: the process manager is initialized and access is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        crate::pm::copy_from_user(pm, caller_pid, &mut args, args_ptr as *const PushArgs)
            .map_err(SleepError::Generic)?;
    }

    let destination_pid: ProcessIdentifier = args.dst_pid;
    let destination_tid: ThreadIdentifier = args.dst_tid;
    let buffer_raw: usize = args.buffer as usize;
    let transfer_len: usize = args.len as usize;

    // Validate the destination identifiers copied from user space. The by-pointer ABI lets a caller
    // place an arbitrary raw value in the descriptor, so reject the negative/sentinel identifiers
    // that the previous register-based ABI rejected while converting an unsigned register value into
    // an identifier. Converting the identifier back to a `u32` fails for exactly those negative
    // values, so invalid identifiers keep failing early and predictably with `InvalidArgument`.
    u32::try_from(destination_pid).map_err(|error| {
        let reason: &str = "invalid destination process identifier";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
             destination_pid={destination_pid:?}, error={error:?})"
        );
        SleepError::Generic(error)
    })?;
    u32::try_from(destination_tid).map_err(|error| {
        let reason: &str = "invalid destination thread identifier";
        error!(
            "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
             destination_tid={destination_tid:?}, error={error:?})"
        );
        SleepError::Generic(error)
    })?;

    let timeout: super::rendezvous::RendezvousTimeout =
        super::rendezvous::RendezvousTimeout::resolve(args.timeout).map_err(SleepError::Generic)?;

    trace!(
        "tid={:?}, pid={:?}, dst_tid={:?}, dst_pid={:?}, buffer={:#x}, len={}",
        caller_tid,
        caller_pid,
        destination_tid,
        destination_pid,
        buffer_raw,
        transfer_len
    );

    // When the destination is the host I/O backend, use the vmbus for data chunk transfer instead
    // of the rendezvous cross-process copy. The user buffer virtual address is translated to a
    // guest physical address so the VMM can directly read the data without an intermediate kernel
    // buffer copy.
    #[cfg(feature = "stdio")]
    if destination_pid == ProcessIdentifier::KERNEL {
        if transfer_len == 0 || transfer_len > SG_BULK_MAX_BYTES {
            let reason: &str = "invalid scatter/gather bulk push length";
            error!(
                "{reason} (caller_pid={caller_pid:?}, caller_tid={caller_tid:?}, \
                 buffer={buffer_raw:#x}, len={transfer_len})"
            );
            return Err(SleepError::Generic(Error::new(ErrorCode::InvalidArgument, reason)));
        }

        trace!(
            "push(): data chunk transfer via vmbus (caller_pid={caller_pid:?}, \
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

        return crate::stdio::write_bulk(
            caller_pid,
            caller_tid,
            destination_pid,
            destination_tid,
            GuestSgBulkKind::Push,
            &segments,
            args.len,
        )
        .map_err(SleepError::Generic);
    }

    super::rendezvous::do_push(
        caller_pid,
        caller_tid,
        destination_pid,
        destination_tid,
        buffer_raw,
        transfer_len,
        timeout,
    )
}
