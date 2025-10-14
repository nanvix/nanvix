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
};
use kvm_bindings::kvm_irqchip;
use kvm_ioctls::{
    Kvm,
    VmFd,
};

//==================================================================================================
// IrqChip State
//==================================================================================================

#[derive(Serialize, Deserialize)]
pub struct IrqChipState {
    /// Interrupt controller.
    irqchip: kvm_irqchip,
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

        let mut irqchip: kvm_irqchip = kvm_irqchip::default();
        if let Err(e) = vm_fd.get_irqchip(&mut irqchip) {
            let reason: String = format!("failed getting irqchip (error={e:?})");
            error!("get_state(): {reason}");
            anyhow::bail!(reason)
        };

        Ok(IrqChipState { irqchip })
    }

    pub fn restore_state(&mut self, vm_fd: &VmFd, state: &IrqChipState) -> Result<()> {
        trace!("restore_state()");

        if let Err(e) = vm_fd.set_irqchip(&state.irqchip) {
            let reason: String = format!("failed setting irqchip (error={e:?})");
            error!("set_state(): {reason}");
            anyhow::bail!(reason)
        };

        Ok(())
    }
}
