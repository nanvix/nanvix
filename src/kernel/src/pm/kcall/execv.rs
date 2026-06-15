// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
    mm::Vmem,
    pm::{
        self,
        ProcessManager,
    },
};
use ::arch::mem::PAGE_SIZE;
use ::core::mem::size_of;
use ::sys::{
    error::ErrorCode,
    mm::{
        Address,
        VirtualAddress,
    },
    pm::{
        ExecvArgs,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for replacing the image of the calling process (POSIX `execv()`).
///
/// The kernel performs no filesystem I/O, so the caller is responsible for having loaded the ELF
/// image into its own address space (for example, via `mmap`). This handler validates the
/// [`ExecvArgs`] descriptor copied from user space, then hands off to [`ProcessManager::exec`],
/// which streams the image directly from the caller's address space, builds the new image, and
/// switches into it. There is no artificial image-size limit: the image is bounded only by what
/// fits in the caller's address space and the available physical memory.
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `arg0`: Pointer to the [`ExecvArgs`] structure in user space.
///
/// # Returns
///
/// On success this never returns: the calling process's image is replaced. On failure a
/// [`KcallResult`] carrying the error code is returned and the calling process is left intact.
///
pub fn execv(pid: ProcessIdentifier, arg0: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // Unpack kernel call arguments.
    let unsafe_args: VirtualAddress = VirtualAddress::from_raw_value(arg0 as usize);

    // Check that the argument structure lies entirely in user space.
    if !Vmem::is_user_region(unsafe_args, size_of::<ExecvArgs>()) {
        let reason: &str = "execv args do not lie in user space";
        error!("{reason} (args={unsafe_args:?})");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Copy the argument structure from user space.
    let mut args: ExecvArgs = ExecvArgs::default();
    if let Err(error) =
        pm::copy_from_user(pm, pid, &mut args, unsafe_args.into_raw_value() as *const ExecvArgs)
    {
        let reason: &str = "failed to copy execv args from user space";
        error!("{reason} (error={error:?})");
        return KcallResult::Error(error.code.into());
    }

    // Validate the ELF image size. The only bound is that the image must be non-empty and lie
    // within the caller's user address space (checked next); there is no fixed maximum.
    if args.elf_len == 0 {
        let reason: &str = "execv ELF image is empty";
        error!("{reason}");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Validate that the ELF image lies entirely in user space.
    if !Vmem::is_user_region(args.elf_ptr, args.elf_len) {
        let reason: &str = "execv ELF image does not lie in user space";
        error!("{reason} (elf_ptr={:?}, elf_len={})", args.elf_ptr, args.elf_len);
        return KcallResult::Error(ErrorCode::BadAddress.into());
    }

    // Validate the argument and environment string lengths. Each must fit, together with its NUL
    // terminator, within a single page (mirroring the limit enforced when building the image).
    if args.args_len >= PAGE_SIZE || args.env_len >= PAGE_SIZE {
        let reason: &str = "execv argument or environment string is too long";
        error!("{reason} (args_len={}, env_len={})", args.args_len, args.env_len);
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Validate that the argument and environment strings lie in user space (when non-empty).
    if args.args_len > 0 && !Vmem::is_user_region(args.args_ptr, args.args_len) {
        let reason: &str = "execv argument string does not lie in user space";
        error!("{reason} (args_ptr={:?}, args_len={})", args.args_ptr, args.args_len);
        return KcallResult::Error(ErrorCode::BadAddress.into());
    }
    if args.env_len > 0 && !Vmem::is_user_region(args.env_ptr, args.env_len) {
        let reason: &str = "execv environment string does not lie in user space";
        error!("{reason} (env_ptr={:?}, env_len={})", args.env_ptr, args.env_len);
        return KcallResult::Error(ErrorCode::BadAddress.into());
    }

    // Hand off to the process manager. On success this never returns: the calling process's image
    // is replaced and control transfers to the new image. Only a failure surfaces here.
    // SAFETY: the calling thread is not the kernel, it holds no reference to the process manager,
    // access is synchronized, and the processor is running with interrupts disabled in privileged
    // mode while servicing this kernel call.
    let error: ::sys::error::Error = unsafe { ProcessManager::exec(pid, args) };
    KcallResult::Error(error.code.into())
}
