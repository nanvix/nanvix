// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(clippy::cast_possible_truncation)]

use super::exit::{
    MmioAccess,
    ResetKind,
    VirtualProcessorExitContext,
};
use crate::vmm::microvm::whp::{
    arm64::{
        Arm64RunVpExitContext,
        EXIT_REASON_ARM64_RESET,
        EXIT_REASON_CANCELED,
        EXIT_REASON_GPA_INTERCEPT,
        EXIT_REASON_INVALID_VP_REGISTER,
        EXIT_REASON_UNMAPPED_GPA,
        EXIT_REASON_UNRECOVERABLE_EXCEPTION,
        EXIT_REASON_UNSUPPORTED_FEATURE,
        PSTATE_EL1H_MASKED,
        REGISTER_CNTKCTL_EL1,
        REGISTER_CNTV_CTL_EL0,
        REGISTER_CNTV_CVAL_EL0,
        REGISTER_CNTVCT_EL0,
        REGISTER_CONTEXT_IDR_EL1,
        REGISTER_CPACR_EL1,
        REGISTER_ELR_EL1,
        REGISTER_ESR_EL1,
        REGISTER_FAR_EL1,
        REGISTER_FP,
        REGISTER_FPCR,
        REGISTER_FPSR,
        REGISTER_GICR_BASE_GPA,
        REGISTER_LR,
        REGISTER_MAIR_EL1,
        REGISTER_MP_IDR_EL1,
        REGISTER_PC,
        REGISTER_PSTATE,
        REGISTER_Q0,
        REGISTER_Q31,
        REGISTER_SCTLR_EL1,
        REGISTER_SP,
        REGISTER_SP_EL0,
        REGISTER_SP_EL1,
        REGISTER_SPSR_EL1,
        REGISTER_TCR_EL1,
        REGISTER_TPIDR_EL0,
        REGISTER_TPIDR_EL1,
        REGISTER_TPIDRRO_EL0,
        REGISTER_TTBR0_EL1,
        REGISTER_TTBR1_EL1,
        REGISTER_VBAR_EL1,
        REGISTER_X0,
        REGISTER_X1,
        REGISTER_X28,
        RESET_TYPE_POWER_OFF,
        RESET_TYPE_REBOOT,
        VP_STATE_GLOBAL_INTERRUPT,
        VP_STATE_INTERRUPT_CONTROLLER,
    },
    partition::WhpPartition,
};
use ::anyhow::Result;
use ::log::{
    error,
    trace,
    warn,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::mem;
use windows::Win32::System::Hypervisor::{
    WHV_PARTITION_HANDLE,
    WHV_REGISTER_NAME,
    WHV_REGISTER_VALUE,
    WHV_VIRTUAL_PROCESSOR_STATE_TYPE,
    WHvCancelRunVirtualProcessor,
    WHvCreateVirtualProcessor,
    WHvDeleteVirtualProcessor,
    WHvGetVirtualProcessorRegisters,
    WHvGetVirtualProcessorState,
    WHvRunVirtualProcessor,
    WHvSetVirtualProcessorRegisters,
    WHvSetVirtualProcessorState,
};

#[derive(Serialize, Deserialize)]
pub struct VcpuState {
    register_values: Vec<[u8; 16]>,
    local_interrupt_state: Vec<u8>,
    global_interrupt_state: Vec<u8>,
}

pub struct VirtualProcessor {
    partition: WHV_PARTITION_HANDLE,
    index: u32,
    online: bool,
    exit_status: u16,
}

unsafe impl Send for VirtualProcessor {}
unsafe impl Sync for VirtualProcessor {}

fn reg_count(names: &[WHV_REGISTER_NAME]) -> u32 {
    u32::try_from(names.len()).unwrap_or(u32::MAX)
}

unsafe fn whp_get_registers(
    partition: WHV_PARTITION_HANDLE,
    vp_index: u32,
    names: &[WHV_REGISTER_NAME],
    values: &mut [WHV_REGISTER_VALUE],
) -> windows::core::Result<()> {
    unsafe {
        WHvGetVirtualProcessorRegisters(
            partition,
            vp_index,
            names.as_ptr(),
            reg_count(names),
            values.as_mut_ptr(),
        )
    }
}

unsafe fn whp_set_registers(
    partition: WHV_PARTITION_HANDLE,
    vp_index: u32,
    names: &[WHV_REGISTER_NAME],
    values: &[WHV_REGISTER_VALUE],
) -> windows::core::Result<()> {
    unsafe {
        WHvSetVirtualProcessorRegisters(
            partition,
            vp_index,
            names.as_ptr(),
            reg_count(names),
            values.as_ptr(),
        )
    }
}

fn snapshot_register_names() -> Vec<WHV_REGISTER_NAME> {
    let mut names: Vec<WHV_REGISTER_NAME> = Vec::with_capacity(87);

    for register in REGISTER_X0.0..=REGISTER_X28.0 {
        names.push(WHV_REGISTER_NAME(register));
    }
    names.extend_from_slice(&[
        REGISTER_FP,
        REGISTER_LR,
        REGISTER_SP,
        REGISTER_SP_EL0,
        REGISTER_SP_EL1,
        REGISTER_PC,
        REGISTER_PSTATE,
    ]);
    for register in REGISTER_Q0.0..=REGISTER_Q31.0 {
        names.push(WHV_REGISTER_NAME(register));
    }
    names.extend_from_slice(&[
        REGISTER_FPCR,
        REGISTER_FPSR,
        REGISTER_SPSR_EL1,
        REGISTER_ELR_EL1,
        REGISTER_MP_IDR_EL1,
        REGISTER_SCTLR_EL1,
        REGISTER_CPACR_EL1,
        REGISTER_TTBR0_EL1,
        REGISTER_TTBR1_EL1,
        REGISTER_TCR_EL1,
        REGISTER_ESR_EL1,
        REGISTER_FAR_EL1,
        REGISTER_MAIR_EL1,
        REGISTER_VBAR_EL1,
        REGISTER_CONTEXT_IDR_EL1,
        REGISTER_TPIDR_EL1,
        REGISTER_TPIDRRO_EL0,
        REGISTER_TPIDR_EL0,
        REGISTER_CNTKCTL_EL1,
        REGISTER_CNTV_CTL_EL0,
        REGISTER_CNTV_CVAL_EL0,
        REGISTER_CNTVCT_EL0,
        REGISTER_GICR_BASE_GPA,
    ]);

    names
}

fn get_state_blob(
    partition: WHV_PARTITION_HANDLE,
    vp_index: u32,
    state_type: WHV_VIRTUAL_PROCESSOR_STATE_TYPE,
) -> Result<Vec<u8>> {
    let mut state: Vec<u8> = vec![0; 4096];

    loop {
        let mut written: u32 = 0;
        let result = unsafe {
            WHvGetVirtualProcessorState(
                partition,
                vp_index,
                state_type,
                state.as_mut_ptr().cast(),
                state.len() as u32,
                Some(&mut written),
            )
        };

        match result {
            Ok(()) => {
                state.truncate(written as usize);
                return Ok(state);
            },
            Err(_error) if written as usize > state.len() => {
                state.resize(written as usize, 0);
            },
            Err(error) => {
                anyhow::bail!(
                    "failed to get ARM64 vCPU state (type={:#010x}, error={error:?})",
                    state_type.0
                );
            },
        }
    }
}

fn set_state_blob(
    partition: WHV_PARTITION_HANDLE,
    vp_index: u32,
    state_type: WHV_VIRTUAL_PROCESSOR_STATE_TYPE,
    state: &[u8],
) -> Result<()> {
    unsafe {
        WHvSetVirtualProcessorState(
            partition,
            vp_index,
            state_type,
            state.as_ptr().cast(),
            state.len() as u32,
        )
        .map_err(|error| {
            anyhow::anyhow!(
                "failed to set ARM64 vCPU state (type={:#010x}, error={error:?})",
                state_type.0
            )
        })
    }
}

impl VirtualProcessor {
    pub fn new(partition: &WhpPartition, index: u32) -> Result<Self> {
        trace!("VirtualProcessor::new(): index={index}");

        unsafe {
            WHvCreateVirtualProcessor(partition.handle(), index, 0).map_err(|error| {
                anyhow::anyhow!("failed to create ARM64 virtual processor (error={error:?})")
            })?;
        }

        Ok(Self {
            partition: partition.handle(),
            index,
            online: false,
            exit_status: 0,
        })
    }

    pub fn reset(&mut self, pc: u64, x0: u64, x1: u64) -> Result<()> {
        trace!("reset(): pc={pc:#018x}, x0={x0:#018x}, x1={x1:#018x}");

        let names: [WHV_REGISTER_NAME; 6] = [
            REGISTER_GICR_BASE_GPA,
            REGISTER_PC,
            REGISTER_PSTATE,
            REGISTER_X0,
            REGISTER_X1,
            REGISTER_SCTLR_EL1,
        ];
        let mut values: [WHV_REGISTER_VALUE; 6] = [unsafe { mem::zeroed() }; 6];
        values[0].Reg64 = ::config::microvm::DEFAULT_GICR_BASE as u64;
        values[1].Reg64 = pc;
        values[2].Reg64 = PSTATE_EL1H_MASKED;
        values[3].Reg64 = x0;
        values[4].Reg64 = x1;
        values[5].Reg64 = 0;

        unsafe {
            whp_set_registers(self.partition, self.index, &names, &values).map_err(|error| {
                anyhow::anyhow!("failed to set ARM64 boot registers (error={error:?})")
            })?;
        }

        self.online = true;
        Ok(())
    }

    pub fn poweroff(&mut self, exit_status: u16) {
        self.online = false;
        self.exit_status = exit_status;
    }

    pub fn exit_status(&self) -> u16 {
        self.exit_status
    }

    pub fn is_online(&self) -> bool {
        self.online
    }

    pub fn cancel(&self) {
        unsafe {
            if let Err(error) = WHvCancelRunVirtualProcessor(self.partition, self.index, 0) {
                warn!("cancel(): failed to cancel ARM64 vCPU (error={error:?})");
            }
        }
    }

    pub fn get_profile_regs(&self) -> Result<(u32, u32, u32)> {
        Err(anyhow::anyhow!("guest stack profiling is not supported for AArch64 guests"))
    }

    pub fn set_rip_and_rax(&mut self, _value: u64) {
        warn!("set_rip_and_rax(): ignored on ARM64");
    }

    pub fn flush_pending_rip(&mut self) {}

    pub fn read_mmio_write_value(&self, access: MmioAccess) -> Result<u64> {
        let syndrome: u64 = access.syndrome();
        let ec: u64 = (syndrome >> 26) & 0x3f;
        let is_valid: bool = ((syndrome >> 24) & 1) != 0;
        let is_write: bool = ((syndrome >> 6) & 1) != 0;
        let access_size: u64 = 1 << ((syndrome >> 22) & 0x3);
        let source_register: u64 = (syndrome >> 16) & 0x1f;

        if !matches!(ec, 0x24 | 0x25) || !is_valid || !is_write {
            anyhow::bail!(
                "invalid ARM64 MMIO write syndrome (pc={:#018x}, syndrome={syndrome:#018x})",
                access.pc()
            );
        }
        if access_size != mem::size_of::<usize>() as u64 {
            anyhow::bail!("invalid ARM64 doorbell write size ({access_size} bytes)");
        }
        if source_register == 31 {
            anyhow::bail!("ARM64 doorbell pointer was written from XZR");
        }

        let names: [WHV_REGISTER_NAME; 1] =
            [WHV_REGISTER_NAME(REGISTER_X0.0 + source_register as i32)];
        let mut values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        unsafe {
            whp_get_registers(self.partition, self.index, &names, &mut values).map_err(
                |error| anyhow::anyhow!("failed to read ARM64 MMIO source register ({error:?})"),
            )?;
            Ok(values[0].Reg64)
        }
    }

    pub fn advance_mmio(&mut self, access: MmioAccess) -> Result<()> {
        let names: [WHV_REGISTER_NAME; 1] = [REGISTER_PC];
        let mut values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        values[0].Reg64 = access
            .pc()
            .checked_add(4)
            .ok_or_else(|| anyhow::anyhow!("ARM64 program counter overflow"))?;
        unsafe {
            whp_set_registers(self.partition, self.index, &names, &values)
                .map_err(|error| anyhow::anyhow!("failed to advance ARM64 PC ({error:?})"))
        }
    }

    pub fn save_state(&self) -> Result<VcpuState> {
        let names: Vec<WHV_REGISTER_NAME> = snapshot_register_names();
        let mut values: Vec<WHV_REGISTER_VALUE> = vec![unsafe { mem::zeroed() }; names.len()];

        unsafe {
            whp_get_registers(self.partition, self.index, &names, &mut values)
                .map_err(|error| anyhow::anyhow!("failed to save ARM64 registers ({error:?})"))?;
        }

        let register_values: Vec<[u8; 16]> = values
            .iter()
            .map(|value| {
                let mut bytes: [u8; 16] = [0; 16];
                unsafe {
                    ::std::ptr::copy_nonoverlapping(
                        (value as *const WHV_REGISTER_VALUE).cast::<u8>(),
                        bytes.as_mut_ptr(),
                        bytes.len(),
                    );
                }
                bytes
            })
            .collect();

        Ok(VcpuState {
            register_values,
            local_interrupt_state: get_state_blob(
                self.partition,
                self.index,
                VP_STATE_INTERRUPT_CONTROLLER,
            )?,
            global_interrupt_state: get_state_blob(
                self.partition,
                u32::MAX,
                VP_STATE_GLOBAL_INTERRUPT,
            )?,
        })
    }

    pub fn load_state(&mut self, state: &VcpuState) -> Result<()> {
        let names: Vec<WHV_REGISTER_NAME> = snapshot_register_names();
        if state.register_values.len() != names.len() {
            anyhow::bail!(
                "ARM64 snapshot register count mismatch (expected={}, got={})",
                names.len(),
                state.register_values.len()
            );
        }

        let values: Vec<WHV_REGISTER_VALUE> = state
            .register_values
            .iter()
            .map(|bytes| {
                let mut value: WHV_REGISTER_VALUE = unsafe { mem::zeroed() };
                unsafe {
                    ::std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        (&mut value as *mut WHV_REGISTER_VALUE).cast::<u8>(),
                        bytes.len(),
                    );
                }
                value
            })
            .collect();

        unsafe {
            whp_set_registers(self.partition, self.index, &names, &values).map_err(|error| {
                anyhow::anyhow!("failed to restore ARM64 registers ({error:?})")
            })?;
        }
        set_state_blob(
            self.partition,
            u32::MAX,
            VP_STATE_GLOBAL_INTERRUPT,
            &state.global_interrupt_state,
        )?;
        set_state_blob(
            self.partition,
            self.index,
            VP_STATE_INTERRUPT_CONTROLLER,
            &state.local_interrupt_state,
        )?;

        self.online = true;
        Ok(())
    }

    pub fn run(&mut self) -> VirtualProcessorExitContext {
        let mut exit_context: Arm64RunVpExitContext = Arm64RunVpExitContext::default();
        let result = unsafe {
            WHvRunVirtualProcessor(
                self.partition,
                self.index,
                (&mut exit_context as *mut Arm64RunVpExitContext).cast(),
                mem::size_of::<Arm64RunVpExitContext>() as u32,
            )
        };

        match result {
            Ok(()) => self.parse_exit_context(&exit_context),
            Err(error) => {
                error!("run(): error running ARM64 vCPU (error={error:?})");
                VirtualProcessorExitContext::Unknown
            },
        }
    }

    fn parse_exit_context(
        &mut self,
        exit_context: &Arm64RunVpExitContext,
    ) -> VirtualProcessorExitContext {
        match exit_context.exit_reason {
            EXIT_REASON_UNMAPPED_GPA | EXIT_REASON_GPA_INTERCEPT => {
                let memory_access = unsafe { exit_context.data.memory_access };
                trace!(
                    "run(): ARM64 MMIO access at GPA {:#018x}, PC {:#018x}, syndrome {:#018x}",
                    memory_access.gpa, memory_access.header.pc, memory_access.syndrome
                );
                VirtualProcessorExitContext::Mmio(MmioAccess::new_aarch64(
                    memory_access.gpa,
                    memory_access.header.pc,
                    memory_access.syndrome,
                ))
            },
            EXIT_REASON_CANCELED => VirtualProcessorExitContext::Interrupted,
            EXIT_REASON_ARM64_RESET => {
                let reset = unsafe { exit_context.data.reset };
                match reset.reset_type {
                    RESET_TYPE_POWER_OFF => {
                        trace!("run(): ARM64 power-off reset exit");
                        VirtualProcessorExitContext::Reset(ResetKind::PowerOff)
                    },
                    RESET_TYPE_REBOOT => {
                        warn!("run(): ARM64 reboot reset exit");
                        VirtualProcessorExitContext::Reset(ResetKind::Reboot)
                    },
                    reset_type => {
                        warn!("run(): unknown ARM64 reset exit (type={reset_type})");
                        VirtualProcessorExitContext::Unknown
                    },
                }
            },
            EXIT_REASON_INVALID_VP_REGISTER
            | EXIT_REASON_UNRECOVERABLE_EXCEPTION
            | EXIT_REASON_UNSUPPORTED_FEATURE => {
                warn!("run(): fatal ARM64 WHP exit reason ({:#010x})", exit_context.exit_reason);
                VirtualProcessorExitContext::Unknown
            },
            reason => {
                warn!("run(): unhandled ARM64 WHP exit reason ({reason:#010x})");
                VirtualProcessorExitContext::Unknown
            },
        }
    }
}

impl Drop for VirtualProcessor {
    fn drop(&mut self) {
        unsafe {
            if let Err(error) = WHvDeleteVirtualProcessor(self.partition, self.index) {
                error!("failed to delete ARM64 vCPU (error={error:?})");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        REGISTER_CNTVCT_EL0,
        snapshot_register_names,
    };

    #[test]
    fn snapshot_registers_include_virtual_counter() {
        assert!(snapshot_register_names().contains(&REGISTER_CNTVCT_EL0));
    }
}
