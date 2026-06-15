# Global (Cross-Module) Properties

System-level invariants that span more than one verified module. Each entry
notes the modules that participate and the per-module properties that establish
or consume it. IDs are `GLOBAL-N`.

Contributing modules so far: `mm::virt::vmem` (`Vmem`),
`mm::virt::manager` (`VirtMemoryManager`), and the lower-level
`mm::phys` / `bitmap` / `slab` / `raw-array` dependencies (verified specs).

---

## GLOBAL-1 — Physical frame ownership balance (no leak / no double-free)

Across all address spaces, every physical user frame is conserved: the sum of
per-frame references is unchanged by any operation that does not deliberately
allocate or free a frame, and reaches zero exactly once (freeing the frame).

- **Established by** `vmem`: `map`/`map_kpage` take ownership (domain grows,
  `uframe.leak()`), `unmap` returns the frame (domain shrinks), `map`-on-error
  drops the supplied frame, `Drop` releases everything once (vmem MOD-4,
  FN-8/FN-9/FN-3).
- **Consumed by** `manager`: fork (`link_user_pages` / `rollback_linked_pages`)
  and demand allocation (`alloc_upages` / `try_unmap_upage`) rely on map/unmap
  being exactly balanced so that a rollback restores the starting refcounts.
- **Backed by** `mm::phys` (`UserFrame`/`KernelFrame` RAII) and the `bitmap`
  allocator (a frame is free iff its bit is clear; alloc/free flip exactly one
  bit). Caveat: vmem **SB-2** (empty user page table leaked on `map`'s late
  error path) is a local exception to be investigated.

## GLOBAL-2 — Copy-on-write soundness across address spaces

A physical frame shared (CoW) between two address spaces is never mutated
in-place while still shared; it is privatized first.

- **Invariant link:** a CoW user page is hardware read-only in *every* sharing
  `Vmem` (vmem TYPE-4 / MOD-5: `cow ⇒ ¬write`).
- **User-mode writes** trap and are resolved lazily: `manager::attempt_cow`
  (guarded by `spec_is_cow_write_fault`) calls `vmem::resolve_cow_at`
  (vmem FN-14), which privatizes the page (fresh frame + copy, or last-reference
  fast path) and drops the shared reference.
- **Kernel-side physical-alias writes** must privatize eagerly first:
  `vmem::resolve_cow_for_region` (FN-15) is called before
  `copy_to_user_unaligned_unchecked` / `copy_user_to_user` write through the
  physical alias, which bypasses the page-table read-only bit. After it,
  `region_cow_resolved` holds over the write range.
- **Fork sharing** (`manager::links_child_cow`) sets the CoW marks in both
  parent and child via `vmem::mark_user_page_cow` (FN-11) for logically-writable
  pages, preserving TYPE-4 on both sides.
- Caveat: vmem **SB-3** (dst-side validation skipped in the copy dry run, and
  the committed path mutating CoW state) affects how tightly this property can
  be stated for the kernel-write path.

## GLOBAL-3 — Total user/kernel address partition

Every virtual address is classified user *xor* kernel, machine-wide, by pure
instance-independent predicates over the architectural layout constants
(`user_base()`, `user_end()`).

- **Defined by** `vmem`: `spec_is_user_addr` / `spec_is_kernel_addr`
  (`is_user_addr`, `is_kernel_addr`, vmem MOD-1, FN-16/FN-S1) and their region
  forms `spec_is_user_region` / `spec_is_kernel_region` (FN-17/FN-S2), which
  reject zero-size and overflowing ranges (vmem MOD-2).
- **Consumed by** syscall argument validation across `pm/kcall/*`,
  `pm/process/manager`, and `mm/elf.rs` (every untrusted-address guard), and by
  the copy validators whose single-sided checks are sound only under this
  partition (vmem MOD-3). The disjointness of `VmemView.user.dom()` and
  `VmemView.kernel.dom()` (vmem TYPE-7) is a direct consequence.

## GLOBAL-4 — Page-directory base consistency (MMU programming)

The value programmed into CR3 for an address space equals its abstract
page-directory base, which is a valid page-aligned physical frame.

- **Established by** `vmem`: `pgdir().physical_address().addr_nat() ==
  self@.pgdir` (FN-5) with `self@.pgdir` page-aligned and in physical memory
  (TYPE-6); `clone` guarantees a *distinct* base from its source (FN-2).
- **Consumed by** `manager`/`pm::process::manager` context switch
  (`pgdir().physical_address()` → CR3) and `vmem::load` (FN-4). The "active CR3"
  is global MMU state modeled outside any single `VmemView`.

---

## Notes

- `vmem`'s `internal_inv()` (TYPE-8) is the abstraction relation tying the
  concrete `LinkedList`/`Rc<RefCell>`/page-table representation to `VmemView`.
  It is currently a `true` stub and must be strengthened during proving so that
  all module functions verify without `external_body`; the GLOBAL-* properties
  above depend on it holding.
- Architectural constants are mirrored as spec literals in `vmem.spec.rs`
  (`user_base`, `user_end`, `phys_mem_size`, `page_size`) and must stay in sync
  with `sys::config` / `arch::mem` (build-time configuration). A drift here
  would silently weaken GLOBAL-3 / GLOBAL-4.
