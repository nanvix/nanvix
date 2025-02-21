// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// Architecture-specific symbols.
#[cfg(target_arch = "x86")]
#[path = "arch/x86.rs"]
pub mod arch;

/// Debug facilities.
pub mod debug;

/// Event handling kernel calls.
pub mod event;

/// Inter-Process Communication (IPC) kernel calls.
pub mod ipc;

/// Memory management kernel calls.
pub mod mm;

/// Process management kernel calls.
pub mod pm;

/// Execution scheduling kernel calls.
pub mod sched;

//==================================================================================================
//  Macros
//==================================================================================================

#[macro_export]
macro_rules! kcall0 {
    ($kcall_number:expr) => {{
        let kcall_number: u32 = $kcall_number;
        unsafe { ::core::hint::black_box($crate::kcall::arch::kcall0(kcall_number)) }
    }};
}

#[macro_export]
macro_rules! kcall1 {
    ($kcall_number:expr, $arg:expr) => {{
        let kcall_number: u32 = $kcall_number;
        let arg: u32 = $arg;
        unsafe { ::core::hint::black_box($crate::kcall::arch::kcall1(kcall_number, arg)) }
    }};
}

#[macro_export]
macro_rules! kcall2 {
    ($kcall_number:expr, $arg0:expr, $arg1:expr) => {{
        let kcall_number: u32 = $kcall_number;
        let arg0: u32 = $arg0;
        let arg1: u32 = $arg1;
        unsafe { ::core::hint::black_box($crate::kcall::arch::kcall2(kcall_number, arg0, arg1)) }
    }};
}

#[macro_export]
macro_rules! kcall3 {
    ($kcall_number:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {{
        let kcall_number: u32 = $kcall_number;
        let arg0: u32 = $arg0;
        let arg1: u32 = $arg1;
        let arg2: u32 = $arg2;
        unsafe {
            ::core::hint::black_box($crate::kcall::arch::kcall3(kcall_number, arg0, arg1, arg2))
        }
    }};
}

#[macro_export]
macro_rules! kcall4 {
    ($kcall_number:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {{
        let kcall_number: u32 = $kcall_number;
        let arg0: u32 = $arg0;
        let arg1: u32 = $arg1;
        let arg2: u32 = $arg2;
        let arg3: u32 = $arg3;
        unsafe {
            ::core::hint::black_box($crate::kcall::arch::kcall4(
                kcall_number,
                arg0,
                arg1,
                arg2,
                arg3,
            ))
        }
    }};
}
