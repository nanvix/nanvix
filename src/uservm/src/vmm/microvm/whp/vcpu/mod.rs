// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// The WHP vCPU uses u32 casts for Windows API parameters.
#![allow(clippy::cast_possible_truncation)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod exit;

//==================================================================================================
// Re-exports
//==================================================================================================

pub use exit::*;

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::whp::partition::WhpPartition;
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
    WHV_RUN_VP_EXIT_CONTEXT,
    WHV_RUN_VP_EXIT_REASON,
    WHvCancelRunVirtualProcessor,
    WHvCreateVirtualProcessor,
    WHvDeleteVirtualProcessor,
    WHvGetVirtualProcessorInterruptControllerState2,
    WHvGetVirtualProcessorRegisters,
    WHvRunVirtualProcessor,
    WHvSetVirtualProcessorInterruptControllerState2,
    WHvSetVirtualProcessorRegisters,
};

//==================================================================================================
// Constants
//==================================================================================================

/// RFLAGS interrupt enable flag (bit 9).
pub const RFLAGS_INTERRUPT_ENABLE: u64 = 1 << 9;

/// RFLAGS reserved bit that must always be set.
const RFLAGS_RESERVED_BIT1: u64 = 1 << 1;

// WHP register name constants.
const WHV_X64_REGISTER_RIP: WHV_REGISTER_NAME = WHV_REGISTER_NAME(16);
const WHV_X64_REGISTER_RFLAGS: WHV_REGISTER_NAME = WHV_REGISTER_NAME(17);
const WHV_X64_REGISTER_RAX: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0);
const WHV_X64_REGISTER_RBX: WHV_REGISTER_NAME = WHV_REGISTER_NAME(3);
const WHV_X64_REGISTER_CS: WHV_REGISTER_NAME = WHV_REGISTER_NAME(19);
/// Deliverability notifications register (controls interrupt-window exits).
const WHV_X64_REGISTER_DELIVERABILITY_NOTIFICATIONS: WHV_REGISTER_NAME =
    WHV_REGISTER_NAME(-2_147_483_644i32);
/// Pending interruption register (inject interrupts into the vCPU).
const WHV_X64_REGISTER_PENDING_INTERRUPTION: WHV_REGISTER_NAME = WHV_REGISTER_NAME(i32::MIN); // 0x80000000
/// TSC register (virtual/MSR register space).
const WHV_X64_REGISTER_TSC: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x2000);

/// Complete list of vCPU registers saved in a WHP snapshot.
const SNAPSHOT_REGISTER_NAMES: [WHV_REGISTER_NAME; 52] = [
    // General purpose registers.
    WHV_REGISTER_NAME(0x00), // RAX
    WHV_REGISTER_NAME(0x01), // RCX
    WHV_REGISTER_NAME(0x02), // RDX
    WHV_REGISTER_NAME(0x03), // RBX
    WHV_REGISTER_NAME(0x04), // RSP
    WHV_REGISTER_NAME(0x05), // RBP
    WHV_REGISTER_NAME(0x06), // RSI
    WHV_REGISTER_NAME(0x07), // RDI
    WHV_REGISTER_NAME(0x08), // R8
    WHV_REGISTER_NAME(0x09), // R9
    WHV_REGISTER_NAME(0x0A), // R10
    WHV_REGISTER_NAME(0x0B), // R11
    WHV_REGISTER_NAME(0x0C), // R12
    WHV_REGISTER_NAME(0x0D), // R13
    WHV_REGISTER_NAME(0x0E), // R14
    WHV_REGISTER_NAME(0x0F), // R15
    // Instruction pointer and flags.
    WHV_REGISTER_NAME(0x10), // RIP
    WHV_REGISTER_NAME(0x11), // RFLAGS
    // Segment registers.
    WHV_REGISTER_NAME(0x12), // ES
    WHV_REGISTER_NAME(0x13), // CS
    WHV_REGISTER_NAME(0x14), // SS
    WHV_REGISTER_NAME(0x15), // DS
    WHV_REGISTER_NAME(0x16), // FS
    WHV_REGISTER_NAME(0x17), // GS
    WHV_REGISTER_NAME(0x18), // LDTR
    WHV_REGISTER_NAME(0x19), // TR
    // Table registers.
    WHV_REGISTER_NAME(0x1A), // IDTR
    WHV_REGISTER_NAME(0x1B), // GDTR
    // Control registers.
    WHV_REGISTER_NAME(0x1C), // CR0
    WHV_REGISTER_NAME(0x1D), // CR2
    WHV_REGISTER_NAME(0x1E), // CR3
    WHV_REGISTER_NAME(0x1F), // CR4
    WHV_REGISTER_NAME(0x20), // CR8
    // Debug registers.
    WHV_REGISTER_NAME(0x21), // DR0
    WHV_REGISTER_NAME(0x22), // DR1
    WHV_REGISTER_NAME(0x23), // DR2
    WHV_REGISTER_NAME(0x24), // DR3
    WHV_REGISTER_NAME(0x25), // DR6
    WHV_REGISTER_NAME(0x26), // DR7
    // Extended control register.
    WHV_REGISTER_NAME(0x27), // XCR0
    // Virtual / MSR registers.
    WHV_REGISTER_NAME(0x2000), // TSC
    WHV_REGISTER_NAME(0x2001), // EFER
    WHV_REGISTER_NAME(0x2002), // KernelGsBase
    WHV_REGISTER_NAME(0x2003), // ApicBase
    WHV_REGISTER_NAME(0x2004), // PAT
    WHV_REGISTER_NAME(0x2005), // SysenterCs
    WHV_REGISTER_NAME(0x2006), // SysenterEip
    WHV_REGISTER_NAME(0x2007), // SysenterEsp
    WHV_REGISTER_NAME(0x2008), // Star
    WHV_REGISTER_NAME(0x2009), // Lstar
    WHV_REGISTER_NAME(0x200A), // Cstar
    WHV_REGISTER_NAME(0x200B), // Sfmask
];

// Compile-time assertion: WHV_REGISTER_VALUE must be exactly 16 bytes for safe raw serialization.
const _: () = assert!(mem::size_of::<WHV_REGISTER_VALUE>() == 16);

//==================================================================================================
// Structures
//==================================================================================================

/// Serializable vCPU state for WHP snapshot/restore.
///
/// Register values are stored as raw 16-byte arrays matching the binary layout of
/// `WHV_REGISTER_VALUE`. The order corresponds to [`SNAPSHOT_REGISTER_NAMES`].
#[derive(Serialize, Deserialize)]
pub struct VcpuState {
    /// Raw register values (each entry is a 16-byte `WHV_REGISTER_VALUE`).
    register_values: Vec<[u8; 16]>,
    /// LAPIC interrupt controller state (raw bytes).
    lapic_state: Vec<u8>,
}

///
/// # Description
///
/// A structure that represents a virtual processor backed by WHP.
///
pub struct VirtualProcessor {
    /// Handle to the WHP partition.
    partition: windows::Win32::System::Hypervisor::WHV_PARTITION_HANDLE,
    /// Processor index.
    index: u32,
    /// Processor state.
    online: bool,
    /// Exit status code.
    exit_status: u16,
    /// Deferred RIP value for PMIO reads (batched with RAX write).
    pending_new_rip: Option<u64>,
}

// SAFETY: `VirtualProcessor` only stores a `WHV_PARTITION_HANDLE` (an opaque OS handle) and
// primitive scalar fields (`u32`, `bool`, `u16`). None of these contain thread-affine state or
// Rust references. The WHP API synchronises concurrent access to the partition at the OS level.
// Therefore it is sound to move or share `VirtualProcessor` across threads.
unsafe impl Send for VirtualProcessor {}
unsafe impl Sync for VirtualProcessor {}

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Register count for WHP get/set operations.
fn reg_count(names: &[WHV_REGISTER_NAME]) -> u32 {
    u32::try_from(names.len()).unwrap_or(u32::MAX)
}

/// Gets virtual processor registers using raw WHP API.
unsafe fn whp_get_registers(
    partition: windows::Win32::System::Hypervisor::WHV_PARTITION_HANDLE,
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

/// Sets virtual processor registers using raw WHP API.
unsafe fn whp_set_registers(
    partition: windows::Win32::System::Hypervisor::WHV_PARTITION_HANDLE,
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

//==================================================================================================
// Implementations
//==================================================================================================

impl VirtualProcessor {
    ///
    /// # Description
    ///
    /// Creates a new virtual processor inside the given WHP partition.
    ///
    pub fn new(partition: &WhpPartition, index: u32) -> Result<Self> {
        trace!("VirtualProcessor::new(): index={index}");

        unsafe {
            WHvCreateVirtualProcessor(partition.handle(), index, 0).map_err(|e| {
                let reason: String = format!("failed to create virtual processor (error={e:?})");
                error!("VirtualProcessor::new(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        Ok(Self {
            partition: partition.handle(),
            index,
            online: false,
            exit_status: 0,
            pending_new_rip: None,
        })
    }

    ///
    /// # Description
    ///
    /// Resets the virtual processor registers to boot state.
    ///
    pub fn reset(&mut self, rip: u64, rax: u64, rbx: u64) -> Result<()> {
        trace!("reset(): rip={rip:#010x}, rax={rax:#010x}, rbx={rbx:#010x}");

        // Get current CS register.
        let cs_name: [WHV_REGISTER_NAME; 1] = [WHV_X64_REGISTER_CS];
        let mut cs_value: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        unsafe {
            whp_get_registers(self.partition, self.index, &cs_name, &mut cs_value).map_err(
                |e| {
                    let reason: String = format!("failed to get CS register (error={e:?})");
                    error!("reset(): {reason}");
                    anyhow::anyhow!(reason)
                },
            )?;
        }

        // Modify CS: set base = 0 and selector = 0.
        cs_value[0].Segment.Base = 0;
        cs_value[0].Segment.Selector = 0;

        // Set the CS register.
        unsafe {
            whp_set_registers(self.partition, self.index, &cs_name, &cs_value).map_err(|e| {
                let reason: String = format!("failed to set CS register (error={e:?})");
                error!("reset(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // Set RIP, RAX, RBX, RFLAGS.
        let reg_names: [WHV_REGISTER_NAME; 4] = [
            WHV_X64_REGISTER_RIP,
            WHV_X64_REGISTER_RAX,
            WHV_X64_REGISTER_RBX,
            WHV_X64_REGISTER_RFLAGS,
        ];

        let mut reg_values: [WHV_REGISTER_VALUE; 4] = [unsafe { mem::zeroed() }; 4];
        reg_values[0].Reg64 = rip;
        reg_values[1].Reg64 = rax;
        reg_values[2].Reg64 = rbx;
        reg_values[3].Reg64 = RFLAGS_RESERVED_BIT1;

        unsafe {
            whp_set_registers(self.partition, self.index, &reg_names, &reg_values).map_err(
                |e| {
                    let reason: String =
                        format!("failed to set general purpose registers (error={e:?})");
                    error!("reset(): {reason}");
                    anyhow::anyhow!(reason)
                },
            )?;
        }

        self.online = true;
        Ok(())
    }

    /// Powers off the virtual processor.
    pub fn poweroff(&mut self, exit_status: u16) {
        trace!("poweroff(): exit_status={exit_status}");
        self.online = false;
        self.exit_status = exit_status;
    }

    /// Gets the exit status code of the virtual processor.
    pub fn exit_status(&self) -> u16 {
        self.exit_status
    }

    /// Checks if the virtual processor is online.
    pub fn is_online(&self) -> bool {
        self.online
    }

    /// Returns the raw WHP partition handle (for use by the timer thread).
    pub fn partition_handle(&self) -> WHV_PARTITION_HANDLE {
        self.partition
    }

    /// Retrieves the LAPIC interrupt controller state as a raw byte vector.
    pub fn get_lapic_state(&self) -> Result<Vec<u8>> {
        // The WHP xAPIC state blob size is not publicly documented.
        // Use a 4 KiB buffer (the full APIC register page) which is
        // large enough for the xAPIC emulation state.
        let mut state = vec![0u8; 4096];
        let mut written: u32 = 0;
        unsafe {
            WHvGetVirtualProcessorInterruptControllerState2(
                self.partition,
                self.index,
                state.as_mut_ptr().cast(),
                state.len() as u32,
                Some(&mut written),
            )
            .map_err(|e| anyhow::anyhow!("failed to get LAPIC state (error={e:?})"))?;
        }
        state.truncate(written as usize);
        Ok(state)
    }

    /// Sets the LAPIC interrupt controller state from a raw byte slice.
    pub fn set_lapic_state(&mut self, state: &[u8]) -> Result<()> {
        unsafe {
            WHvSetVirtualProcessorInterruptControllerState2(
                self.partition,
                self.index,
                state.as_ptr().cast(),
                state.len() as u32,
            )
            .map_err(|e| anyhow::anyhow!("failed to set LAPIC state (error={e:?})"))?;
        }
        Ok(())
    }

    /// Saves the complete vCPU state for snapshot serialization.
    pub fn save_state(&self) -> Result<VcpuState> {
        let count: usize = SNAPSHOT_REGISTER_NAMES.len();
        let mut values: Vec<WHV_REGISTER_VALUE> = vec![unsafe { mem::zeroed() }; count];

        unsafe {
            whp_get_registers(self.partition, self.index, &SNAPSHOT_REGISTER_NAMES, &mut values)
                .map_err(|e| anyhow::anyhow!("save_state: failed to get registers ({e:?})"))?;
        }

        let register_values: Vec<[u8; 16]> = values
            .iter()
            .map(|v| {
                let mut bytes = [0u8; 16];
                // SAFETY: WHV_REGISTER_VALUE is a 16-byte C union; raw copy is valid.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        (v as *const WHV_REGISTER_VALUE).cast::<u8>(),
                        bytes.as_mut_ptr(),
                        16,
                    );
                }
                bytes
            })
            .collect();

        let lapic_state: Vec<u8> = self.get_lapic_state()?;

        Ok(VcpuState {
            register_values,
            lapic_state,
        })
    }

    /// Restores the complete vCPU state from a snapshot.
    pub fn load_state(&mut self, state: &VcpuState) -> Result<()> {
        if state.register_values.len() != SNAPSHOT_REGISTER_NAMES.len() {
            anyhow::bail!(
                "load_state: register count mismatch (expected={}, got={})",
                SNAPSHOT_REGISTER_NAMES.len(),
                state.register_values.len()
            );
        }

        let values: Vec<WHV_REGISTER_VALUE> = state
            .register_values
            .iter()
            .map(|bytes| {
                let mut v: WHV_REGISTER_VALUE = unsafe { mem::zeroed() };
                // SAFETY: WHV_REGISTER_VALUE is a 16-byte C union; raw copy is valid.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        (&mut v as *mut WHV_REGISTER_VALUE).cast::<u8>(),
                        16,
                    );
                }
                v
            })
            .collect();

        unsafe {
            whp_set_registers(self.partition, self.index, &SNAPSHOT_REGISTER_NAMES, &values)
                .map_err(|e| anyhow::anyhow!("load_state: failed to set registers ({e:?})"))?;
        }

        self.set_lapic_state(&state.lapic_state)?;
        self.online = true;

        Ok(())
    }

    /// Performs a fast LAPIC EOI for the given vector by reading and clearing
    /// the corresponding ISR bit via individual register access. This is much
    /// faster than the bulk `WHvGet/SetVirtualProcessorInterruptControllerState2`
    /// API because it reads/writes a single register instead of 4 KiB.
    pub fn write_apic_eoi(&self) -> Result<()> {
        // Vector 0x20 = 32: ISR register index 1 (32/32), bit 0 (32%32).
        // ISR register names: ApicIsr0=12304, ApicIsr1=12305, ...
        let isr_reg = WHV_REGISTER_NAME(12305); // WHvX64RegisterApicIsr1
        let mut values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        unsafe {
            whp_get_registers(self.partition, self.index, &[isr_reg], &mut values)
                .map_err(|e| anyhow::anyhow!("failed to read APIC ISR1 (error={e:?})"))?;
            // Clear bit 0 (vector 0x20 = 32, which maps to ISR1 bit 0).
            let isr_val = values[0].Reg64 & !1u64;
            values[0].Reg64 = isr_val;
            whp_set_registers(self.partition, self.index, &[isr_reg], &values)
                .map_err(|e| anyhow::anyhow!("failed to write APIC ISR1 (error={e:?})"))?;
        }
        Ok(())
    }

    /// Cancels a running virtual processor, causing `WHvRunVirtualProcessor` to
    /// return with the `Canceled` exit reason.
    pub fn cancel(&self) {
        unsafe {
            if let Err(e) = WHvCancelRunVirtualProcessor(self.partition, self.index, 0) {
                warn!("cancel(): failed to cancel vCPU (error={e:?})");
            }
        }
    }

    /// Reads the current RFLAGS register value.
    pub fn get_rflags(&self) -> Result<u64> {
        let names: [WHV_REGISTER_NAME; 1] = [WHV_X64_REGISTER_RFLAGS];
        let mut values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        unsafe {
            whp_get_registers(self.partition, self.index, &names, &mut values)
                .map_err(|e| anyhow::anyhow!("failed to get RFLAGS (error={e:?})"))?;
        }
        Ok(unsafe { values[0].Reg64 })
    }

    /// Sets the RFLAGS register to the given value.
    pub fn set_rflags(&mut self, rflags: u64) -> Result<()> {
        let names: [WHV_REGISTER_NAME; 1] = [WHV_X64_REGISTER_RFLAGS];
        let mut values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        values[0].Reg64 = rflags;
        unsafe {
            whp_set_registers(self.partition, self.index, &names, &values)
                .map_err(|e| anyhow::anyhow!("failed to set RFLAGS (error={e:?})"))?;
        }
        Ok(())
    }

    /// Reads the current TSC register value.
    pub fn get_tsc(&self) -> Result<u64> {
        let names: [WHV_REGISTER_NAME; 1] = [WHV_X64_REGISTER_TSC];
        let mut values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        unsafe {
            whp_get_registers(self.partition, self.index, &names, &mut values)
                .map_err(|e| anyhow::anyhow!("failed to get TSC (error={e:?})"))?;
        }
        Ok(unsafe { values[0].Reg64 })
    }

    /// Reads the current RIP register value.
    pub fn get_rip(&self) -> Result<u64> {
        let names: [WHV_REGISTER_NAME; 1] = [WHV_X64_REGISTER_RIP];
        let mut values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        unsafe {
            whp_get_registers(self.partition, self.index, &names, &mut values)
                .map_err(|e| anyhow::anyhow!("failed to get RIP (error={e:?})"))?;
        }
        Ok(unsafe { values[0].Reg64 })
    }

    /// Reads guest EIP, EBP, and CR3 for stack profiling.
    pub fn get_profile_regs(&self) -> Result<(u32, u32, u32)> {
        const WHV_X64_REGISTER_RBP: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x05);
        const WHV_X64_REGISTER_CR3: WHV_REGISTER_NAME = WHV_REGISTER_NAME(0x1E);
        let names: [WHV_REGISTER_NAME; 3] = [
            WHV_X64_REGISTER_RIP,
            WHV_X64_REGISTER_RBP,
            WHV_X64_REGISTER_CR3,
        ];
        let mut values: [WHV_REGISTER_VALUE; 3] = [unsafe { mem::zeroed() }; 3];
        unsafe {
            whp_get_registers(self.partition, self.index, &names, &mut values)
                .map_err(|e| anyhow::anyhow!("failed to get profile regs (error={e:?})"))?;
        }
        let eip = u32::try_from(unsafe { values[0].Reg64 })
            .map_err(|_| anyhow::anyhow!("RIP value exceeds u32 range"))?;
        let ebp = u32::try_from(unsafe { values[1].Reg64 })
            .map_err(|_| anyhow::anyhow!("RBP value exceeds u32 range"))?;
        let cr3 = u32::try_from(unsafe { values[2].Reg64 })
            .map_err(|_| anyhow::anyhow!("CR3 value exceeds u32 range"))?;
        Ok((eip, ebp, cr3))
    }

    /// Sets the RIP register to the given value.
    pub fn set_rip(&mut self, rip: u64) -> Result<()> {
        let names: [WHV_REGISTER_NAME; 1] = [WHV_X64_REGISTER_RIP];
        let mut values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        values[0].Reg64 = rip;
        unsafe {
            whp_set_registers(self.partition, self.index, &names, &values)
                .map_err(|e| anyhow::anyhow!("failed to set RIP (error={e:?})"))?;
        }
        Ok(())
    }

    /// Enables or disables interrupt deliverability notifications.
    /// When enabled, the vCPU will exit with `InterruptWindow` when IF transitions to 1.
    pub fn set_deliverability_notifications(&mut self, enable: bool) {
        let names: [WHV_REGISTER_NAME; 1] = [WHV_X64_REGISTER_DELIVERABILITY_NOTIFICATIONS];
        let mut values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        if enable {
            // Bit 1 = InterruptNotification.
            values[0].Reg64 = 0x2;
        }
        unsafe {
            if let Err(e) = whp_set_registers(self.partition, self.index, &names, &values) {
                warn!("set_deliverability_notifications(): failed (error={e:?})");
            }
        }
    }

    /// Injects an external interrupt using the WHP PendingInterruption register.
    /// The vCPU must be in an interruptible state (IF=1) for this to work.
    pub fn inject_pending_interruption(&mut self, vector: u32) -> Result<()> {
        // WHV_X64_PENDING_INTERRUPTION_REGISTER layout (64-bit):
        //   Bit  0:     InterruptionPending = 1
        //   Bits 1-3:   InterruptionType = 0 (External interrupt)
        //   Bits 16-31: InterruptionVector
        let pending_val: u64 = ((vector as u64) << 16) | 1u64;

        let names = [WHV_X64_REGISTER_PENDING_INTERRUPTION];
        let mut values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        values[0].Reg64 = pending_val;

        unsafe {
            whp_set_registers(self.partition, self.index, &names, &values)
                .map_err(|e| anyhow::anyhow!("inject_pending_interruption: {e:?}"))?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Runs the virtual processor until it exits.
    ///
    pub fn run(&mut self) -> VirtualProcessorExitContext {
        let mut exit_context: WHV_RUN_VP_EXIT_CONTEXT = unsafe { mem::zeroed() };
        let exit_context_size: u32 =
            u32::try_from(mem::size_of::<WHV_RUN_VP_EXIT_CONTEXT>()).unwrap_or(u32::MAX);

        let result: windows::core::Result<()> = unsafe {
            WHvRunVirtualProcessor(
                self.partition,
                self.index,
                (&mut exit_context as *mut WHV_RUN_VP_EXIT_CONTEXT).cast::<std::ffi::c_void>(),
                exit_context_size,
            )
        };

        match result {
            Ok(()) => self.parse_exit_context(&exit_context),
            Err(error) => {
                error!("run(): error running vCPU (error={error:?})");
                VirtualProcessorExitContext::Interrupted
            },
        }
    }

    /// Parses the WHP exit context and returns the appropriate exit context variant.
    fn parse_exit_context(
        &mut self,
        exit_context: &WHV_RUN_VP_EXIT_CONTEXT,
    ) -> VirtualProcessorExitContext {
        match exit_context.ExitReason {
            // I/O port access (WHvRunVpExitReasonX64IoPortAccess = 2).
            WHV_RUN_VP_EXIT_REASON(2) => {
                let io_context = unsafe { &exit_context.Anonymous.IoPortAccess };
                let port: u16 = io_context.PortNumber;

                // Extract fields from the AccessInfo bitfield.
                let access_info_raw: u32 = unsafe { io_context.AccessInfo.Anonymous._bitfield };
                let is_write: bool = (access_info_raw & 1) != 0;
                let access_size: u8 = ((access_info_raw >> 1) & 0x7) as u8;

                if access_size == 0 {
                    warn!("run(): unsupported pmio access width (width={access_size})");
                    return VirtualProcessorExitContext::Unknown;
                }

                if is_write {
                    let value: u32 = (io_context.Rax & 0xFFFF_FFFF) as u32;
                    let width: exit::PmioWidth = match access_size {
                        1 => exit::PmioWidth::Byte,
                        2 => exit::PmioWidth::Word,
                        4 => exit::PmioWidth::Dword,
                        _ => {
                            warn!("run(): unsupported pmio write width (width={access_size})");
                            return VirtualProcessorExitContext::Unknown;
                        },
                    };
                    self.advance_rip(exit_context);
                    VirtualProcessorExitContext::Pmio(exit::PmioAccess::PmioOut(port, value, width))
                } else {
                    let data: Vec<u8> = vec![0u8; access_size as usize];
                    // Defer RIP advance for reads — batched with RAX write.
                    self.pending_new_rip = Some(Self::next_rip(exit_context));
                    VirtualProcessorExitContext::Pmio(exit::PmioAccess::PmioIn(port, data))
                }
            },
            // Halt (WHvRunVpExitReasonX64Halt = 0x0008).
            WHV_RUN_VP_EXIT_REASON(8) => VirtualProcessorExitContext::Halt,
            // Canceled (WHvRunVpExitReasonCanceled = 0x2001).
            WHV_RUN_VP_EXIT_REASON(0x2001) => VirtualProcessorExitContext::Interrupted,
            // Interrupt window (WHvRunVpExitReasonX64InterruptWindow = 7).
            WHV_RUN_VP_EXIT_REASON(7) => VirtualProcessorExitContext::InterruptWindow,
            // Memory access (WHvRunVpExitReasonMemoryAccess = 1).
            WHV_RUN_VP_EXIT_REASON(1) => {
                let gpa: u64 = unsafe { exit_context.Anonymous.MemoryAccess.Gpa };
                trace!("run(): MMIO access at GPA {gpa:#010x}");
                VirtualProcessorExitContext::Mmio(gpa)
            },
            // Other exit reasons.
            reason => {
                warn!("run(): unhandled WHP exit reason ({reason:?})");
                VirtualProcessorExitContext::Unknown
            },
        }
    }

    /// Computes the RIP value after the current instruction from the exit context.
    fn next_rip(exit_context: &WHV_RUN_VP_EXIT_CONTEXT) -> u64 {
        // InstructionLength is packed in the lower 4 bits of VpContext._bitfield
        // (the InstructionLengthCr8 byte), NOT in ExecutionState._bitfield.
        let instruction_length: u64 = u64::from(exit_context.VpContext._bitfield & 0xF);
        exit_context.VpContext.Rip + instruction_length
    }

    /// Advances the instruction pointer past the current instruction after handling a VM exit.
    fn advance_rip(&mut self, exit_context: &WHV_RUN_VP_EXIT_CONTEXT) {
        let new_rip: u64 = Self::next_rip(exit_context);

        let reg_names: [WHV_REGISTER_NAME; 1] = [WHV_X64_REGISTER_RIP];
        let mut reg_values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
        reg_values[0].Reg64 = new_rip;

        unsafe {
            if let Err(e) = whp_set_registers(self.partition, self.index, &reg_names, &reg_values) {
                warn!("advance_rip(): failed to advance RIP (error={e:?})");
            }
        }
    }

    /// Sets RAX and flushes any deferred RIP advance in a single WHvSet call.
    pub fn set_rip_and_rax(&mut self, rax_value: u64) {
        if let Some(new_rip) = self.pending_new_rip.take() {
            let reg_names: [WHV_REGISTER_NAME; 2] = [WHV_X64_REGISTER_RIP, WHV_X64_REGISTER_RAX];
            let mut reg_values: [WHV_REGISTER_VALUE; 2] =
                [unsafe { mem::zeroed() }, unsafe { mem::zeroed() }];
            reg_values[0].Reg64 = new_rip;
            reg_values[1].Reg64 = rax_value;
            unsafe {
                if let Err(e) =
                    whp_set_registers(self.partition, self.index, &reg_names, &reg_values)
                {
                    warn!("set_rip_and_rax(): failed (error={e:?})");
                }
            }
        } else {
            // No pending RIP — just set RAX.
            let reg_names: [WHV_REGISTER_NAME; 1] = [WHV_X64_REGISTER_RAX];
            let mut reg_values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
            reg_values[0].Reg64 = rax_value;
            unsafe {
                if let Err(e) =
                    whp_set_registers(self.partition, self.index, &reg_names, &reg_values)
                {
                    warn!("set_rip_and_rax(): failed to set RAX (error={e:?})");
                }
            }
        }
    }

    /// Flushes any deferred RIP advance (for exits handled without setting RAX).
    pub fn flush_pending_rip(&mut self) {
        if let Some(new_rip) = self.pending_new_rip.take() {
            let reg_names: [WHV_REGISTER_NAME; 1] = [WHV_X64_REGISTER_RIP];
            let mut reg_values: [WHV_REGISTER_VALUE; 1] = [unsafe { mem::zeroed() }];
            reg_values[0].Reg64 = new_rip;
            unsafe {
                if let Err(e) =
                    whp_set_registers(self.partition, self.index, &reg_names, &reg_values)
                {
                    warn!("flush_pending_rip(): failed (error={e:?})");
                }
            }
        }
    }
}

impl Drop for VirtualProcessor {
    fn drop(&mut self) {
        trace!("VirtualProcessor::drop()");
        unsafe {
            if let Err(e) = WHvDeleteVirtualProcessor(self.partition, self.index) {
                error!("VirtualProcessor::drop(): failed to delete vCPU (error={e:?})");
            }
        }
    }
}
