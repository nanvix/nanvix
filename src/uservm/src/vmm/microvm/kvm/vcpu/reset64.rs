// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! 64-bit (long mode) guest reset logic for the virtual processor.

use super::VirtualProcessor;
use crate::vmm::kvm::vmem::VirtualMemory;
use ::anyhow::Result;
use ::kvm_bindings::{
    kvm_regs,
    kvm_segment,
    kvm_sregs,
};
use ::log::{
    error,
    trace,
};

/// RFLAGS value with interrupt-enable set.
const RFLAGS_INTERRUPT_ENABLE: u64 = 0x2;

impl VirtualProcessor {
    /// Resets the virtual processor for a 64-bit guest (long mode).
    pub(super) fn reset_64bit(
        &mut self,
        rip: u64,
        rax: u64,
        rbx: u64,
        vmem: &mut VirtualMemory,
    ) -> Result<()> {
        // Place boot structures below the kernel load address (0x100000).
        // Avoid 0x0000-0x1FFF (used by pvclock at 0x1000 and interrupt vectors).
        const BOOT_GDT_ADDR: u64 = 0x5000;
        const BOOT_PML4_ADDR: u64 = 0x6000;
        const BOOT_PDPT_ADDR: u64 = 0x7000;
        const BOOT_PD0_ADDR: u64 = 0x8000; // PD for first 1 GiB.
        const BOOT_PD1_ADDR: u64 = 0x9000; // PD for second 1 GiB.

        // Build GDT in guest memory.
        // Entry 0: null descriptor.
        // Entry 1: 64-bit code segment (selector 0x08).
        // Entry 2: 64-bit data segment (selector 0x10).
        // Entry 3: 64-bit user code segment (selector 0x1B, DPL=3).
        // Entry 4: 64-bit user data segment (selector 0x23, DPL=3).
        let gdt: [u64; 5] = [
            0x0000_0000_0000_0000, // null
            0x00AF_9A00_0000_FFFF, // code: L=1, D=0, P=1, DPL=0, S=1, type=0xA
            0x00CF_9200_0000_FFFF, // data: P=1, DPL=0, S=1, type=0x2
            0x00AF_FA00_0000_FFFF, // user code: L=1, D=0, P=1, DPL=3, S=1, type=0xA
            0x00CF_F200_0000_FFFF, // user data: P=1, DPL=3, S=1, type=0x2
        ];
        let gdt_bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(gdt.as_ptr().cast::<u8>(), gdt.len() * 8) };
        vmem.write_bytes(BOOT_GDT_ADDR, gdt_bytes)?;

        // Build page tables: identity-map first 2 GiB using 2 MiB pages.
        // PML4[0] → PDPT (user-accessible so user-mode can traverse to PD1).
        let pml4_entry: u64 = BOOT_PDPT_ADDR | 0x07; // present + writable + user.
        vmem.write_bytes(BOOT_PML4_ADDR, &pml4_entry.to_le_bytes())?;

        // PDPT[0] → PD0 (first 1 GiB, supervisor-only for kernel).
        // PDPT[1] → PD1 (second 1 GiB, user-accessible for user-space).
        let pdpt_entry0: u64 = BOOT_PD0_ADDR | 0x03; // present + writable.
        vmem.write_bytes(BOOT_PDPT_ADDR, &pdpt_entry0.to_le_bytes())?;
        let pdpt_entry1: u64 = BOOT_PD1_ADDR | 0x07; // present + writable + user.
        vmem.write_bytes(BOOT_PDPT_ADDR + 8, &pdpt_entry1.to_le_bytes())?;

        // PD0: 512 entries mapping [0, 1 GiB), each 2 MiB (PS bit set = 0x83).
        // Flags: present(0x1) + writable(0x2) + PS(0x80). Supervisor-only.
        for i in 0u64..512 {
            let pd_entry: u64 = (i * 0x20_0000) | 0x83;
            vmem.write_bytes(BOOT_PD0_ADDR + i * 8, &pd_entry.to_le_bytes())?;
        }

        // PD1: 512 entries mapping [1 GiB, 2 GiB), each 2 MiB.
        // Flags: present(0x1) + writable(0x2) + user(0x4) + PS(0x80). User-accessible.
        for i in 0u64..512 {
            let pd_entry: u64 = (0x4000_0000 + i * 0x20_0000) | 0x87;
            vmem.write_bytes(BOOT_PD1_ADDR + i * 8, &pd_entry.to_le_bytes())?;
        }

        // Configure system registers for long mode.
        let mut sregs: kvm_sregs = self.fd.get_sregs()?;

        // Enable PAE in CR4.
        sregs.cr4 = 1 << 5; // CR4.PAE

        // Set CR3 to PML4 base.
        sregs.cr3 = BOOT_PML4_ADDR;

        // Enable long mode in EFER MSR.
        sregs.efer = (1 << 8) | (1 << 10); // EFER.LME | EFER.LMA

        // Set CR0: PE (bit 0) + NE (bit 5) + PG (bit 31). Clear CD and NW.
        sregs.cr0 = (1 << 0) | (1 << 5) | (1 << 31); // CR0.PE | CR0.NE | CR0.PG

        // Set up code segment for 64-bit mode.
        sregs.cs = kvm_segment {
            base: 0,
            limit: 0xFFFF_FFFF,
            selector: 0x08,
            type_: 0x0A, // Execute/Read code segment.
            present: 1,
            dpl: 0,
            db: 0, // D=0 for 64-bit.
            s: 1,  // Code/data segment.
            l: 1,  // Long mode.
            g: 1,  // Granularity 4K.
            avl: 0,
            unusable: 0,
            padding: 0,
        };

        // Set up data segments.
        let data_seg: kvm_segment = kvm_segment {
            base: 0,
            limit: 0xFFFF_FFFF,
            selector: 0x10,
            type_: 0x02, // Read/Write data segment.
            present: 1,
            dpl: 0,
            db: 1,
            s: 1,
            l: 0,
            g: 1,
            avl: 0,
            unusable: 0,
            padding: 0,
        };
        sregs.ds = data_seg;
        sregs.es = data_seg;
        sregs.fs = data_seg;
        sregs.gs = data_seg;
        sregs.ss = data_seg;

        // Set up GDT register.
        sregs.gdt.base = BOOT_GDT_ADDR;
        sregs.gdt.limit = (u16::try_from(gdt.len())
            .map_err(|_| anyhow::anyhow!("GDT length overflows u16"))?
            * 8)
            - 1;

        if let Err(e) = self.fd.set_sregs(&sregs) {
            error!("reset_64bit(): failed to set sregs: {e:?}");
            return Err(anyhow::anyhow!("failed to set sregs: {e:?}"));
        }

        // Dump key sregs for debugging.
        let verify_sregs: kvm_sregs = self.fd.get_sregs()?;
        trace!(
            "reset_64bit(): sregs after set: cr0={:#x}, cr3={:#x}, cr4={:#x}, efer={:#x}, \
             cs.selector={:#x}, cs.l={}, cs.db={}, gdt.base={:#x}, gdt.limit={:#x}",
            verify_sregs.cr0,
            verify_sregs.cr3,
            verify_sregs.cr4,
            verify_sregs.efer,
            verify_sregs.cs.selector,
            verify_sregs.cs.l,
            verify_sregs.cs.db,
            verify_sregs.gdt.base,
            verify_sregs.gdt.limit
        );

        // Set up general purpose registers.
        let mut regs: kvm_regs = self.fd.get_regs()?;
        regs.rip = rip;
        regs.rax = rax;
        regs.rbx = rbx;
        regs.rflags = RFLAGS_INTERRUPT_ENABLE;
        // Set up stack pointer at the top of boot area (will be overwritten by kernel).
        regs.rsp = 0;
        self.fd.set_regs(&regs)?;

        Ok(())
    }
}
