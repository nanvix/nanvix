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
