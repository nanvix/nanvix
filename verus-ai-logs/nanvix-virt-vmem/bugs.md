# Bugs — `mm::virt::vmem`

## BUG-001: Ghost `vmem_view` placeholder does not reflect real address-space state

- **Severity:** spec-internal (does not affect runtime; `#[cfg(verus_keep_ghost)]`-gated).
- **Where:** `src/kernel/src/mm/virt/vmem.rs`, `Vmem::new` and `Vmem::clone` ghost-field
  initialization (the `vmem_view: Ghost<...>` field).
- **Reported by:** specification reviewer, turn 1 (FR-5).

### Description
The ghost `vmem_view` field was initialized to a placeholder abstract state
(`VmemView { user: Map::empty(), kernel: Map::empty(), pgdir: 0 }`). This is wrong
in two ways:

1. `pgdir: 0` is not the real page-directory base. `VmemView::inv()` requires
   `spec_is_physical_region(self.pgdir, page_size())` and `is_page_aligned(self.pgdir)`,
   neither of which `0` satisfies. So `inv()` cannot hold for the constructed value
   without `admit()`.
2. For `clone`, the postcondition `v@.kernel == from@.kernel` claims the clone
   carries the source's kernel mappings, but an `empty()` kernel map contradicts that.

The original `pgdir: 0` literal additionally caused a hard compile error
(`E0308: expected nat, found integer`) because `pgdir` is `nat` and `0` is an `int`
literal — this blocked Verus from running at all.

### Fix applied (this phase)
- Compile fix (FR-1): the placeholder construction was replaced with
  `Ghost::assume_new()`, which yields an unconstrained ghost value of the correct
  type and gets past `cargo check` so Verus runs.
- Correctness (FR-5): the ghost must be made to mirror the *real* built state — the
  populated kernel map and the actual page-directory physical base
  (`pgdir == pd.physical_address()@`). This requires the construction loops in
  `new`/`clone` to maintain a loop invariant relating the in-progress ghost map to
  the page-directory contents. Marked as proving-phase work: in the spec phase the
  `Ok`-arm postconditions are discharged with `proof! { admit(); }` (allowed by the
  task statement), so the unconstrained ghost is sound *for the spec phase*. The
  proving phase must replace `assume_new()` with a real construction and remove the
  `admit()`.

### Status
- Compile blocker: **FIXED** (`assume_new`).
- Semantic placeholder: **RECORDED** — to be eliminated in the proving phase together
  with the `admit()` removal (reviewer FR-2/FR-4).

---

## SB-1: `kctrl` MMIO identity-map path can violate TYPE-5 and escapes `spec_kctrl`

- **Severity:** spec-vs-code mismatch (record-only; proving-phase investigation).
- **Where:** `vmem.rs` `Vmem::kctrl` (MMIO/absent-PTE branch); `vmem.spec.rs`
  `spec_kctrl` and `VmemView::inv()` TYPE-5 (`spec_is_physical_region`).
- **Reported by:** property analysis (SB-1), spec reviewer.

### Description
When `dry_run == false` and the kernel PTE is absent, `kctrl` *creates* an
identity-mapped entry with `frame_addr == vaddr` (`FrameAddress::new(.. vaddr ..)`).
1. `spec_kctrl` only models `kernel.insert(v, { perms, ..self.kernel[v] })` — it reads
   the *existing* `self.kernel[v]` frame and assumes the key is already present. It does
   **not** model inserting a brand-new identity mapping, so the real-run "absent PTE"
   branch is unspecified.
2. For a high kernel/MMIO vaddr (`>= user_end() = 0xf000_0000`), `frame == vaddr` is far
   above `phys_mem_size() = 0x800_0000`, so `spec_is_physical_region(frame, page_size())`
   is **false**; inserting such a mapping violates TYPE-5.

Either the View must special-case identity-mapped MMIO frames (relax TYPE-5) or `kctrl`'s
contract must restrict to already-present kernel pages. The dry-run branch also reports
success for an absent PTE while the real run performs a *create*, so the two passes do not
validate the same operation (couples with SB-3 / MOD-7).

### Status
**RECORDED** — proving phase to decide between View relaxation vs. contract restriction.

---

## SB-2: `map` / `map_kpage` leak an empty page table on a late error

- **Severity:** resource leak (record-only; proving-phase investigation).
- **Where:** `vmem.rs` `Vmem::map` (alloc + PDE-map a fresh user page table before the
  final `page_table.map(...)`), and the analogous `map_kpage` NOTE.
- **Reported by:** property analysis (SB-2).

### Description
A freshly allocated user page table is pushed to `user_page_tables` and mapped in `pgdir`
*before* the final `page_table.map(...)`. If that final `map` fails, the function returns
`Err` (correctly dropping the user frame) but the empty page table is left allocated and
PDE-mapped — the "if we fail beyond this point we should unmap the page table" NOTE is not
acted on. At View level `self@ == old@` still holds (empty page tables are not modeled), so
the error-arm spec is satisfiable, but it is a real physical resource leak contradicting the
MOD-4 "exactly balanced" intent.

### Status
**RECORDED** — proving phase to decide whether to add cleanup (code fix) or model empty
page tables (View change).

---

## SB-3: `copy_to_user_unaligned_unchecked` dry-run / commit asymmetry

- **Severity:** spec-vs-code mismatch (record-only; proving-phase investigation).
- **Where:** `vmem.rs` `Vmem::copy_to_user_unaligned_unchecked` and its scaffold contract
  (`ensures final(self)@ == old(self)@`).
- **Reported by:** property analysis (SB-3), spec reviewer.

### Description
**SB-3a (dry-run does not validate destination):** the destination-side checks
(`find_user_frame(vaddr)` and `is_physical_region(dst_phys_addr_raw, copy_size)`) are both
inside `if !dry_run`. The dry run only validates the *source* region, so a successful dry
run does **not** guarantee the real run won't `kpanic!` on an unmapped destination page or
out-of-range destination physical address. This partially violates the documented
"dry run ⇒ later real run cannot fail on validation" contract (caller_analysis FN) and
MOD-7, and undermines `copy_to_user_unaligned`'s "all-or-nothing, never panics" claim (FN-27)
for unmapped destinations.

**SB-3b (commit mutates the View):** the real run calls `resolve_cow_for_region(dst, size)`,
which mutates the View (`cow` / `write` / `frame` of dst pages). The committed path therefore
does **not** satisfy the scaffold `ensures final(self)@ == old(self)@`. The correct contract
should state that dst CoW pages in `[dst, dst+size)` become non-CoW / writable / freshly
framed (à la FN-15 `resolve_cow_for_region`), not full preservation. This is left as a
record because the correct contract depends on whether callers guarantee the destination is
pre-mapped and CoW-resolved (a deliberate design choice per the property analysis).

### Status
**RECORDED** — proving phase to tighten the dry-run validation (SB-3a) and replace the
`self@ == old@` ensures with CoW-resolution semantics on the destination range (SB-3b).
