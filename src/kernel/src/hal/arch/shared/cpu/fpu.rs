// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::cpu::{
    cr0::{
        Cr0Register,
        EmulationFlag,
        MonitorCoprocessorFlag,
    },
    cr4::{
        Cr4Register,
        OsFxsaveFlag,
        OsSimdExceptionFlag,
    },
    mxcrs::{
        DenormalOperationMask,
        DivideByZeroMask,
        MxcsrRegister,
        OverflowMask,
        PrecisionMask,
        UnderflowMask,
    },
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
    /// It is safe to call this function if and only if the following conditions are met:
    /// - Calls to this function are synchronized.
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
    /// It is safe to call this function if the following conditions are met:
    /// - `to` is a valid pointer to a memory region that is at least `FPU_STATE_SIZE` bytes long.
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
    /// This function is unsafe because:
    /// - It dereferences a raw pointer.
    /// - It changes the state of the FPU.
    ///
    /// It is safe to call this function if the following conditions are met:
    /// - `from` is a valid pointer to a memory region that is at least `FPU_STATE_SIZE` bytes long.
    /// - The caller executes in a context where changing the FPU state is safe.
    ///
    pub unsafe fn restore(from: *const FpuState) {
        asm!("fxrstor [{}]", in(reg) from, options(nostack, preserves_flags));
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Captures a thread's FPU image into a raw buffer.
///
/// When `is_owner` is set the thread currently owns the live FPU, so its registers are flushed to
/// the kernel-side save area first; otherwise the already-saved area is copied as is.
///
/// # Safety
///
/// `fpu` must point to the FPU save area of the target thread. When `is_owner` is set, the live FPU
/// must belong to that thread so flushing it does not clobber another thread's state.
pub unsafe fn capture_fpu(fpu: *mut FpuState, is_owner: bool) -> [u8; FPU_STATE_SIZE] {
    let mut image: [u8; FPU_STATE_SIZE] = [0u8; FPU_STATE_SIZE];
    unsafe {
        if is_owner {
            FpuState::save(fpu);
        }
        ::core::ptr::copy_nonoverlapping(fpu.cast::<u8>(), image.as_mut_ptr(), FPU_STATE_SIZE);
    }
    image
}

/// Installs a raw FPU image into a thread.
///
/// When `is_owner` is set the thread currently owns the live FPU, so the restored area is reloaded
/// into the registers; otherwise only the kernel-side save area is updated.
///
/// # Safety
///
/// As for [`capture_fpu`]. When `is_owner` is set the live FPU is reloaded from `fpu`, which must
/// hold a valid `FXSAVE` image.
pub unsafe fn install_fpu(fpu: *mut FpuState, is_owner: bool, image: &[u8; FPU_STATE_SIZE]) {
    unsafe {
        ::core::ptr::copy_nonoverlapping(image.as_ptr(), fpu.cast::<u8>(), FPU_STATE_SIZE);
        if is_owner {
            FpuState::restore(fpu);
        }
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
///
/// It is safe to call this function if the following conditions are met:
/// - Calls to this function are synchronized.
/// - The caller runs on a processor that supports either SSE or SSE2 features.
/// - The caller runs on a processor that supports FXSAVE and FXRSTOR instructions.
/// - The caller runs at processor privilege level 0.
///
pub unsafe fn init() {
    // Disable x87 emulation and enable coprocessor monitoring.
    let mut cr0: Cr0Register = Cr0Register::read();
    cr0.emulation = EmulationFlag::Disabled;
    cr0.monitor_coprocessor = MonitorCoprocessorFlag::Enabled;
    cr0.write();

    // Enable support for fxsave and fxrstor instructions, as well as support for SIMD exceptions.
    let mut cr4: Cr4Register = Cr4Register::read();
    cr4.os_fxsave = OsFxsaveFlag::Enabled;
    cr4.os_simd_exception = OsSimdExceptionFlag::Enabled;
    cr4.write();

    // Mask all exceptions in the MXCSR register.
    let mut mxcrs: MxcsrRegister = MxcsrRegister::read();
    mxcrs.precision_mask = PrecisionMask::Masked;
    mxcrs.underflow_mask = UnderflowMask::Masked;
    mxcrs.overflow_mask = OverflowMask::Masked;
    mxcrs.divide_by_zero_mask = DivideByZeroMask::Masked;
    mxcrs.denormal_operation_mask = DenormalOperationMask::Masked;
    mxcrs.write();

    // Save the initial FPU state.
    // SAFETY: access to INITIAL_FPU_STATE is synchronized.
    asm!("fxsave [{}]", in(reg) &mut INITIAL_FPU_STATE.data);
}
