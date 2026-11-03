// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// C library types intentionally follow C naming conventions.
#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

use crate::jmp_buf::{
    jmp_buf_reg,
    JMP_BUF_REGS,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// The `sigjmp_buf` type stores the execution context for `sigsetjmp`/`siglongjmp`.
///
/// On x86-32, the following registers are saved: EBX, ESI, EDI, EBP, ESP, and the return address
/// (EIP). On x86-64, the saved registers are: RBX, RBP, R12, R13, R14, R15, RSP, and the return
/// address (RIP). The `savemask` field records whether `sigsetjmp` was asked to save the signal
/// mask.
///
#[repr(C)]
pub struct sigjmp_buf {
    /// Saved registers. x86-32: EBX, ESI, EDI, EBP, ESP, EIP. x86-64: RBX, RBP, R12, R13, R14,
    /// R15, RSP, RIP.
    pub regs: [jmp_buf_reg; JMP_BUF_REGS],
    /// Nonzero if `sigsetjmp` was called with a nonzero `savemask` argument.
    pub savemask: c_int,
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
