// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use crate::hal::arch::x86::mem::gdt::Gdt;
use crate::{
    kcall::KcallResult,
    mm::Vmem,
    pm::ProcessManager,
};
#[cfg(target_arch = "aarch64")]
use ::sys::mm::Address;
use ::sys::{
    error::ErrorCode,
    mm::VirtualAddress,
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
/// Sets the base address for the user-space thread data area of a thread.
///
/// # Parameters
///
/// - `pid`: The process identifier of the calling process.
/// - `tid`: The thread identifier of the calling thread.
/// - `arg0`: The user-space thread data area pointer.
///
/// # Return Value
///
/// On successful completion, this function returns a status code for a successful kernel call. On
/// failure, this function returns an error code that indicates the reason of failure.
///
/// # Errors
///
/// This function fails with the following error codes:
///
/// - [`ErrorCode::InvalidArgument`]: The provided thread data area pointer is invalid.
/// - [`ErrorCode::NoSuchEntry`]: The specified process or thread does not exist.
/// - [`ErrorCode::ResourceBusy`]: The process manager is busy and cannot handle the request.
///
pub fn set_thread_data_area(
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    arg0: u32,
) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // Unpack arguments.
    let user_tda: VirtualAddress = VirtualAddress::from_raw_value(arg0 as usize);

    trace!("pid={pid:?}, tid={tid:?}, user_tda={user_tda:?}");

    // Check if thread-local storage does not lie within the user space.
    let user_tda: Option<VirtualAddress> = if user_tda != VirtualAddress::from_raw_value(0) {
        if !Vmem::is_user_addr(user_tda) {
            error!(
                "invalid base address for the user-space thread data area (tid={tid:?}, \
                 pid={pid:?}, user_tda={user_tda:?})"
            );
            return KcallResult::Error(ErrorCode::InvalidArgument.into());
        }

        Some(user_tda)
    } else {
        None
    };

    // Handle kernel call.
    match pm.set_thread_data_area(pid, tid, user_tda) {
        Ok(()) => {
            // Update the GDT entry immediately so the GS/FS base takes effect
            // without waiting for a context switch. This is critical for
            // thread_local variables (accessed via %gs on x86) to work in
            // single-threaded processes.
            //
            // SAFETY: We are in the kcall dispatcher, running in privileged
            // mode with interrupts disabled. Modifying the GDT is safe here.
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            {
                if let Some(tda_addr) = user_tda {
                    unsafe {
                        Gdt::set_thread_data_area(tda_addr.into());
                    }
                } else {
                    // Clear %gs/%fs so they do not reference a stale TDA.
                    unsafe {
                        Gdt::clear_thread_data_area_segments();
                    }
                }
            }

            #[cfg(target_arch = "aarch64")]
            {
                let tpidr_el0: usize = user_tda.map_or(0, |address| address.into_raw_value());
                unsafe {
                    core::arch::asm!(
                        "msr tpidr_el0, {value}",
                        value = in(reg) tpidr_el0,
                        options(nostack, preserves_flags),
                    );
                }
            }

            trace!("success (user_tda={user_tda:?})");
            KcallResult::ok()
        },

        Err(error) => {
            error!("{error:?}");
            KcallResult::Error(error.code.into())
        },
    }
}
