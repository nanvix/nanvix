# Caller Analysis: `mm::virt::identity_map`

## Scope

Verification-order target functions (only these are in scope):

- `identity_map_page` — `pub(crate)`
- `ensure_pt` — private helper
- `ensure_pte` — private helper

All other functions in the module (`init`, `memcpy`, `memset`, `sync_kernel_pdes`,
`with_kernel_address_space`, `Cr3Guard::drop`, `page_aligned_cover`,
`ensure_identity_mapped_range`, tests) are out of scope and must not be touched.
They appear below only as *callers* that constrain the in-scope functions.

## Script Output

See: `find_callers_output.md` (raw `find_callers_lsp.py` / rust-analyzer output).

Crate: `kernel`. No cross-crate dependents (intra-crate analysis only).

## Call Graph (in-scope functions)

```
init (L121)  ──────────────► ensure_pt (L508) ──► (PAGE_TABLE_ALLOCATOR.alloc_as)
                                  ▲
identity_map_page (L649) ─────────┘
        │
        └──────────────────► ensure_pte (L581) ──► (paging::invlpg)

ensure_identity_mapped_range (L463) ──► identity_map_page   [internal]
KernelFrame::new (phys/kframe.rs:104) ──► identity_map_page [external, pub(crate)]
```

### `identity_map_page` — callers
| Caller | Location | Call |
|--------|----------|------|
| `ensure_identity_mapped_range` (internal, private) | `identity_map.rs:478` | `identity_map_page(page_addr)?` (loops over all pages of a range; itself called by `memcpy`/`memset`) |
| `KernelFrame::new` (external, `pub(super)`) | `mm/phys/kframe.rs:104` | `identity_map_page(phys_addr)?` after `PageAligned::from_raw_value(base)` |

### `ensure_pt` — callers (private; both in-module)
| Caller | Location |
|--------|----------|
| `init` | `identity_map.rs:133` (pre-allocates a PT for every PDE in `[0, MEMORY_SIZE)`) |
| `identity_map_page` | `identity_map.rs:663` |

### `ensure_pte` — callers (private; in-module)
| Caller | Location |
|--------|----------|
| `identity_map_page` | `identity_map.rs:667` |

## Trait Obligations

None for the in-scope functions. (The module's only trait impl is `Drop for Cr3Guard`,
which is out of scope and unrelated to identity mapping.)

## Caller Expectations

### `identity_map_page(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>`

This is the only externally-visible (`pub(crate)`) in-scope function. The principal
external caller is `KernelFrame::new`, which must make a freshly-allocated physical
frame accessible through the kernel's identity map before handing out a `KernelFrame`
whose `Deref`/`DerefMut` dereference that physical address.

- **Callers assume on `Ok(())`:**
  - The page containing `phys_addr` is present and identity-mapped (V==P) in the kernel
    page directory, i.e. the page can subsequently be read/written through its physical
    address while the kernel address space is active. `KernelFrame::new` relies on this so
    that later `deref`/`deref_mut`/`clear` on the frame are sound.
  - The operation is **idempotent / no-op safe**: if the PTE is already present,
    nothing changes and `Ok(())` is still returned. Callers may map the same page
    repeatedly (e.g. multiple `KernelFrame`s over the same frame, or overlapping ranges
    in `memcpy`/`memset`).
  - The mapping has supervisor + read/write permissions (callers need to write frames,
    e.g. `KernelFrame::clear`, `memset`).
  - The TLB is consistent for the mapped page (a new mapping invalidates the stale TLB
    entry via `invlpg`), so the very next access through `phys_addr` sees the new mapping.
  - **Pre-init no-op:** if the lazy mapper is not yet initialized (`KERNEL_PD_PADDR == 0`,
    boot page tables still active), the call is a no-op returning `Ok(())`. Early-boot
    callers rely on this to succeed before `init` runs.
  - No physical frame is consumed from the frame allocator (page tables come from the BSS
    pool), so callers can use this from within frame-allocation paths (`KernelFrame::new`)
    without recursive frame allocation / deadlock.

- **Callers assume on `Err(e)`:**
  - The frame was *not* made accessible; the caller must not dereference it.
    `KernelFrame::new` propagates the error and does **not** construct the `KernelFrame`.
  - The error is a `sys::error::Error`; observed codes are `InvalidArgument` (failed PDE/PTE
    read), `OutOfMemory` (BSS PT pool exhausted, via `ensure_pt`), and `BadAddress`
    (frame number out of range). Callers branch on `Result`, not on the specific code.

- **Callers don't care about:**
  - Whether a new page table had to be allocated/installed (PDE absent) or the PDE was
    already present — only that the final PTE is present.
  - The page-directory / page-table index split (`pd_index`/`pt_index`), the BSS pool
    bump-allocator mechanics, or the `Table::from_address` raw-pointer construction.
  - The concrete PTE/PDE flag bit layout, beyond "present, writable, supervisor".
  - The internal split between `ensure_pt` and `ensure_pte`.

### `ensure_pt(pd: Table<PageDirectoryEntry>, pde_idx: TableIndex) -> Result<usize, Error>`

Private; callers are `init` and `identity_map_page` only.

- **Callers assume on `Ok(pt_paddr)`:**
  - The PDE at `pde_idx` in `pd` is present after the call, and `pt_paddr` is the
    physical address of the page table it points to (page-aligned, BSS-backed, all PTEs
    initially absent if freshly allocated).
  - Idempotent: if the PDE was already present, the *existing* PT's frame address is
    returned and the PDE is left unchanged. `init` relies on this to be safe to call once
    per PDE; `identity_map_page` relies on it so repeated maps reuse the same PT.
  - `pt_paddr` is immediately usable as `Table::from_address(pt_paddr)` for a follow-up
    `ensure_pte` (i.e. it is identity-mapped / directly addressable).
- **Callers assume on `Err`:** the PDE may be unchanged and no usable PT address is
  produced; the caller (`identity_map_page`, `init`) propagates the error with `?`.
  `OutOfMemory` specifically signals BSS PT-pool exhaustion.
- **Callers don't care about:** the allocator slot bookkeeping, the exact flag set written
  into the new PDE, or whether allocation happened.

### `ensure_pte(pt: Table<PageTableEntry>, pte_idx: TableIndex, phys_addr: usize) -> Result<(), Error>`

Private; sole caller is `identity_map_page`.

- **Callers assume on `Ok(())`:**
  - The PTE at `pte_idx` in `pt` is present and maps to the frame containing `phys_addr`
    (identity mapping, writable, supervisor). This is the step that actually realizes the
    V==P guarantee `identity_map_page` promises its callers.
  - Idempotent: if the PTE was already present it is left unchanged (no re-write, no extra
    `invlpg`) and `Ok(())` is returned.
  - On a freshly-installed mapping the corresponding TLB entry for `phys_addr` was
    invalidated, so the mapping is immediately effective.
- **Callers assume on `Err`:** the PTE was not installed; `identity_map_page` propagates
  the error. `BadAddress` means `phys_addr` produced an out-of-range frame number.
- **Callers don't care about:** the precise flag encoding or the TLB-invalidation
  mechanism, only that the entry is present and effective.

## Abstract Resource

This module manages the **kernel's identity map**: a single global mapping from physical
addresses to equal-valued virtual addresses (V == P) for the kernel address space, backed
by the kernel page directory plus BSS-pooled page tables. The in-scope functions answer one
question for callers — *"is this physical page reachable at its own address in the kernel
address space?"* — and, if not, make it so without consuming frame-allocator memory.

Key operations (in scope): map one page (`identity_map_page`), ensure the covering page
table exists (`ensure_pt`), ensure the leaf entry exists (`ensure_pte`).

## Key Invariants (caller perspective)

- **Idempotence / monotonicity:** mapping is never removed or remapped by these functions;
  a present PDE/PTE is left untouched. Repeated calls on the same page are safe no-ops.
  Once a page is identity-mapped, it stays mapped.
- **Post-success accessibility:** after `Ok` from `identity_map_page`, the page is present,
  writable, supervisor-only, and TLB-consistent in the kernel address space.
- **Page-table source is the BSS pool, not the frame allocator:** these functions never
  recurse into physical frame allocation, so they are safe to call from within frame
  allocation (`KernelFrame::new`).
- **Pre-init transparency:** before `init` publishes `KERNEL_PD_PADDR`, `identity_map_page`
  is a successful no-op (boot page tables already cover the relevant memory).
- **Failure leaves callers safe:** on `Err`, the target page must be treated as not
  accessible; callers do not proceed to dereference it.
- **Single-pass within a call:** `identity_map_page` touches exactly one PDE then one PTE
  (`ensure_pt` then `ensure_pte`) for the page; `pt_paddr` returned by `ensure_pt` is the
  table consumed by the subsequent `ensure_pte`.

## Pre-existing Specs (from upstream verification)

- `identity_map.spec.rs` and `identity_map.proof.rs` exist but are **empty stubs**
  (`verus! { } // verus!`). No `#[verus_spec]`, `View`, or `requires/ensures` annotations
  are present on any in-scope function. **No View type exists yet.**
- The external caller `KernelFrame::new` (`mm/phys/kframe.rs`) is already verified and is
  marked `#[verus_verify(external_body)]` with its own `requires/ensures` (over the frame
  abstraction `kf@ == base@`). It does **not** impose any spec onto `identity_map_page`;
  it simply `?`-propagates the `Result`. So there is no upstream bias to inherit — the View
  for this module can be designed cleanly from the expectations above.

### TCB / `external_body` note

`identity_map_page`, `ensure_pt`, and `ensure_pte` are **not** listed in
`verus-ai-logs/tcb-allowed.md`, so they must not be marked `external_body`. Their
dependencies that *are* allowed `external_body` (the BSS bump allocator
`FixedSizeBumpAllocator::alloc`/`alloc_as`) sit underneath `ensure_pt` and already carry
abstract `ensures`; the raw `Table::read`/`write` and `paging::invlpg` operations are the
trusted HAL boundary these functions build on.
