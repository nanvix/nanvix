// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use windows::Win32::System::Hypervisor::{
    WHV_PARTITION_PROPERTY_CODE,
    WHV_REGISTER_NAME,
    WHV_VIRTUAL_PROCESSOR_STATE_TYPE,
};

pub const PARTITION_PROPERTY_ARM64_IC_PARAMETERS: WHV_PARTITION_PROPERTY_CODE =
    WHV_PARTITION_PROPERTY_CODE(0x0000_1012);

pub const REGISTER_X0: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0002_0000);
pub const REGISTER_X1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0002_0001);
pub const REGISTER_X28: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0002_001c);
pub const REGISTER_FP: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0002_001d);
pub const REGISTER_LR: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0002_001e);
pub const REGISTER_SP: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0002_001f);
pub const REGISTER_SP_EL0: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0002_0020);
pub const REGISTER_SP_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0002_0021);
pub const REGISTER_PC: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0002_0022);
pub const REGISTER_PSTATE: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0002_0023);

pub const REGISTER_Q0: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0003_0000);
pub const REGISTER_Q31: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0003_001f);

pub const REGISTER_MP_IDR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0001);
pub const REGISTER_SCTLR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0002);
pub const REGISTER_CPACR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0004);
pub const REGISTER_TTBR0_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0005);
pub const REGISTER_TTBR1_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0006);
pub const REGISTER_TCR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0007);
pub const REGISTER_ESR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0008);
pub const REGISTER_FAR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0009);
pub const REGISTER_MAIR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_000b);
pub const REGISTER_VBAR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_000c);
pub const REGISTER_CONTEXT_IDR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_000d);
pub const REGISTER_TPIDR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_000e);
pub const REGISTER_TPIDRRO_EL0: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0010);
pub const REGISTER_TPIDR_EL0: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0011);
pub const REGISTER_FPCR: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0012);
pub const REGISTER_FPSR: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0013);
pub const REGISTER_SPSR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0014);
pub const REGISTER_ELR_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0004_0015);

pub const REGISTER_CNTKCTL_EL1: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0005_8008);
pub const REGISTER_CNTV_CTL_EL0: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0005_800e);
pub const REGISTER_CNTV_CVAL_EL0: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0005_800f);
pub const REGISTER_CNTVCT_EL0: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0005_8011);
pub const REGISTER_GICR_BASE_GPA: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x0006_3000);

pub const VP_STATE_INTERRUPT_CONTROLLER: WHV_VIRTUAL_PROCESSOR_STATE_TYPE =
    WHV_VIRTUAL_PROCESSOR_STATE_TYPE(i32::MIN);
pub const VP_STATE_GLOBAL_INTERRUPT: WHV_VIRTUAL_PROCESSOR_STATE_TYPE =
    WHV_VIRTUAL_PROCESSOR_STATE_TYPE(0xc000_0006u32.cast_signed());

pub const EXIT_REASON_UNMAPPED_GPA: u32 = 0x8000_0000;
pub const EXIT_REASON_GPA_INTERCEPT: u32 = 0x8000_0001;
pub const EXIT_REASON_INVALID_VP_REGISTER: u32 = 0x8000_0020;
pub const EXIT_REASON_UNRECOVERABLE_EXCEPTION: u32 = 0x8000_0021;
pub const EXIT_REASON_UNSUPPORTED_FEATURE: u32 = 0x8000_0022;
pub const EXIT_REASON_ARM64_RESET: u32 = 0x8001_000c;
pub const EXIT_REASON_CANCELED: u32 = u32::MAX;

pub const RESET_TYPE_POWER_OFF: i32 = 0;
pub const RESET_TYPE_REBOOT: i32 = 1;

pub const PSTATE_EL1H_MASKED: u64 = 0x3c5;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Arm64IcGicV3Parameters {
    pub gicd_base_address: u64,
    pub gits_translater_base_address: u64,
    pub reserved: u32,
    pub gic_lpi_int_id_bits: u32,
    pub gic_ppi_overflow_interrupt_from_cntv: u32,
    pub gic_ppi_performance_monitors_interrupt: u32,
    pub reserved1: [u32; 6],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Arm64IcParameters {
    pub emulation_mode: i32,
    pub reserved: u32,
    pub gic_v3_parameters: Arm64IcGicV3Parameters,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Arm64InterruptControl {
    pub target_partition: u64,
    pub interrupt_control: u64,
    pub destination_address: u64,
    pub requested_vector: u32,
    pub target_vtl: u8,
    pub reserved0: u8,
    pub reserved1: u16,
}

impl Arm64InterruptControl {
    /// Creates a request that asserts an interrupt line.
    pub fn asserted(vector: u32) -> Self {
        Self {
            interrupt_control: 1 << 34,
            requested_vector: vector,
            ..Self::default()
        }
    }

    /// Creates a request that deasserts an interrupt line.
    pub fn deasserted(vector: u32) -> Self {
        Self {
            requested_vector: vector,
            ..Self::default()
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Arm64VpExecutionState {
    pub bits: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Arm64InterceptMessageHeader {
    pub vp_index: u32,
    pub instruction_length: u8,
    pub intercept_access_type: u8,
    pub execution_state: Arm64VpExecutionState,
    pub pc: u64,
    pub cpsr: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Arm64MemoryAccessContext {
    pub header: Arm64InterceptMessageHeader,
    pub reserved0: u32,
    pub instruction_byte_count: u8,
    pub access_info: u8,
    pub reserved1: u16,
    pub instruction_bytes: [u8; 4],
    pub reserved2: u32,
    pub gva: u64,
    pub gpa: u64,
    pub syndrome: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Arm64ResetContext {
    pub header: Arm64InterceptMessageHeader,
    pub reset_type: i32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union Arm64ExitContextData {
    pub memory_access: Arm64MemoryAccessContext,
    pub reset: Arm64ResetContext,
    pub raw: [u64; 32],
}

impl Default for Arm64ExitContextData {
    fn default() -> Self {
        Self { raw: [0; 32] }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Arm64RunVpExitContext {
    pub exit_reason: u32,
    pub reserved: u32,
    pub reserved1: u64,
    pub data: Arm64ExitContextData,
}

const _: () = assert!(core::mem::size_of::<Arm64IcGicV3Parameters>() == 56);
const _: () = assert!(core::mem::size_of::<Arm64IcParameters>() == 64);
const _: () = assert!(core::mem::size_of::<Arm64InterruptControl>() == 32);
const _: () = assert!(core::mem::size_of::<Arm64InterceptMessageHeader>() == 24);
const _: () = assert!(core::mem::size_of::<Arm64MemoryAccessContext>() == 64);
const _: () = assert!(core::mem::size_of::<Arm64RunVpExitContext>() == 272);

#[cfg(test)]
mod tests {
    use super::Arm64InterruptControl;

    #[test]
    fn interrupt_control_encodes_line_state() {
        let asserted: Arm64InterruptControl = Arm64InterruptControl::asserted(32);
        let deasserted: Arm64InterruptControl = Arm64InterruptControl::deasserted(32);

        assert_eq!(asserted.interrupt_control, 1 << 34);
        assert_eq!(deasserted.interrupt_control, 0);
        assert_eq!(asserted.requested_vector, 32);
        assert_eq!(deasserted.requested_vector, 32);
    }
}
