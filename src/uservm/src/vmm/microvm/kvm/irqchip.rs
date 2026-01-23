// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::serde::{
    Deserialize,
    Serialize,
};
use ::syslog::{
    error,
    trace,
    warn,
};
use kvm_bindings::{
    kvm_create_device,
    kvm_irqchip,
};
use kvm_ioctls::{
    Kvm,
    VmFd,
};

//==================================================================================================
// IrqChip State
//==================================================================================================

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
enum Backend {
    /// Traditional in-kernel PIC/IOAPIC + LAPIC setup created via `KVM_CREATE_IRQCHIP`.
    InKernelChip,
    /// LAPIC device created explicitly with `KVM_CREATE_DEVICE` (split irqchip setups).
    LapicDevice,
}

#[derive(Serialize, Deserialize)]
pub struct IrqChipState {
    /// Interrupt controller.
    backend: Backend,
    irqchip: Option<kvm_irqchip>,
}

//==================================================================================================
// IrqChip
//==================================================================================================

pub struct IrqChip {
    backend: Backend,
}

// kvm-bindings may not expose the LAPIC device type on all kernel versions; define it locally.
const KVM_DEV_TYPE_APIC: u32 = 3;

impl IrqChip {
    ///
    /// # Description
    ///
    /// Creates a interrupt controller and attaches it to a virtual partition.
    ///
    /// # Parameters
    ///
    /// - `kvm_fd`: Handle to the KVM.
    /// - `vm_fd`: Handle to the virtual machine.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn new(kvm_fd: &mut Kvm, vm_fd: &mut VmFd) -> Result<IrqChip> {
        trace!("new(): kvm_fd={kvm_fd:?}, vm_fd={vm_fd:?}");

        // Prefer split irqchip LAPIC device when available (e.g., microvm machine type).
        if kvm_fd.check_extension(kvm_ioctls::Cap::SplitIrqchip) {
            let mut device: kvm_create_device = kvm_create_device {
                type_: KVM_DEV_TYPE_APIC,
                fd: 0,
                flags: 0,
            };

            match vm_fd.create_device(&mut device) {
                Ok(_) => {
                    return Ok(IrqChip {
                        backend: Backend::LapicDevice,
                    });
                },
                Err(e) => {
                    warn!("new(): failed to create lapic device, falling back (error={e:?})");
                },
            }
        }

        // Check if KVM supports the legacy in-kernel irqchip path.
        let has_irqchip_support: bool = kvm_fd.check_extension(kvm_ioctls::Cap::Irqchip);
        if !has_irqchip_support {
            let reason: &str = "irqchip is not supported";
            error!("new(): {reason}");
            anyhow::bail!(reason);
        }

        vm_fd.create_irq_chip()?;

        Ok(IrqChip {
            backend: Backend::InKernelChip,
        })
    }

    pub fn save_state(&self, vm_fd: &VmFd) -> Result<IrqChipState> {
        trace!("save_state()");

        match self.backend {
            Backend::LapicDevice => Ok(IrqChipState {
                backend: Backend::LapicDevice,
                irqchip: None,
            }),
            Backend::InKernelChip => {
                let mut irqchip: kvm_irqchip = kvm_irqchip::default();
                if let Err(e) = vm_fd.get_irqchip(&mut irqchip) {
                    let reason: String = format!("failed getting irqchip (error={e:?})");
                    error!("get_state(): {reason}");
                    anyhow::bail!(reason)
                };

                Ok(IrqChipState {
                    backend: Backend::InKernelChip,
                    irqchip: Some(irqchip),
                })
            },
        }
    }

    pub fn restore_state(&mut self, vm_fd: &VmFd, state: &IrqChipState) -> Result<()> {
        trace!("restore_state()");

        match state.backend {
            Backend::LapicDevice => Ok(()),
            Backend::InKernelChip => {
                let Some(irqchip) = state.irqchip else {
                    let reason: &str = "missing irqchip state for in-kernel backend";
                    error!("restore_state(): {reason}");
                    anyhow::bail!(reason)
                };

                if let Err(e) = vm_fd.set_irqchip(&irqchip) {
                    let reason: String = format!("failed setting irqchip (error={e:?})");
                    error!("set_state(): {reason}");
                    anyhow::bail!(reason)
                };

                Ok(())
            },
        }
    }
}
