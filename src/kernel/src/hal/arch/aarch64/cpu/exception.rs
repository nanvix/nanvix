// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::ContextInformation;
use ::arch::cpu::excp::Exception;
use ::core::arch::asm;

#[derive(Clone)]
#[repr(C)]
pub struct ExceptionInformation {
    num: u64,
    code: u64,
    addr: u64,
    instruction: u64,
}

impl ExceptionInformation {
    pub const fn new(num: u32, code: u32, addr: u64, instruction: u64) -> Self {
        Self {
            num: num as u64,
            code: code as u64,
            addr,
            instruction,
        }
    }

    pub fn num(&self) -> u32 {
        self.num as u32
    }

    pub fn code(&self) -> u32 {
        self.code as u32
    }

    pub fn addr(&self) -> u64 {
        self.addr
    }

    pub fn instruction(&self) -> u64 {
        self.instruction
    }
}

impl core::fmt::Debug for ExceptionInformation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match Exception::try_from(self.num()) {
            Ok(excp) => write!(
                f,
                "{excp:?} (syndrome={:#x}, faulting addr={:#018x}, faulting instruction={:#018x})",
                self.code, self.addr, self.instruction
            ),
            Err(_) => write!(
                f,
                "unknown exception {} (syndrome={:#x}, faulting addr={:#018x}, faulting \
                 instruction={:#018x})",
                self.num, self.code, self.addr, self.instruction
            ),
        }
    }
}

pub unsafe fn init_vectors() {
    unsafe extern "C" {
        static __aarch64_vector_table: u8;
    }
    let vectors: u64 = (&raw const __aarch64_vector_table) as u64;
    unsafe {
        asm!(
            "msr vbar_el1, {vectors}",
            "isb",
            vectors = in(reg) vectors,
            options(nostack, preserves_flags),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn aarch64_exception_dispatch(
    ctx: *mut ContextInformation,
    esr: u64,
    far: u64,
) {
    let ec: u32 = ((esr >> 26) & 0x3f) as u32;
    let num: u32 = match ec {
        // Instruction and data aborts are the AArch64 equivalent of x86 page faults.
        0x20 | 0x21 | 0x24 | 0x25 => Exception::PageFault as u32,
        0x07 => Exception::CoprocessorNotAvailable as u32,
        0x00 => Exception::InvalidOpcode as u32,
        0x22 => Exception::AlignmentCheck as u32,
        _ => Exception::GeneralProtectionFault as u32,
    };
    let code: u32 = if num == Exception::PageFault as u32 {
        let fault_status: u32 = esr as u32 & 0x3f;
        let translation_fault: bool = (0x4..=0x7).contains(&fault_status);
        let write: bool = (esr & (1 << 6)) != 0;
        let user: bool = unsafe { (*ctx).spsr & 0xf == 0 };
        let instruction: bool = matches!(ec, 0x20 | 0x21);
        u32::from(!translation_fault)
            | (u32::from(write) << 1)
            | (u32::from(user) << 2)
            | (u32::from(instruction) << 4)
    } else {
        esr as u32
    };
    let info: ExceptionInformation =
        ExceptionInformation::new(num, code, far, unsafe { (*ctx).elr });
    unsafe { super::exception_controller::dispatch(&info, &mut *ctx) }
}
