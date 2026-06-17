// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! 64-bit guest reset logic for the virtual processor.
//!
//! The guest starts in 16-bit real mode — identical to the 32-bit path. The kernel's own
//! trampoline code (in the `.trampoline` ELF section) handles the real-mode → protected-mode →
//! long-mode transition, building its own GDT and page tables along the way.

use super::{
    RFLAGS_INTERRUPT_ENABLE,
    VirtualProcessor,
};
use crate::vmm::kvm::vmem::VirtualMemory;
use ::anyhow::Result;

impl VirtualProcessor {
    /// Resets the virtual processor for a 64-bit guest.
    ///
    /// The guest is started in 16-bit real mode, just like the 32-bit path. The kernel's
    /// trampoline code transitions through protected mode to long mode, setting up its own
    /// GDT and identity-mapped page tables before jumping to the 64-bit entry point.
    pub(super) fn reset_64bit(
        &mut self,
        rip: u64,
        rax: u64,
        rbx: u64,
        _vmem: &mut VirtualMemory,
    ) -> Result<()> {
        // Reset system registers — start in 16-bit real mode (KVM default).
        let mut vcpu_sregs = self.fd.get_sregs()?;
        vcpu_sregs.cs.base = 0;
        vcpu_sregs.cs.selector = 0;
        self.fd.set_sregs(&vcpu_sregs)?;

        // Reset general purpose registers.
        let mut vcpu_regs = self.fd.get_regs()?;
        vcpu_regs.rip = rip;
        vcpu_regs.rax = rax;
        vcpu_regs.rbx = rbx;
        vcpu_regs.rflags = RFLAGS_INTERRUPT_ENABLE;
        self.fd.set_regs(&vcpu_regs)?;

        Ok(())
    }
}
