// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![feature(never_type)]
#![no_std]

//! Nanvix unmangled-symbol shim for `sys::kcall::*`.
//!
//! This crate exists solely to expose the kernel-call thunks defined in
//! the `sys::kcall` module hierarchy as unmangled (`#[no_mangle]`) globals
//! so that consumers can resolve them at link time by their canonical
//! names (`__kcall_*`, `_do_exit_thread`, `_do_start_thread`,
//! `__kcall_snapshot`).
//!
//! Most exports use Rust ABI (taking and returning Rust types such as
//! `Result<T, Error>`, `&mut T`, `Duration`, etc.) -- only
//! `_do_exit_thread`, `__kcall_snapshot`, and the `_do_start_thread`
//! `global_asm!` stub are `extern "C"`.  The `#[no_mangle]` attribute
//! affects the symbol's *name*, not its calling convention: this crate
//! is therefore a link-time-resolution shim, not a C-callable ABI.
//! Consumers that link these symbols by `extern` declaration must
//! match the Rust ABI signatures.
//!
//! Splitting the unmangled surface into its own crate avoids the
//! duplicate-symbol issue that arose when `libnvx_crt0.a` and
//! `libposix.a` BOTH baked the `sys` crate's `#[no_mangle]` exports
//! into their static archives.  Now only the consumers that explicitly
//! depend on `sys-ffi` (notably `libposix`) end up with the unmangled
//! symbols in their archive; `libnvx_crt0` stays free of them.
//!
//! Each wrapper here is a thin trampoline that immediately forwards to
//! the Rust-mangled equivalent in `sys::kcall::*`.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    option::Option,
    time::Duration,
};
use ::sys::{
    error::Error,
    event::{
        Event,
        EventCtrlRequest,
        EventDescriptor,
    },
    ipc::Message,
    mm::{
        AccessPermission,
        MmioRegionInfo,
        VirtualAddress,
    },
    pm::{
        Capability,
        ConditionAddress,
        MutexAddress,
        ProcessIdentifier,
        ThreadCreateArgs,
        ThreadIdentifier,
    },
    time::SystemTime,
};

//==================================================================================================
// debug
//==================================================================================================

#[unsafe(no_mangle)]
pub fn __kcall_debug(buf: *const u8, size: usize) -> Result<(), Error> {
    ::sys::kcall::debug::__kcall_debug(buf, size)
}

//==================================================================================================
// event
//==================================================================================================

#[unsafe(no_mangle)]
pub fn __kcall_resume(event: EventDescriptor) -> Result<(), Error> {
    ::sys::kcall::event::__kcall_resume(event)
}

#[unsafe(no_mangle)]
pub fn __kcall_evctrl(ev: Event, req: EventCtrlRequest) -> Result<(), Error> {
    ::sys::kcall::event::__kcall_evctrl(ev, req)
}

//==================================================================================================
// ipc
//==================================================================================================

#[unsafe(no_mangle)]
pub fn __kcall_send(message: &Message) -> Result<(), Error> {
    ::sys::kcall::ipc::__kcall_send(message)
}

#[unsafe(no_mangle)]
pub fn __kcall_recv() -> Result<Message, Error> {
    ::sys::kcall::ipc::__kcall_recv()
}

#[unsafe(no_mangle)]
pub fn __kcall_push(
    destination_pid: ProcessIdentifier,
    destination_tid: ThreadIdentifier,
    buffer: &[u8],
) -> Result<(), Error> {
    ::sys::kcall::ipc::__kcall_push(destination_pid, destination_tid, buffer)
}

#[unsafe(no_mangle)]
pub fn __kcall_pull(
    sender_pid: ProcessIdentifier,
    sender_tid: ThreadIdentifier,
    buffer: &mut [u8],
) -> Result<usize, Error> {
    ::sys::kcall::ipc::__kcall_pull(sender_pid, sender_tid, buffer)
}

//==================================================================================================
// mm
//==================================================================================================

#[unsafe(no_mangle)]
pub fn __kcall_mmap(
    pid: ProcessIdentifier,
    vaddr: VirtualAddress,
    npages: usize,
    access: AccessPermission,
) -> Result<(), Error> {
    ::sys::kcall::mm::__kcall_mmap(pid, vaddr, npages, access)
}

#[unsafe(no_mangle)]
pub fn __kcall_munmap(pid: ProcessIdentifier, vaddr: VirtualAddress) -> Result<(), Error> {
    ::sys::kcall::mm::__kcall_munmap(pid, vaddr)
}

#[unsafe(no_mangle)]
pub fn __kcall_mprotect(
    pid: ProcessIdentifier,
    vaddr: VirtualAddress,
    access: AccessPermission,
) -> Result<(), Error> {
    ::sys::kcall::mm::__kcall_mprotect(pid, vaddr, access)
}

#[unsafe(no_mangle)]
pub fn __kcall_mmio_alloc(tag: u64) -> Result<(), Error> {
    ::sys::kcall::mm::__kcall_mmio_alloc(tag)
}

#[unsafe(no_mangle)]
pub fn __kcall_mmio_free(tag: u64) -> Result<(), Error> {
    ::sys::kcall::mm::__kcall_mmio_free(tag)
}

#[unsafe(no_mangle)]
pub fn __kcall_mmio_info(tag: u64) -> Result<MmioRegionInfo, Error> {
    ::sys::kcall::mm::__kcall_mmio_info(tag)
}

//==================================================================================================
// pm
//==================================================================================================

#[unsafe(no_mangle)]
pub fn __kcall_getpid() -> Result<ProcessIdentifier, Error> {
    ::sys::kcall::pm::__kcall_getpid()
}

#[unsafe(no_mangle)]
pub fn __kcall_gettid() -> Result<ThreadIdentifier, Error> {
    ::sys::kcall::pm::__kcall_gettid()
}

#[unsafe(no_mangle)]
pub fn __kcall_exit(status: i32) -> Result<!, Error> {
    ::sys::kcall::pm::__kcall_exit(status)
}

#[unsafe(no_mangle)]
pub fn __kcall_capctl(capability: Capability, value: bool) -> Result<(), Error> {
    ::sys::kcall::pm::__kcall_capctl(capability, value)
}

#[unsafe(no_mangle)]
pub fn __kcall_terminate(pid: ProcessIdentifier) -> Result<(), Error> {
    ::sys::kcall::pm::__kcall_terminate(pid)
}

#[unsafe(no_mangle)]
pub extern "C" fn _do_exit_thread(status: usize) -> ! {
    ::sys::kcall::pm::_do_exit_thread(status)
}

#[unsafe(no_mangle)]
pub fn __kcall_create_thread(args: &mut ThreadCreateArgs) -> Result<ThreadIdentifier, Error> {
    ::sys::kcall::pm::__kcall_create_thread(args)
}

#[unsafe(no_mangle)]
pub fn __kcall_exit_thread(status: usize) -> Result<!, Error> {
    ::sys::kcall::pm::__kcall_exit_thread(status)
}

#[unsafe(no_mangle)]
pub fn __kcall_join_thread(tid: ThreadIdentifier, retval: &mut usize) -> Result<(), Error> {
    ::sys::kcall::pm::__kcall_join_thread(tid, retval)
}

#[unsafe(no_mangle)]
pub fn __kcall_detach_thread(tid: ThreadIdentifier) -> Result<(), Error> {
    ::sys::kcall::pm::__kcall_detach_thread(tid)
}

#[unsafe(no_mangle)]
pub fn __kcall_duplicate(args: &ThreadCreateArgs) -> Result<ProcessIdentifier, Error> {
    ::sys::kcall::pm::__kcall_duplicate(args)
}

#[unsafe(no_mangle)]
pub fn __kcall_lock_mutex(
    mutex_addr: MutexAddress,
    timeout: Option<SystemTime>,
) -> Result<(), Error> {
    ::sys::kcall::pm::__kcall_lock_mutex(mutex_addr, timeout)
}

#[unsafe(no_mangle)]
pub fn __kcall_unlock_mutex(mutex_addr: MutexAddress) -> Result<(), Error> {
    ::sys::kcall::pm::__kcall_unlock_mutex(mutex_addr)
}

#[unsafe(no_mangle)]
pub fn __kcall_signal_cond(
    cond_addr: ConditionAddress,
    broadcast: bool,
) -> Result<usize, Error> {
    ::sys::kcall::pm::__kcall_signal_cond(cond_addr, broadcast)
}

#[unsafe(no_mangle)]
pub fn __kcall_wait_cond(
    cond_addr: ConditionAddress,
    mutex_addr: MutexAddress,
    timeout: Option<SystemTime>,
) -> Result<(), Error> {
    ::sys::kcall::pm::__kcall_wait_cond(cond_addr, mutex_addr, timeout)
}

#[unsafe(no_mangle)]
pub fn __kcall_gettime(buffer: &mut SystemTime) -> Result<(), Error> {
    ::sys::kcall::pm::__kcall_gettime(buffer)
}

#[unsafe(no_mangle)]
pub fn __kcall_sleep(timeout: Duration) -> Result<(), Error> {
    ::sys::kcall::pm::__kcall_sleep(timeout)
}

#[unsafe(no_mangle)]
pub fn __kcall_get_thread_data_area() -> Result<*mut u8, Error> {
    ::sys::kcall::pm::__kcall_get_thread_data_area()
}

#[unsafe(no_mangle)]
pub fn __kcall_set_thread_data_area(user_tda: *mut u8) -> Result<(), Error> {
    ::sys::kcall::pm::__kcall_set_thread_data_area(user_tda)
}

#[unsafe(no_mangle)]
pub extern "C" fn __kcall_snapshot() -> i32 {
    ::sys::kcall::pm::__kcall_snapshot()
}

//==================================================================================================
// sched
//==================================================================================================

#[unsafe(no_mangle)]
pub fn __kcall_sched_yield() -> Result<(), Error> {
    ::sys::kcall::sched::__kcall_sched_yield()
}

//==================================================================================================
// Thread bootstrap stub
//==================================================================================================

// `_do_start_thread` is the kernel-supplied entry point for newly created
// threads.  The kernel sets up a trap frame so that IRET "returns" here
// with the user thread function pointer in EDX and its argument in ECX.
//
// The asm lives here in `sys-ffi` (not `sys`) so the
// `.global _do_start_thread` symbol exists in a single static archive
// (`libposix.a`), avoiding the duplicate-strong-symbol clash that would
// otherwise occur when both `libnvx_crt0.a` and `libposix.a` link the
// `sys` crate.
//
// `__kcall_create_thread` (in `sys::kcall::pm`) references this symbol
// via `extern "C" fn _do_start_thread()`; the link resolves it from
// `libposix.a` at link time.
//
// The `global_asm!` is unconditional (no `cfg(target_arch = ...)`)
// because Nanvix is x86-only today and `__kcall_create_thread`
// declares `extern "C" fn _do_start_thread()` unconditionally; a
// conditional asm here would surface as a link-time unresolved-symbol
// error rather than a build-time error on any future non-x86 port.
::core::arch::global_asm!(
    r#"
    .global _do_start_thread
    .extern _do_exit_thread
    .type _do_start_thread, @function

    _do_start_thread:
        #
        # Entry point for newly created threads.
        #
        # The kernel sets up a trap frame so that IRET "returns" to this function.
        # The kernel passes the thread function pointer in EDX and its argument
        # in ECX.
        #
        # This stub calls func(arg) and then _do_exit_thread(status) directly,
        # enforcing 16-byte stack alignment before each CALL instruction. This
        # avoids routing through a Rust intermediate function whose compiler-
        # generated prologue may not preserve 16-byte alignment (the Nanvix Rust
        # target disables SSE, so LLVM omits alignment-preserving prologues).
        # The callee func may require 16-byte-aligned stack frames (e.g.,
        # movaps) when SSE instructions are present.
        #

        # Save func and arg into callee-saved registers.
        # This is the thread root frame so there is no caller state to preserve.
        mov esi, edx        # ESI = func
        mov edi, ecx        # EDI = arg

        # Set up frame pointer and force 16-byte alignment.
        and esp, -16
        mov ebp, esp

        #
        # Call func(arg).
        #
        # Stack alignment arithmetic (i386 SysV ABI):
        #   and esp,-16 -> ESP = 0 (mod 16)   (force-aligned)
        #   sub esp, 12 -> ESP = 4 (mod 16)   (alignment padding)
        #   push edi    -> ESP = 0 (mod 16)   (push arg)
        #   call esi    -> ESP = 12 (mod 16)  (return address pushed by CALL)
        #
        sub esp, 12
        push edi
        call esi

        #
        # Call _do_exit_thread(status).
        #
        # func() returned status in EAX.  Re-align the stack for the next call.
        #
        # Stack alignment arithmetic:
        #   and esp,-16 -> ESP = 0 (mod 16)   (force-aligned)
        #   sub esp, 12 -> ESP = 4 (mod 16)   (alignment padding)
        #   push eax    -> ESP = 0 (mod 16)   (push status)
        #   call        -> ESP = 12 (mod 16)  (return address pushed by CALL)
        #
        and esp, -16
        sub esp, 12
        push eax
        call _do_exit_thread

    # Safety net: _do_exit_thread() calls exit_thread() and never returns.
    # If it somehow does, spin forever rather than falling through.
    1: jmp 1b
    "#
);
