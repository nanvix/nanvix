// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    trace,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use kvm_bindings::{
    KVM_IRQCHIP_IOAPIC,
    KVM_IRQCHIP_PIC_MASTER,
    KVM_IRQCHIP_PIC_SLAVE,
    kvm_irqchip,
};
use kvm_ioctls::{
    Kvm,
    VmFd,
};

//==================================================================================================
// IrqChip State
//==================================================================================================

#[derive(Serialize, Deserialize)]
pub struct IrqChipState {
    /// PIC master (chip_id = 0).
    pic_master: kvm_irqchip,
    /// PIC slave (chip_id = 1).
    pic_slave: kvm_irqchip,
    /// IOAPIC (chip_id = 2).
    ioapic: kvm_irqchip,
}

//==================================================================================================
// IrqChip
//==================================================================================================

pub struct IrqChip;

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

        // Check if KVM does not support irqchip.
        let has_irqchip_support: bool = kvm_fd.check_extension(kvm_ioctls::Cap::Irqchip);
        if !has_irqchip_support {
            let reason: &str = "irqchip is not supported";
            error!("new(): {reason}");
            anyhow::bail!(reason);
        }

        vm_fd.create_irq_chip()?;

        Ok(IrqChip)
    }

    pub fn save_state(&self, vm_fd: &VmFd) -> Result<IrqChipState> {
        trace!("save_state()");

        // `kvm_irqchip` is a tagged union: callers set `chip_id` and KVM_GET_IRQCHIP fills
        // in the corresponding chip's state. We must save all three chips (master PIC,
        // slave PIC, IOAPIC); if only the master PIC is preserved, on restore the slave
        // and IOAPIC come back zeroed and IRQ delivery is scrambled.
        let get = |chip_id: u32, label: &str| -> Result<kvm_irqchip> {
            let mut chip: kvm_irqchip = kvm_irqchip {
                chip_id,
                ..Default::default()
            };
            if let Err(e) = vm_fd.get_irqchip(&mut chip) {
                let reason: String = format!("failed getting {label} (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            }
            Ok(chip)
        };

        Ok(IrqChipState {
            pic_master: get(KVM_IRQCHIP_PIC_MASTER, "pic_master")?,
            pic_slave: get(KVM_IRQCHIP_PIC_SLAVE, "pic_slave")?,
            ioapic: get(KVM_IRQCHIP_IOAPIC, "ioapic")?,
        })
    }

    pub fn restore_state(&mut self, vm_fd: &VmFd, state: &IrqChipState) -> Result<()> {
        trace!("restore_state()");

        for (chip, label) in [
            (&state.pic_master, "pic_master"),
            (&state.pic_slave, "pic_slave"),
            (&state.ioapic, "ioapic"),
        ] {
            if let Err(e) = vm_fd.set_irqchip(chip) {
                let reason: String = format!("failed setting {label} (error={e:?})");
                error!("restore_state(): {reason}");
                anyhow::bail!(reason)
            }
        }

        Ok(())
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
