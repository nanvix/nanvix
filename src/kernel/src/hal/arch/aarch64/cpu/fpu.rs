// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::core::arch::{
    asm,
    global_asm,
};

pub const FPU_STATE_SIZE: usize = 528;
const CPACR_FPEN_SHIFT: u64 = 20;
const CPACR_FPEN_MASK: u64 = 0b11 << CPACR_FPEN_SHIFT;
const CPACR_FPEN_TRAP_EL0: u64 = 0b01 << CPACR_FPEN_SHIFT;
const CPACR_FPEN_NO_TRAP: u64 = 0b11 << CPACR_FPEN_SHIFT;

#[repr(C, align(16))]
pub struct FpuState {
    data: [u8; FPU_STATE_SIZE],
}

::static_assert::assert_eq_size!(FpuState, FPU_STATE_SIZE);
::static_assert::assert_eq_align!(FpuState, 16);

global_asm!(
    r#"
    .section .text,"ax",@progbits
    .arch armv8-a+simd
    .global __aarch64_fpu_save
    .global __aarch64_fpu_restore

__aarch64_fpu_save:
    stp q0, q1, [x0, #0]
    stp q2, q3, [x0, #32]
    stp q4, q5, [x0, #64]
    stp q6, q7, [x0, #96]
    stp q8, q9, [x0, #128]
    stp q10, q11, [x0, #160]
    stp q12, q13, [x0, #192]
    stp q14, q15, [x0, #224]
    stp q16, q17, [x0, #256]
    stp q18, q19, [x0, #288]
    stp q20, q21, [x0, #320]
    stp q22, q23, [x0, #352]
    stp q24, q25, [x0, #384]
    stp q26, q27, [x0, #416]
    stp q28, q29, [x0, #448]
    stp q30, q31, [x0, #480]
    mrs x1, fpcr
    str x1, [x0, #512]
    mrs x1, fpsr
    str x1, [x0, #520]
    ret

__aarch64_fpu_restore:
    ldr x1, [x0, #512]
    msr fpcr, x1
    ldr x1, [x0, #520]
    msr fpsr, x1
    ldp q0, q1, [x0, #0]
    ldp q2, q3, [x0, #32]
    ldp q4, q5, [x0, #64]
    ldp q6, q7, [x0, #96]
    ldp q8, q9, [x0, #128]
    ldp q10, q11, [x0, #160]
    ldp q12, q13, [x0, #192]
    ldp q14, q15, [x0, #224]
    ldp q16, q17, [x0, #256]
    ldp q18, q19, [x0, #288]
    ldp q20, q21, [x0, #320]
    ldp q22, q23, [x0, #352]
    ldp q24, q25, [x0, #384]
    ldp q26, q27, [x0, #416]
    ldp q28, q29, [x0, #448]
    ldp q30, q31, [x0, #480]
    ret
"#
);

impl FpuState {
    pub unsafe fn new() -> Self {
        Self {
            data: [0; FPU_STATE_SIZE],
        }
    }

    pub unsafe fn save(to: *mut FpuState) {
        unsafe extern "C" {
            fn __aarch64_fpu_save(to: *mut FpuState);
        }
        unsafe { __aarch64_fpu_save(to) };
    }

    pub unsafe fn restore(from: *const FpuState) {
        unsafe extern "C" {
            fn __aarch64_fpu_restore(from: *const FpuState);
        }
        unsafe { __aarch64_fpu_restore(from) };
    }
}

pub unsafe fn capture_fpu(fpu: *mut FpuState, is_owner: bool) -> [u8; FPU_STATE_SIZE] {
    let mut image: [u8; FPU_STATE_SIZE] = [0; FPU_STATE_SIZE];
    unsafe {
        if is_owner {
            FpuState::save(fpu);
        }
        core::ptr::copy_nonoverlapping(fpu.cast::<u8>(), image.as_mut_ptr(), FPU_STATE_SIZE);
    }
    image
}

pub unsafe fn install_fpu(fpu: *mut FpuState, is_owner: bool, image: &[u8; FPU_STATE_SIZE]) {
    unsafe {
        core::ptr::copy_nonoverlapping(image.as_ptr(), fpu.cast::<u8>(), FPU_STATE_SIZE);
        if is_owner {
            FpuState::restore(fpu);
        }
    }
}

pub(super) unsafe fn enable_user_access() {
    let mut cpacr: u64;
    unsafe {
        asm!("mrs {value}, cpacr_el1", value = out(reg) cpacr, options(nostack));
        cpacr = (cpacr & !CPACR_FPEN_MASK) | CPACR_FPEN_NO_TRAP;
        asm!(
            "msr cpacr_el1, {value}",
            "isb",
            value = in(reg) cpacr,
            options(nostack, preserves_flags),
        );
    }
}

pub(super) unsafe fn disable_user_access() {
    let mut cpacr: u64;
    unsafe {
        asm!("mrs {value}, cpacr_el1", value = out(reg) cpacr, options(nostack));
        cpacr = (cpacr & !CPACR_FPEN_MASK) | CPACR_FPEN_TRAP_EL0;
        asm!(
            "msr cpacr_el1, {value}",
            "isb",
            value = in(reg) cpacr,
            options(nostack, preserves_flags),
        );
    }
}

pub unsafe fn init() {
    unsafe {
        enable_user_access();
    }
}
