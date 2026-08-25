# Virtual Memory Interaction Lines

The terminal MMU-state interactions are listed below.

## Paging-Structure Memory

- `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs:176-177` - write: zero all
  PDEs.
- `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs:183` - read: raw PDE.
- `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs:188` - write: raw PDE.
- `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs:572-573` - read: raw PTEs
  during bulk validation.
- `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs:595` - write: raw PTE during
  bulk fill.
- `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs:603-604` - write: zero all
  PTEs.
- `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs:610` - read: raw PTE.
- `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs:616` - write: raw PTE.
- `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs:658-659` - read: raw PTEs
  during iteration.
- `src/libs/arch/src/x86/mem/paging/table.rs:143` - volatile read of a PDE or PTE.
- `src/libs/arch/src/x86/mem/paging/table.rs:158` - volatile write of a PDE or PTE.

## x86_64 Hardware Paging Structures

In `src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs`:

- `91` - volatile write zeroing a reused paging-structure page; bypasses
  `write_entry`.
- `128` - `read_entry`: terminal volatile hardware-entry read.
- `139` - `write_entry`: terminal volatile hardware-entry write.

All other hardware PML4, PDPT, PD, and PT operations in this file converge on
`read_entry` and `write_entry`.

## Translation-Control Registers

- `src/kernel/src/hal/arch/x86/mem/mmu/mod.rs:27` - write CR3.
- `src/kernel/src/hal/arch/x86/mem/mmu/mod.rs:28` - read CR0.
- `src/kernel/src/hal/arch/x86/mem/mmu/mod.rs:30` - write CR0, enabling paging.
- `src/libs/arch/src/x86/cpu/cr3.rs:369,376` - read CR3.
- `src/libs/arch/src/x86/cpu/cr3.rs:398,404` - write CR3.
- `src/kernel/src/hal/arch/x86/asm/hooks.rs:413` - read CR3 during a context switch.
- `src/kernel/src/hal/arch/x86/asm/hooks.rs:429` - write CR3 during a context switch.
- `src/kernel/src/hal/arch/x86_64/asm/hooks.rs:380` - read CR3 during a context
  switch.
- `src/kernel/src/hal/arch/x86_64/asm/hooks.rs:401` - write CR3 during a context
  switch.
- `src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs:213` - read CR3 during hardware
  page-table initialization.

## Fault State

- `src/kernel/src/hal/arch/x86/asm/hooks.rs:248` - read CR2 for the faulting virtual
  address.
- `src/kernel/src/hal/arch/x86_64/asm/hooks.rs:208` - read CR2 for the faulting
  virtual address.

## TLB State

- `src/libs/arch/src/x86/mem/paging/mod.rs:65-68` - `invlpg`, invalidating one
  translation.
- `src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs:197` - x86_64-local `invlpg`.

CR3 writes also implicitly invalidate applicable cached translations.
