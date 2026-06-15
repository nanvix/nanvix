# Property Analysis: `mm::virt::manager` (`VirtMemoryManager`)

Source: `src/kernel/src/mm/virt/manager.rs`
Module-scoped verification: `make verify-kernel MODULE=mm::virt::manager`

This analysis identifies *what* must hold for the nine in-scope entry points.
It expresses every observable effect in terms of the **inherited `VmemView`**
(from `mm::virt::vmem`) and the manager-level spec vocabulary defined in the
view design (`maps_user_run_with`, `region_user_unmapped`, `links_child_cow`,
`link_user_pages_pre`, `logically_writable`, `page_run`,
`spec_is_cow_write_fault`). The manager itself is zero-sized, so its own View is
unit; all state lives in the `Vmem` argument(s), the returned RAII values, and
the global physical-frame pool (external).

Scope reminder — top-level specs for exactly these (caller-before-callee):
`new_vmem` (L260), `link_user_pages` (L337), `try_resolve_cow_fault` (L599),
`try_unmap_upage` (L659), `alloc_upages` (L713), `ctrl_upage` (L865),
`alloc_kpage` (L900), `alloc_kpages` (L939), `load_elf` (L991). The private
helpers `link_one_user_page`, `rollback_linked_pages`,
`make_uninitialized_array`, and `init`/`get`/`get_mut`/`new` are out of scope
but their bodies must still verify when `external_body` is removed.

Legend: ✅ = success-path (Ok), ❌ = error-path (Err), ⟲ = liveness/termination.

---

## 1. Type Invariants (TYPE-N)

### TYPE-1 — Manager view is unit and stable
`VirtMemoryManagerView` is a marker (zero-sized) type; `inv(self)` is `true` and
`view(&self)` maps the singleton to the unit value. Consequently every method
trivially satisfies `self@ == old(self)@`. No field-level well-formedness exists
on the manager (initialization via `MEMORY_MANAGER_INIT` is established by
out-of-scope `init`/`get_mut`; in-scope methods always receive an already-valid
`&mut self`). This clause carries no information and is the *reason* the
abstraction boundary is the `Vmem` argument, not the manager.

### TYPE-2 — Inherited `VmemView::inv()` is the load-bearing type invariant
Every `Vmem` argument and every produced/returned `Vmem` satisfies
`vmem.inv() == vmem@.inv()`, i.e.:
- all `user` keys are page-aligned user addresses; all `kernel` keys are
  page-aligned kernel addresses (key well-formedness);
- every user/kernel frame is page-aligned and lies in a valid physical region;
- `pgdir` is page-aligned and in a valid physical region;
- **`cow ⟹ ¬perms.write`** for every present user page.
Each mutating in-scope method must *require* it on inputs and *ensure* it on
outputs.

### TYPE-3 — Returned `KernelPage` owns a valid frame
`alloc_kpage`'s `Ok(kpage)` yields a `KernelPage` whose backing frame is
page-aligned and within a valid physical region
(`is_page_aligned(kpage@) ∧ spec_is_physical_region(kpage@, page_size())`), and
which releases that frame exactly once on `Drop`. Likewise each entry of
`alloc_kpages`' filled `kframes` is a valid page-aligned physical frame.

### TYPE-4 — Copy-on-write read-only coupling (core of TYPE-2, called out)
For every present user page, `cow ⟹ ¬write`. This is the structural coupling
that makes CoW sound: a shared page is hardware read-only, so a write *must*
trap. `link_user_pages` (which sets CoW marks) and `try_resolve_cow_fault`
(which clears one) must both re-establish it.

---

## 2. Function Contracts (FN-N)

### `new_vmem(&self, vmem: &Vmem) -> Result<Vmem, Error>`

- **FN-1 ✅ Fresh empty user space cloned from parent.** `Ok(new) ⟹`
  `new.inv() ∧ new@.user == Map::empty() ∧ new@.kernel == vmem@.kernel`.
- **FN-2 ✅ Distinct, valid page directory.** `Ok(new) ⟹ new@.pgdir != vmem@.pgdir`
  and (from `new.inv()`) `new@.pgdir` is page-aligned and in physical memory.
  *Establishes GLOBAL-4.* **Note:** the current draft `#[verus_spec]` on L249–257
  omits this `pgdir` distinctness clause although the view design and GLOBAL-4
  require it; it should be added (the impl backs the child with a freshly
  allocated kernel frame, so distinctness holds).
- **FN-3 ✅/❌ Input untouched.** `vmem` is `&Vmem`; `vmem@` is unchanged on every
  path (read-only clone source).
- **FN-4 ❌ No partial address space, no frame leak.** `Err ⟹` no new `Vmem`
  exists and the kernel frame allocated for the would-be page directory is
  released (RAII `Drop` of `pgdir_page` when `Vmem::clone` fails) — *consumes
  GLOBAL-1*. Caller can `?`-propagate with nothing to undo.

### `link_user_pages(&mut self, parent: &mut Vmem, child: &mut Vmem) -> Result<(), Error>`

- **FN-5 (requires) Non-overlap precondition.**
  `link_user_pages_pre(parent@, child@)`: `child` has no user mapping at any
  vaddr where `parent` does. (Relied on by the rollback heuristic, which cannot
  distinguish a freshly linked child page from a pre-existing overlapping one.)
  Plus `parent.inv() ∧ child.inv()`.
- **FN-6 ✅ Forked CoW sharing.** `Ok(()) ⟹ links_child_cow(old(parent)@,
  parent@, old(child)@, child@)`:
  - child's user domain becomes `old(child).dom ∪ old(parent).dom`; each linked
    child page shares the parent's frame at the same vaddr;
  - a *logically-writable* parent page (`perms.write ∨ cow`) becomes CoW in
    **both** parent and child; a genuinely read-only parent page is shared
    read-only and **left untouched** in the parent;
  - parent's user domain and per-page frames are preserved (only CoW marks may
    flip); pre-existing non-overlapping child pages are preserved;
  - both kernel halves and both `pgdir`s are unchanged.
- **FN-7 ✅ Invariant preserved on both spaces.** `Ok(()) ⟹ parent.inv() ∧
  child.inv()` (TYPE-2/TYPE-4 hold: any newly CoW page is forced read-only).
- **FN-8 ❌ Rollback / well-formedness.** `Err ⟹ parent.inv() ∧ child.inv()`.
  The *intended* (caller-expected, docstring-stated) property is **full
  rollback**: `parent@ == old(parent)@ ∧ child@ == old(child)@`. See
  **Suspected Bugs SB-1** — the chunked rollback path
  (`rollback_linked_pages`) deliberately does *not* clear parent CoW marks
  installed by earlier fully-linked chunks, so full parent restoration may be
  unprovable; the strongest honest Err guarantee is invariant preservation plus
  "every child page this call linked is unmapped."
- **FN-9 ❌ No frame leak on rollback.** Every shared reference acquired by
  `UserFrame::share` during linking is released (child `unmap` → `UserFrame`
  `Drop`) when rollback runs. *Consumes GLOBAL-1.*

### `try_resolve_cow_fault(&mut self, vmem, fault_addr, error_code) -> Result<bool, Error>`

Let `v = page_base(fault_addr)`.

- **FN-10 ✅(true) Resolved CoW write.** `Ok(true) ⟹
  spec_is_cow_write_fault(error_code) ∧ old(vmem)@.user_mapped(v) ∧
  old(vmem)@.user[v].cow ∧ (∃ f. is_page_aligned(f) ∧
  spec_is_physical_region(f, page_size()) ∧ vmem@ == old(vmem)@.spec_resolve_cow(v, f))`.
  The faulting page is privatized (fresh frame `f`, writable, CoW cleared) and
  the old shared reference released. *Consumes GLOBAL-2.*
- **FN-11 ✅(false) Complete negative characterization.** `Ok(false) ⟹ vmem@ ==
  old(vmem)@ ∧ (¬spec_is_cow_write_fault(error_code) ∨ ¬spec_is_user_addr(v) ∨
  ¬old(vmem)@.user_mapped(v) ∨ ¬old(vmem)@.user[v].cow)`. This is the *complete*
  set of reasons `false` is returned; the state is untouched.
- **FN-12 ❌ Failure preserves state.** `Err ⟹ vmem@ == old(vmem)@` — it *was* a
  CoW write fault (FN-13) but privatization failed (e.g. no free frame); no
  partial mutation is observable.
- **FN-13 ❌ Invalid/non-user addresses never `Err`.** A non-present / non-write
  / non-user error code, or a non-user / unrepresentable `fault_addr`, yields
  `Ok(false)` — these are checked *before* `vmem` is touched. `Err` implies a
  genuine CoW fault.
- **FN-14 ✅/❌ Invariant preserved.** `final(vmem).inv()` on every path.

### `try_unmap_upage(&mut self, vmem, vaddr) -> Result<bool, Error>`

- **FN-15 ✅(true) Present page removed.** `Ok(true) ⟹ old(vmem)@.user_mapped(vaddr@)
  ∧ vmem@ == old(vmem)@.spec_unmap(vaddr@) ∧ final(vmem).inv()`. The backing user
  frame is freed (RAII `Drop`). *Consumes GLOBAL-1.*
- **FN-16 ✅(false) Idempotent benign skip.** `Ok(false) ⟹
  ¬old(vmem)@.user_mapped(vaddr@) ∧ vmem@ == old(vmem)@`. A second call on an
  already-unmapped page returns `Ok(false)` (relied on by teardown/rollback
  loops); this is *not* an error.
- **FN-17 ❌ Failure preserves state.** `Err ⟹ vmem@ == old(vmem)@` (single
  delegated `unmap`; relies on vmem `unmap`'s error-state-preservation).

### `alloc_upages(&mut self, vmem, vaddr, access, clear, nframes, uframes) -> Result<(), Error>`

- **FN-18 (requires) Scratch buffer empty.** `old(uframes)@.len() == 0` and
  (dynamically validated) `uframes.capacity() >= nframes`. Violation ⟹
  `InvalidArgument` with **no side effects** (vmem and uframes unchanged).
- **FN-19 ❌ Range validation errors.** A non-user or zero/overflowing range ⟹
  `BadAddress`; an `nframes * PAGE_SIZE` multiplication overflow ⟹
  `InvalidArgument`; an already-mapped page in the range ⟹ `ResourceBusy`. The
  static characterization of the legal case is
  `region_user_unmapped(old(vmem)@, vaddr@, nframes)`. All these early returns
  leave `vmem@` and `uframes` unchanged.
- **FN-20 ✅ Contiguous run mapped.** `Ok(()) ⟹ maps_user_run_with(old(vmem)@,
  vmem@, vaddr@, nframes, access.perms_view())`: the user domain grows by exactly
  `page_run(vaddr@, nframes)`; pages outside the run are bit-for-bit preserved;
  each new page has perms `access.perms_view()` and is **not** CoW; kernel half
  and `pgdir` unchanged. Plus `final(vmem).inv() ∧ final(uframes)@.len() == 0`.
- **FN-21 ❌ Complete rollback.** `Err ⟹ vmem@ == old(vmem)@ ∧ final(vmem).inv() ∧
  final(uframes)@.len() == 0`. Any pages mapped before the failure are unmapped
  (via `try_unmap_upage`) and their frames freed; the `drain` drops any
  un-mapped frames. **No frame leak.** *Consumes GLOBAL-1.*
- **FN-22 ✅/❌ Scratch buffer drained.** On *both* paths `uframes` is left empty
  (drained / cleared), never reallocated.

### `ctrl_upage(&mut self, vmem, vaddr, access) -> Result<(), Error>`

- **FN-23 ✅ Permissions changed in place.** `Ok(()) ⟹ vmem@ ==
  old(vmem)@.spec_uctrl(vaddr@, access.perms_view()) ∧ final(vmem).inv()`. Only
  the named page's perms change; frame, CoW bit, domain, kernel half, pgdir
  preserved.
- **FN-24 ❌ Failure preserves state.** `Err ⟹ vmem@ == old(vmem)@` (permissions
  unchanged). Returned when the page is not mapped (delegated `uctrl` errors).
- **FN-25 (success precond, liveness) Page already mapped.** `vmem@.user_mapped(vaddr@)`
  is the condition under which `Ok` is *guaranteed* (the function does not
  allocate). It is a liveness precondition (see LIVE-4), not a safety `requires`:
  the Err path (FN-24) already handles the unmapped case soundly.

### `alloc_kpage(&mut self, clear) -> Result<KernelPage, Error>`

- **FN-26 ✅ Valid kernel page.** `Ok(kpage) ⟹ is_page_aligned(kpage@) ∧
  spec_is_physical_region(kpage@, page_size())`; if `clear`, the frame is zeroed
  (byte content excluded from the View — see §6). Owns its frame; freed on `Drop`.
- **FN-27 ❌ Nothing allocated.** `Err ⟹` no page produced; caller `?`-propagates.
  Manager view unaffected (TYPE-1).

### `alloc_kpages(&mut self, clear, count, kframes) -> Result<(), Error>`

- **FN-28 (requires) Scratch buffer empty.** `old(kframes)@.len() == 0` and
  (dynamically validated) `kframes.capacity() >= count`. Violation ⟹
  `InvalidArgument`, no side effects.
- **FN-29 ✅ Exactly `count` valid frames.** `Ok(()) ⟹ final(kframes)@.len() ==
  count`, each entry a valid page-aligned physical frame; each zeroed if `clear`.
- **FN-30 ❌ Empty, no leak.** `Err ⟹ final(kframes)@.len() == 0` — frames
  allocated before a `clear` failure are dropped/freed (`kframes.clear()`).
  *Consumes GLOBAL-1.*

### `load_elf(&mut self, vmem, elf) -> Result<(VirtualAddress, PageAligned<VirtualAddress>), Error>`

- **FN-31 ✅ Image loaded, user-confined.** `Ok((entry, args_vaddr)) ⟹
  final(vmem).inv() ∧ old(vmem)@.user.dom().subset_of(final(vmem)@.user.dom()) ∧
  final(vmem)@.kernel == old(vmem)@.kernel ∧ final(vmem)@.pgdir == old(vmem)@.pgdir ∧
  spec_is_user_addr(entry@) ∧ spec_is_user_addr(args_vaddr@) ∧
  is_page_aligned(args_vaddr@)`. The user domain only grows; the kernel half and
  page directory are untouched; both returned addresses are valid user-space
  addresses (and `args_vaddr` is page-aligned, ready for a follow-up
  `alloc_upages`). *Consumes GLOBAL-3.*
- **FN-32 ❌ No rollback, still well-formed.** `Err ⟹ final(vmem).inv()`. Partial
  mappings are acceptable because the caller discards the whole `vmem` on
  failure; the only obligation is that `vmem` remains a well-formed address
  space.

---

## 3. Module-Level Safety (MOD-N)

These hold across the relevant in-scope operations.

- **MOD-1 — User-space confinement.** No user-page operation
  (`alloc_upages`, `link_user_pages`, `try_unmap_upage`, `ctrl_upage`,
  `try_resolve_cow_fault`, `load_elf`) ever inserts, removes, or mutates a
  *kernel* mapping: `final@.kernel == old@.kernel`. User addresses touched lie in
  `[USER_BASE, USER_END)`; out-of-range inputs yield `BadAddress`/`Ok(false)`
  rather than corrupting the kernel half. *Consumes GLOBAL-3.*
- **MOD-2 — Page-directory invariance on existing spaces.** No mutating in-scope
  operation on an existing `vmem` changes its root: `final@.pgdir == old@.pgdir`.
  Only `new_vmem` mints a fresh, distinct `pgdir` (FN-2). *Supports GLOBAL-4.*
- **MOD-3 — Transactional (all-or-nothing) effects.** `alloc_upages` (FN-21),
  `alloc_kpages` (FN-30), `try_resolve_cow_fault` (FN-12), `try_unmap_upage`
  (FN-17), and `ctrl_upage` (FN-24) leave their target unchanged on `Err`.
  `link_user_pages` *intends* the same (FN-8) but see **SB-1**. `load_elf` is the
  deliberate exception (FN-32, caller-discards-on-fail).
- **MOD-4 — CoW read-only coupling preserved (TYPE-4 system-wide).** Every
  operation maintains `cow ⟹ ¬write` on every page of every touched `Vmem`.
  *Consumes GLOBAL-2.*
- **MOD-5 — Frame ownership balance (no leak / no double-free).** Across
  `new_vmem`, `link_user_pages`/rollback, `alloc_upages`/rollback,
  `try_unmap_upage`, `alloc_kpage`, `alloc_kpages`, every physical frame
  acquired is either retained by a live mapping/RAII handle or released exactly
  once; no map/unmap/share/drop path leaks or double-frees. *Consumes GLOBAL-1.*
- **MOD-6 — Caller-owned scratch-buffer discipline.** `uframes`/`kframes` are
  validated empty with sufficient capacity, used only as scratch, never
  reallocated, and left empty on return (FN-18/FN-22/FN-28/FN-30).
- **MOD-7 — Page alignment of inserted keys.** Every vaddr inserted into
  `user`/`kernel` is page-aligned (enforced by the `PageAligned` argument type
  and `page_run` membership), so TYPE-2 key well-formedness is preserved.

---

## 4. Liveness (LIVE-N)

- **LIVE-1 — `alloc_upages` conditional success.** If `uframes` is empty with
  `capacity ≥ nframes`, the range is entirely user-space and currently unmapped
  (`region_user_unmapped`), `nframes ≥ 1`, and the physical pool has `nframes`
  free user frames, then the call succeeds (`Ok(())`).
- **LIVE-2 — `try_unmap_upage` reclamation/idempotence.** After `Ok(true)`, the
  vaddr is unmapped and its frame returned to the pool, so the address becomes
  re-mappable; repeated calls thereafter return `Ok(false)` (no error storm in
  teardown loops).
- **LIVE-3 — `try_resolve_cow_fault` progress.** A genuine CoW write fault, given
  a free physical frame, is resolved (`Ok(true)`), leaving the page private,
  writable, and non-CoW — so the faulting instruction can retry without
  re-faulting (the page is `region_cow_resolved` at `v`).
- **LIVE-4 — `ctrl_upage` guaranteed success.** When the target page is mapped
  (FN-25) and `access` is representable, `ctrl_upage` succeeds.
- **LIVE-5 — Loop termination.** Every loop in the (in-scope and transitively
  reachable) bodies terminates, with these decreasing measures:
  - `alloc_upages` range-check `while` — `nframes - checked_count`;
  - `alloc_upages` map `loop` — the finite `drain` iterator (≤ `nframes` items);
  - `alloc_upages` rollback `while` — `mapped_count - rollback_count`;
  - `link_user_pages` outer `loop` — count of parent user pages not yet present
    in child (each full chunk removes `LINK_CHUNK`; terminates at `count == 0` or
    `count < LINK_CHUNK`);
  - `link_user_pages` inner `while slot_idx < count` — `count - slot_idx`;
  - `rollback_linked_pages` outer `loop` — count of child user pages with a
    parent counterpart still mapped (strictly decreases per chunk).

---

## 5. Cross-Module Properties (GLOBAL-N)

This module *consumes* (relies on) and *establishes* (contributes to) the
system invariants in `global_properties.md`:

- **GLOBAL-1 (frame ownership balance)** — consumed by FN-4, FN-9, FN-15, FN-21,
  FN-30 and MOD-5; backed by `vmem` map/unmap RAII and the `bitmap` allocator.
- **GLOBAL-2 (CoW soundness across address spaces)** — `link_user_pages` (FN-6)
  installs CoW marks on both sharers; `try_resolve_cow_fault` (FN-10) privatizes
  before any write; MOD-4 preserves `cow ⟹ ¬write` throughout.
- **GLOBAL-3 (user/kernel partition)** — consumed by FN-31 and MOD-1; user-page
  operations validate addresses against `spec_is_user_addr`/`spec_is_user_region`.
- **GLOBAL-4 (page-directory base consistency)** — `new_vmem` (FN-2) produces a
  distinct, valid, page-aligned physical `pgdir`; MOD-2 keeps it stable
  thereafter.

---

## 6. Explicitly Excluded (with reasons)

- **Byte-level memory contents** — `clear` zeroing, ELF segment bytes, the CoW
  copy. The View is structural/safety-only (ranges, rollback, CoW state, no
  leak); content correctness is out of the abstraction (consistent with the
  vmem View). We capture *that* a page is mapped/cleared, not its bytes.
- **Which physical frame the allocator picks** — kept existential
  (`∃ f. …`) in FN-10; pinning it over-specifies an allocator detail callers do
  not observe.
- **Internal page-table / PTE-bit layout** (AVL CoW bit encoding, page-directory
  walk, chunking strategy) — abstracted away by `VmemView`; callers never depend
  on it.
- **Global physical-frame pool state** (free list, refcounts) — a *global*
  resource deliberately left outside any single View (parallel to `VmemView`
  excluding CR3 and physical memory contents); pool depletion is a liveness
  concern handled, if ever, by a separate global abstraction.
- **Manager initialization (`MEMORY_MANAGER_INIT`)** — established by out-of-scope
  `init`/`get_mut`; in-scope methods receive an already-valid `&mut self`, so it
  is not a caller obligation here.
- **Physical contiguity of `alloc_kpages` frames** — the caller maps each frame
  independently (`map(KernelPage::new)`), so contiguity is not required; only
  count and per-frame validity (FN-29) matter.

---

## Suspected Bugs

### SB-1 — `link_user_pages` cannot guarantee full parent rollback on `Err`
The `link_user_pages` docstring (L312–315) promises that on failure "both
`parent` and `child` are restored to the state they had on entry … any
copy-on-write marks installed on `parent` are cleared." However, the across-chunk
rollback helper `rollback_linked_pages` (L489–500) *intentionally* leaves parent
CoW marks untouched, with a detailed rationale (it cannot reliably distinguish a
mark this call installed from one a prior fork installed). Therefore, if a
failure occurs after one or more *full chunks* have been linked (so those
parents were CoW-marked and the failure is handled by `rollback_linked_pages`,
not the per-page unwind in `link_one_user_page`), `parent@` is **not** restored
to `old(parent)@` — logically-writable parent pages remain CoW-marked (and hence
read-only).

Consequence for specs: the caller-expected/idealized full-rollback property
(FN-8, MOD-3) is likely **unprovable** as stated for the parent. The draft
`#[verus_spec]` Err clause (L330–333) already only asserts `parent.inv() ∧
child.inv()`, which is consistent with this weaker reality. This is a tension
between the two docstrings and the caller analysis's "full rollback" claim. It
may be an intentional design trade-off (extra harmless CoW faults vs. breaking a
co-sharer's CoW) rather than a memory-safety bug — but the *stated* contract and
the implementation disagree, so later phases must decide whether to (a) weaken
the Err guarantee to invariant-preservation + "all child-linked pages unmapped,"
or (b) treat the missing parent unmark as a defect to fix. Recorded for
investigation.

### SB-2 — `new_vmem` draft spec omits the `pgdir` distinctness guarantee
The draft `#[verus_spec]` on `new_vmem` (L249–257) ensures `new@.kernel ==
vmem@.kernel` and `new@.user == empty`, but **not** `new@.pgdir != vmem@.pgdir`,
even though the view design and GLOBAL-4 require a fresh, distinct page-directory
base (the implementation backs the child with a newly allocated kernel frame).
This is a spec gap, not a code defect: the property (FN-2) holds in the code and
should be added so GLOBAL-4 is actually established by this module rather than
merely assumed. Recorded so the specification phase adds the clause rather than
silently dropping it.

---

## Assumed External Specs

std-library operations used in the in-scope (and transitively reachable private)
bodies for which **vstd currently provides no specification**, so the
specification phase will need `assume_specification` (verified local code must
never use `external_body`). Each was checked against
`/mnt/toolchain/verus/vstd/std_specs/`.

- **`Vec::<T, A>::capacity`** — used by the buffer-contract checks in
  `alloc_upages` (L730) and `alloc_kpages` (L950). Searched `std_specs/vec.rs`:
  it defines `len`, `is_empty`, `clear`, `with_capacity`, `push`, `pop`,
  `reserve`, `drain`-less set, etc., but **no `capacity`** accessor. No spec
  found.
- **`Vec::<T, A>::drain` (and the `Drain` iterator's `next`/`Drop`)** — used by
  `alloc_upages` (L784–809) to consume frames while leaving the vector empty.
  Searched `std_specs/vec.rs` and all `std_specs/*.rs` for `drain`: **none**. No
  spec found (vstd models `into_iter` but not `drain`).
- **`<[T]>::iter_mut` (slice mutable iteration)** — used by `alloc_kpages`'
  `kframes.iter_mut().try_for_each(...)` (L960). Searched `std_specs/*.rs` for
  `iter_mut`: **none**. No spec found.
- **`Iterator::try_for_each`** — used by `alloc_kpages` (L960) to clear frames
  with short-circuit on error. Searched `std_specs/*.rs` for
  `try_for_each`/`for_each`: **none**. No spec found.
- **`MaybeUninit::<T>::write`** — used in `link_user_pages` (L365) and
  `rollback_linked_pages` (L512) to initialize a slot. `std_specs/maybe_uninit.rs`
  provides `new`, `uninit`, `assume_init`, `assume_init_ref`, `assume_init_mut`
  but **not `write`**. No spec found.
- **`MaybeUninit::<T>::assume_init_read`** — used in `link_user_pages` (L379) and
  `rollback_linked_pages` (L530). `std_specs/maybe_uninit.rs` has `assume_init`
  (by value) and the `_ref`/`_mut` borrows, but **not `assume_init_read`**. No
  spec found.

Not listed (specs already exist in vstd, or operation is local): `Vec::is_empty`,
`Vec::clear`, `Vec::len` (`std_specs/vec.rs`); `usize::checked_mul` /
`checked_add` (`std_specs/num.rs`); `MaybeUninit::uninit` /
`ManuallyDrop::new`/`into_inner`/`deref` (`std_specs/maybe_uninit.rs`,
`std_specs/manually_drop.rs`); `AtomicBool::load`/`store`
(`std_specs/atomic.rs`, and only in out-of-scope `init`/`get_mut`).
`::sys::mm::align_down`, `Vmem::*`, `UserFrame::share`, `KernelPage::new`,
`PageAligned::from_raw_value`, and `elf::elf32_load` are **local** code (must be
verified normally, never `external_body`). The
`[const { MaybeUninit::uninit() }; N]` array literal is isolated in the
out-of-scope helper `make_uninitialized_array`.
