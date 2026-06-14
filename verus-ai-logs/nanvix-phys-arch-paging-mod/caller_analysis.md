# Caller Analysis: `arch::x86::mem::paging` (`mod.rs`)

## Script Output

Ran:

```bash
python /home/ruize/verus-ai-exp/verus-ai/scripts/find_callers_lsp.py \
  /home/ruize/nanvix-phy/src/libs/arch/src/x86/mem/paging/mod.rs \
  --project-dir /home/ruize/nanvix-phy
```

Summary reported by the script:

- Crate: `arch`, depended on by `sysalloc`, `syscall`, `mkramfs`, `vfsd`,
  `kernel`, `uservm`, `arch-rust`, `test-kernel`, `test-mmio-fault`, `testd`.
- Total exec functions in scope: 1 (`invlpg`, `pub`).
- Reported `invlpg` → **0 external callers** (flagged as possible dead code).

### Script result is a FALSE NEGATIVE

The LSP run failed to resolve the call sites (likely because every caller goes
through the fully-qualified path `::arch::mem::paging::invlpg` and/or the
`unsafe { ... asm! }` body is not indexed). A manual repo-wide search shows the
function **is actively used** by the kernel:

```text
src/kernel/src/mm/virt/identity_map.rs:668          paging::invlpg(phys_addr)
src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs:210,329,385,433,498
                                                    ::arch::mem::paging::invlpg(page_address.into_raw_value())
src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs:170
                                                    ::arch::mem::paging::invlpg(pgtable_address.into_raw_value())
```

(The `hwpt.rs` `invlpg` at `hal/arch/x86_64/mem/mmu/hwpt.rs:165` is a *separate*,
locally-defined `invlpg` for the x86_64 HAL and is **not** a caller of this
module's function.)

In-scope target function for verification: `invlpg`.

## Trait Obligations

None. `invlpg` is a free `unsafe fn`, not part of any trait. There are no
implicit/runtime-dispatched callers (`Drop`, `GlobalAlloc`, `Iterator`, etc.).

## Caller Expectations

### `pub unsafe fn invlpg(vaddr: usize)`

All call sites follow the same pattern: **after** writing or clearing a page
table entry (PTE) or page directory entry (PDE) for a given address, the caller
calls `invlpg(addr)` to flush the corresponding TLB entry so the CPU cannot use
a stale translation.

Representative call sites:

- `identity_map.rs:668` — after `pt.write(pte_idx, new_pte)` installs a fresh
  identity-mapped PTE, flush the TLB for `phys_addr`.
- `page_table.rs:210` (and 329/385/433/498) — after `write_pte(...)`
  changes/clears a PTE, flush the TLB for `page_address` "so the CPU does not use
  a stale mapping to the old frame".
- `page_directory.rs:170` — after `write_pde(...)` clears a PDE, flush the TLB
  for `pgtable_address` "so the CPU does not use a stale PDE pointing to the
  freed page table".

**Callers assume:**
- It is a pure side-effecting operation on the CPU's TLB; it returns `()` and
  there is no error path. A successful return tells the caller nothing beyond
  "the TLB-invalidation instruction was issued for `vaddr`".
- After the call, any cached TLB translation for the page containing `vaddr` is
  invalidated, so the next access re-walks the (just-updated) page tables.
- It does **not** read or modify the page tables, kernel memory, the passed
  address's contents, or any Rust-visible state — it only affects hardware TLB
  state. It therefore cannot invalidate other data structures' invariants.
- `vaddr` is an ordinary `usize`; **any** value is accepted (the instruction is
  defined for any operand and simply has no effect if there is no matching TLB
  entry). Callers pass already-validated mapping addresses but do not rely on
  `invlpg` to range-check them.
- The safety contract is the caller's responsibility: it must be called from
  kernel mode (ring 0). All call sites are in kernel-mode MMU code and wrap the
  call in `unsafe { ... }` with a "called from kernel mode after modifying a
  PTE/PDE" SAFETY note.

**Callers don't care about:**
- The exact instruction encoding / assembly syntax (`invlpg ({0})` AT&T form,
  register selection, or the `nostack`/`preserves_flags` options).
- Any return value (there is none) or status code.
- The internal `core::arch::asm!` details — only the architectural TLB-flush
  effect matters.

## Pre-existing Specs (from upstream verification)

- Source: added during verification of the kernel `mm::virt::identity_map`
  module.
- Location: `src/kernel/src/mm/virt/identity_map.spec.rs:151`:
  ```rust
  pub assume_specification[ ::arch::mem::paging::invlpg ](vaddr: usize);
  ```
- The module's own `mod.spec.rs` is empty (`verus! { }`); there is **no**
  `verus_spec` annotation and **no** `View` type for `invlpg` in this module.

### Assessment
- Coverage: present but minimal. The kernel treats `invlpg` via an
  `assume_specification` with **no `requires` and no `ensures`** — i.e. an
  opaque, total, side-effect-only function with no precondition and a trivial
  postcondition.
- Strength: adequate for callers. Because the effect is purely on hardware TLB
  state (invisible to Verus' memory model) and there is no error path, an empty
  contract is faithful: callers genuinely require nothing of the return and only
  rely on the (un-modeled) hardware side effect.
- View design: N/A — `invlpg` is a standalone function operating on an external
  hardware resource (the TLB), not a data structure, so it needs no `View`. Any
  spec should remain a no-precondition / trivial-postcondition `unsafe fn`.

## Abstract Resource

From the caller's perspective this function manages the **CPU's Translation
Lookaside Buffer (TLB)** — specifically, it provides the single operation
"invalidate the cached address translation for the page containing `vaddr`",
used to keep the TLB coherent after a software change to a page table / page
directory entry. The TLB is hardware state outside Verus' memory model, so the
operation is best modeled as an opaque, total side effect.

## Key Invariants (caller perspective)

- `invlpg` has **no precondition** other than the documented safety obligation
  (kernel mode / ring 0); it accepts any `usize`.
- It is **side-effect-only on the TLB**: it does not touch page tables, frames,
  or any Rust-visible state, and therefore preserves every caller-side
  invariant (page-table well-formedness, mapping counts, allocator state, etc.).
- It is **infallible** (`-> ()`, no error path); a return conveys no
  information beyond "instruction issued".
- Correctness usage contract (caller-enforced, not enforceable by `invlpg`):
  call it *after* the in-memory PTE/PDE for `vaddr` has been updated, so the
  subsequent re-walk observes the new entry rather than a stale TLB cache.
