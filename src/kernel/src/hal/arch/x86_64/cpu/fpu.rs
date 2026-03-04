// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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
    // TODO: Implement full FPU/SSE initialization for x86_64.
    // In long mode, SSE is required and should already be enabled by the boot code.
    // For now, just save the initial FPU state.
    asm!("fxsave [{}]", in(reg) &mut INITIAL_FPU_STATE.data);
}
