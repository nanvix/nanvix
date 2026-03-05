// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::cpu::mxcrs::{
    DenormalOperationMask,
    DivideByZeroMask,
    MxcsrRegister,
    OverflowMask,
    PrecisionMask,
    UnderflowMask,
};
use ::core::arch::asm;

//==================================================================================================
// Constants
//==================================================================================================

/// Size of the FPU state.
const FPU_STATE_SIZE: usize = 512;

/// Alignment of the FPU state.
const _FPU_STATE_ALIGN: usize = 16;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure represents the state of the FPU.
///
#[repr(C, align(16))]
pub struct FpuState {
    /// FPU state data.
    data: [u8; FPU_STATE_SIZE],
}

::static_assert::assert_eq_size!(FpuState, FPU_STATE_SIZE);
::static_assert::assert_eq_align!(FpuState, _FPU_STATE_ALIGN);

//==================================================================================================
// Global Variables
//==================================================================================================

/// Initial FPU state, overwritten during system initialization.
static mut INITIAL_FPU_STATE: FpuState = FpuState {
    data: [0; FPU_STATE_SIZE],
};

//==================================================================================================
// Implementations
//==================================================================================================

impl FpuState {
    ///
    /// # Description
    ///
    /// Constructs a new FPU state.
    ///
    /// # Safety
    ///
    /// It is unsafe to call this function because it accesses global state that is not
    /// synchronized.
    ///
    pub unsafe fn new() -> Self {
        Self {
            data: INITIAL_FPU_STATE.data,
        }
    }

    ///
    /// # Description
    ///
    /// Saves the current FPU state to the given memory location.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it dereferences a raw pointer.
    ///
    pub unsafe fn save(to: *mut FpuState) {
        asm!("fxsave [{}]", in(reg) to, options(nostack, preserves_flags));
    }

    ///
    /// # Description
    ///
    /// Restores the FPU state from the given memory location.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it changes the state of the FPU.
    ///
    pub unsafe fn restore(from: *const FpuState) {
        asm!("fxrstor [{}]", in(reg) from, options(nostack, preserves_flags));
    }
}

///
/// # Description
///
/// Enables SIMD support in the underlying processor.
///
/// # Safety
///
/// It is unsafe to call this function because it executes privileged instructions.
///
pub unsafe fn init() {
    // Disable x87 emulation (clear CR0.EM, bit 2) and enable coprocessor monitoring (set CR0.MP,
    // bit 1).
    let mut cr0: u64;
    asm!("mov {}, cr0", out(reg) cr0, options(nostack, preserves_flags));
    cr0 &= !(1 << 2); // Clear EM (emulation).
    cr0 |= 1 << 1; // Set MP (monitor coprocessor).
    asm!("mov cr0, {}", in(reg) cr0, options(nostack, preserves_flags));

    // Enable support for FXSAVE/FXRSTOR (CR4.OSFXSR, bit 9) and SIMD exceptions
    // (CR4.OSXMMEXCPT, bit 10).
    let mut cr4: u64;
    asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));
    cr4 |= 1 << 9; // Set OSFXSR.
    cr4 |= 1 << 10; // Set OSXMMEXCPT.
    asm!("mov cr4, {}", in(reg) cr4, options(nostack, preserves_flags));

    // Initialize the x87 FPU.
    asm!("fninit", options(nostack));

    // Mask all exceptions in the MXCSR register.
    let mut mxcsr: MxcsrRegister = MxcsrRegister::read();
    mxcsr.precision_mask = PrecisionMask::Masked;
    mxcsr.underflow_mask = UnderflowMask::Masked;
    mxcsr.overflow_mask = OverflowMask::Masked;
    mxcsr.divide_by_zero_mask = DivideByZeroMask::Masked;
    mxcsr.denormal_operation_mask = DenormalOperationMask::Masked;
    mxcsr.write();

    // Save the initial FPU state.
    // SAFETY: access to INITIAL_FPU_STATE is synchronized.
    asm!("fxsave [{}]", in(reg) &mut INITIAL_FPU_STATE.data);
}
