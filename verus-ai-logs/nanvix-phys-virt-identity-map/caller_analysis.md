# Caller Analysis: mm::virt::identity_map

Scope: the three verification-order target functions only —
`identity_map_page`, `ensure_pt`, `ensure_pte`.

## Script Output
See: `verus-ai-logs/nanvix-phys-virt-identity-map/find_callers_lsp_output.md`
(raw output of `find_callers_lsp.py`).

Summary from the script (rust-analyzer LSP, intra-crate; crate `kernel` has no
external dependents):
- Total exec functions: 14 (7 pub/trait-pub, 7 private, 1 type `Cr3Guard`).
- The target module manages the kernel's lazy physical-memory identity map.

### Call sites for the in-scope functions

| Function | Visibility | Callers (file:line) |
|----------|-----------|---------------------|
| `identity_map_page` | `pub(crate)` (`#[verus_verify(external_body)]`) | **External:** `src/kernel/src/mm/phys/kframe.rs:101` (`KernelFrame::new`). **Internal:** `ensure_identity_mapped_range` (identity_map.rs:479), itself reached from `memcpy`/`memset`. |
| `ensure_pt` | private | **Internal only:** `init` (identity_map.rs:134), `identity_map_page` (identity_map.rs:669). |
| `ensure_pte` | private | **Internal only:** `identity_map_page` (identity_map.rs:673). |

Note: `find_callers_lsp.py` also lists `kframe.rs:87/119` as references, but those
are `ensures`/`@` lines inside spec annotations, not exec call sites. The single
real exec call from outside the module is `kframe.rs:101`.

## Trait Obligations
None. None of the three target functions participates in a trait impl. (The only
trait impl in the module is `Drop for Cr3Guard`, which is out of scope.)

## Caller Expectations

### `identity_map_page(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>`

This is the only target function with an out-of-module caller, so its contract is
the externally observable one. It is currently `#[verus_verify(external_body)]`
(trusted placeholder; listed as a not-yet-verified `mm::virt` dependency so that
verified `mm::phys` callers can invoke it).

External caller — `KernelFrame::new` (`mm/phys/kframe.rs:91-107`):
- Calls `identity_map_page(phys_addr)` purely for its **side effect**: making the
  frame's physical address accessible through the kernel identity map so that the
  handle's later `Deref`/`DerefMut`/`memset` accesses do not page-fault.
- **Assumes on success (`Ok(())`)**: after the call, the page containing
  `phys_addr` is identity-mapped in the kernel page directory (present PDE + present
  PTE), and the returned `KernelFrame` can be safely dereferenced. It then returns
  `Ok(Self { base })`.
- **Assumes on failure (`Err`)**: the mapping was not established; the error is
  propagated (logged and returned) and **no** `KernelFrame` is constructed. `base`
  is `Copy`, so nothing is consumed and the raw frame remains the caller's to
  release. The handle's own postcondition (`frame@ == base@`) only concerns the
  success branch.
- **Input contract the caller upholds**: `phys_addr` is page-aligned (built via
  `PageAligned::from_raw_value`, which fails before the call if unaligned).

Internal caller — `ensure_identity_mapped_range` (identity_map.rs:476-480):
- Calls it once per page over a page-aligned range; relies on **idempotence**: if
  the PTE is already present the call is a no-op success. Relies on `Ok(())`
  meaning "this page is now mapped" so the loop can proceed page by page.

Callers assume:
- Idempotent / no-op when the page is already mapped (already-present PDE and PTE).
- Success ⇒ the single page covering `phys_addr` is identity-mapped afterwards.
- No recursive physical-frame allocation: backing page tables come from the BSS
  pool, so calling this during frame setup cannot re-enter the frame allocator.
- Pre-init no-op: if the lazy mapper is not yet initialized
  (`KERNEL_PD_PADDR == 0`, boot page tables still active), the call returns
  `Ok(())` without doing anything — callers must tolerate this.

Callers don't care about:
- Whether a new page table had to be allocated vs. reusing an existing one.
- The physical address of any page table (the `usize` returned by `ensure_pt` is
  purely internal and never surfaces to callers).
- TLB invalidation details, the exact PDE/PTE flag bits, or which BSS slot is used.
- The PDE/PTE index arithmetic (`pd_index`/`pt_index`).

### `ensure_pt(pd: Table<PageDirectoryEntry>, pde_idx) -> Result<usize, Error>` (private)

No out-of-module callers. Internal contract expected by `init` and
`identity_map_page`:
- **Success**: the PDE at `pde_idx` is present afterwards, and the returned `usize`
  is the physical base address of a valid (zeroed-on-allocation, hence all-PTEs-
  absent when freshly created) page table for that PDE.
- **Idempotent**: if the PDE is already present, returns the existing PT physical
  address without allocating.
- **Failure modes the callers propagate** (`?`): `InvalidArgument` (bad PDE read),
  `OutOfMemory` (no BSS slot), `BadAddress` (PT frame number out of range). On
  failure no PDE is installed.
- `init` discards the returned address (only wants the side effect of
  pre-allocating every PDE in `[0, MEMORY_SIZE)`); `identity_map_page` feeds the
  returned address into `Table::from_address` to reach the PT for `ensure_pte`.

### `ensure_pte(pt: Table<PageTableEntry>, pte_idx, phys_addr) -> Result<(), Error>` (private)

Single internal caller `identity_map_page`. Expected contract:
- **Success**: the PTE at `pte_idx` is present and identity-maps `phys_addr`
  (frame = `phys_addr / PAGE_SIZE`); the TLB entry for `phys_addr` has been
  invalidated.
- **Idempotent**: if the PTE is already present, no-op success (does not rewrite
  flags or re-invalidate the TLB).
- **Failure**: `InvalidArgument` (bad PTE read) or `BadAddress` (frame number out
  of range); on failure no PTE is written. The caller simply propagates the error.

## Abstract Resource

From the caller's perspective this module manages the **kernel lazy
identity-map**: a partial function from a page-aligned physical address to a
"present in the kernel address space" state, backed by the kernel page directory
and a BSS-allocated pool of page tables. The only externally meaningful operation
in scope is `identity_map_page` — "ensure this physical page is reachable in the
kernel address space" — which callers treat as an idempotent, side-effecting
guarantee, not as a query returning data.

`ensure_pt` and `ensure_pte` are private sub-steps of that operation (and of
`init`'s bulk pre-allocation); their physical-address / page-table details are not
observable outside the module.

## Key Invariants (caller perspective)

- **Idempotence**: mapping an already-mapped page (or ensuring an already-present
  PDE/PTE) is a no-op success — safe to call repeatedly, e.g. per page in a range.
- **Map-on-success**: `identity_map_page(p) == Ok(())` ⇒ the page covering `p` is
  identity-mapped in the kernel PD afterwards (PDE present, PTE present and
  identity for `p`).
- **No-frame-recursion**: backing page tables are drawn from the BSS pool, never
  from the physical frame allocator, so identity-mapping a frame during frame
  setup cannot re-enter frame allocation.
- **All-or-nothing on failure**: an `Err` leaves no partially-installed entry that
  callers must clean up; the page is simply not (newly) mapped and nothing is
  consumed.
- **Pre-init safety**: before `init` publishes the kernel PD
  (`KERNEL_PD_PADDR == 0`), `identity_map_page` is a no-op success; after `init`,
  every PDE in `[0, MEMORY_SIZE)` is pre-allocated, so at runtime only PTE
  installation (never new PDE allocation) normally occurs.
- **Alignment precondition**: `phys_addr` passed to `identity_map_page` is
  page-aligned (enforced by the `PageAligned` type at the call site).
