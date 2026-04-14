// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Register convention for x86_64 kernel calls:
// - RAX: kernel call number (in), return value (out).
// - RDI: first argument.
// - RSI: second argument.
// - RDX: third argument.
// - R10: fourth argument.

///
/// # Description
///
/// Issues a kernel call with no arguments.
///
/// # Parameters
/// - `kcall_nr` - Kernel call number.
///
/// # Return
///
/// This function returns the value returned by the kernel call.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
///
#[inline(never)]
pub unsafe fn kcall0(kcall_nr: u32) -> i64 {
    let ret: i64;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int 0x80",
            inout("rax") kcall_nr as u64 => ret,
            options(nostack, preserves_flags)
        );
    }

    ret
}

///
/// # Description
///
/// Issues a kernel call with one argument.
///
/// # Parameters
/// - `kcall_nr` - Kernel call number.
/// - `arg0` - First argument for the kernel call.
///
/// # Return Values
///
/// This function returns the value returned by the kernel call.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
///
#[inline(never)]
pub unsafe fn kcall1(kcall_nr: u32, arg0: u32) -> i64 {
    let ret: i64;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int 0x80",
            inout("rax") kcall_nr as u64 => ret,
            in("rdi") arg0 as u64,
            options(nostack, preserves_flags)
        );
    }

    ret
}

///
/// # Description
///
/// Issues a kernel call with two arguments.
///
/// # Parameters
/// - `kcall_nr` - Kernel call number.
/// - `arg0` - First argument for the kernel call.
/// - `arg1` - Second argument for the kernel call.
///
/// # Return Values
///
/// This function returns the value returned by the kernel call.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
///
#[inline(never)]
pub unsafe fn kcall2(kcall_nr: u32, arg0: u32, arg1: u32) -> i64 {
    let ret: i64;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int 0x80",
            inout("rax") kcall_nr as u64 => ret,
            in("rdi") arg0 as u64,
            in("rsi") arg1 as u64,
            options(nostack, preserves_flags)
        );
    }

    ret
}

///
/// # Description
///
/// Issues a kernel call with three arguments.
///
/// # Parameters
/// - `kcall_nr` - Kernel call number.
/// - `arg0` - First argument for the kernel call.
/// - `arg1` - Second argument for the kernel call.
/// - `arg2` - Third argument for the kernel call.
///
/// # Return Values
///
/// This function returns the value returned by the kernel call.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
///
#[inline(never)]
pub unsafe fn kcall3(kcall_nr: u32, arg0: u32, arg1: u32, arg2: u32) -> i64 {
    let ret: i64;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int 0x80",
            inout("rax") kcall_nr as u64 => ret,
            in("rdi") arg0 as u64,
            in("rsi") arg1 as u64,
            in("rdx") arg2 as u64,
            options(nostack, preserves_flags)
        );
    }

    ret
}

///
/// # Description
///
/// Issues a kernel call with four arguments.
///
/// # Parameters
/// - `kcall_nr` - Kernel call number.
/// - `arg0` - First argument for the kernel call.
/// - `arg1` - Second argument for the kernel call.
/// - `arg2` - Third argument for the kernel call.
/// - `arg3` - Fourth argument for the kernel call.
///
/// # Return Values
///
/// This function returns the value returned by the kernel call.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
///
#[inline(never)]
pub unsafe fn kcall4(kcall_nr: u32, arg0: u32, arg1: u32, arg2: u32, arg3: u32) -> i64 {
    let ret: i64;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int 0x80",
            inout("rax") kcall_nr as u64 => ret,
            in("rdi") arg0 as u64,
            in("rsi") arg1 as u64,
            in("rdx") arg2 as u64,
            in("r10") arg3 as u64,
            options(nostack, preserves_flags)
        );
    }

    ret
}
