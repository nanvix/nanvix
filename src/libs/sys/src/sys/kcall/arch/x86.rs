// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch;

//==================================================================================================
// Standalone Functions
//==================================================================================================

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
    let low_ret: u32;
    let high_ret: u32;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int 0x80",
            inout("eax") kcall_nr => low_ret,
            lateout("edx") high_ret,
            options(nostack, preserves_flags)
        );
    }

    ((high_ret as i64) << 32) | (low_ret as i64)
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
    let low_ret: i32;
    let high_ret: i32;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int 0x80",
            inout("eax") kcall_nr => low_ret,
            lateout("edx") high_ret,
            in("ebx") arg0,
            options(nostack, preserves_flags)
        );
    }

    ((high_ret as i64) << 32) | (low_ret as i64)
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
    let low_ret: i32;
    let high_ret: i32;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int 0x80",
            inout("eax") kcall_nr => low_ret,
            lateout("edx") high_ret,
            in("ebx") arg0,
            in("ecx") arg1,
            options(nostack, preserves_flags)
        );
    }

    ((high_ret as i64) << 32) | (low_ret as i64)
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
    let low_ret: i32;
    let high_ret: i32;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int 0x80",
            inout("eax") kcall_nr => low_ret,
            lateout("edx") high_ret,
            in("ebx") arg0,
            in("ecx") arg1,
            in("edx") arg2,
            options(nostack, preserves_flags)
        );
    }

    ((high_ret as i64) << 32) | (low_ret as i64)
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
    let low_ret: i32;
    let high_ret: i32;

    // SAFETY: this will trigger a kernel call.
    unsafe {
        arch::asm!("int 0x80",
            inout("eax") kcall_nr => low_ret,
            lateout("edx") high_ret,
            in("ebx") arg0,
            in("ecx") arg1,
            in("edx") arg2,
            in("edi") arg3,
            options(nostack, preserves_flags)
        );
    }

    ((high_ret as i64) << 32) | (low_ret as i64)
}

//==================================================================================================
// Thread Data Area Helpers
//==================================================================================================

///
/// # Description
///
/// Reads a `u32` value from the Thread Data Area (TDA) via the `%gs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
///
/// # Returns
///
/// The `u32` value stored at `gs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%gs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn read_tda_u32(offset: u32) -> u32 {
    let val: u32;
    unsafe {
        arch::asm!(
            "mov {0:e}, gs:[{1:e}]",
            out(reg) val,
            in(reg) offset,
            options(nostack, readonly, preserves_flags),
        );
    }
    val
}

///
/// # Description
///
/// Writes a `u32` value to the Thread Data Area (TDA) via the `%gs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
/// - `val`: The `u32` value to store at `gs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%gs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn write_tda_u32(offset: u32, val: u32) {
    unsafe {
        arch::asm!(
            "mov gs:[{0:e}], {1:e}",
            in(reg) offset,
            in(reg) val,
            options(nostack, preserves_flags),
        );
    }
}
