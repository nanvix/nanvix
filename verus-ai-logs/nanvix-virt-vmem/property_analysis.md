# Property Analysis: `mm::virt::vmem` (`Vmem`)

Source: `src/kernel/src/mm/virt/vmem.rs` (2162 LoC).
Abstract state model: `view_design.md` / `vmem.spec.rs` (`VmemView`).
Scope: the 26 listed top-level entry points plus the private helpers they call
(`find_user_frame`, `try_find_user_frame`, `lookup_user_page_table`,
`lookup_kernel_page_table`, `replace_user_page_cow_frame`, `is_kernel_addr`,
`is_kernel_region`, `allocate_user_page_table`, `allocate_kernel_page_table`)
and the compiler-dispatched `Drop`.

This document states **what** must hold, in terms of the designed `VmemView`
(`user: Map<nat, UserPageView>`, `kernel: Map<nat, KernelPageView>`,
`pgdir: nat`). It deliberately does not discuss proof tactics. (The source
currently carries scaffolding `#[verus_spec]` + `external_body` annotations from
an earlier phase; those `external_body` markers are placeholders that the
proving phase must remove — every property below must be discharged on the real
function bodies and on the abstraction relation `internal_inv()`.)

Naming of helpers below follows `vmem.spec.rs`: `spec_is_user_addr`,
`spec_is_kernel_addr`, `spec_is_user_region`, `spec_is_kernel_region`,
`spec_is_physical_region`, `is_page_aligned`, `page_base`, `page_offset`,
`page_size()`, `user_base()`, `user_end()`, `phys_mem_size()`, `rdwr_perms()`,
`perms_view()`, `addr_nat()`, and the `VmemView` transitions `spec_map`,
`spec_unmap`, `spec_map_kpage`, `spec_mark_cow`, `spec_unmark_cow`,
`spec_resolve_cow`, `spec_uctrl`, `spec_kctrl`, plus observers `user_mapped`,
`kernel_mapped`, `region_cow_resolved`.

---

## 1. Type Invariants (`VmemView::inv` / `Vmem::inv`)

These are the representation invariant of the abstract state and must hold on
entry and exit of every in-scope mutating method (`old(self).inv()` ⇒
`final(self).inv()`), and on every value produced by `new`/`clone`.

- **TYPE-1 — User keys well-formed.** `∀ v. user.contains_key(v) ⇒
  spec_is_user_addr(v) ∧ is_page_aligned(v)`. Every user mapping is keyed by a
  page-aligned user-space virtual address.
- **TYPE-2 — Kernel keys well-formed.** `∀ v. kernel.contains_key(v) ⇒
  spec_is_kernel_addr(v) ∧ is_page_aligned(v)`.
- **TYPE-3 — User frames valid.** `∀ v. user.contains_key(v) ⇒
  is_page_aligned(user[v].frame) ∧ spec_is_physical_region(user[v].frame,
  page_size())`. Every user page is backed by a page-aligned frame that lies in
  guest physical memory.
- **TYPE-4 — CoW implies read-only (per page).** `∀ v. user.contains_key(v) ∧
  user[v].cow ⇒ ¬user[v].perms.write`. A copy-on-write page is never hardware
  writable. This is the single semantic invariant fork / page-fault correctness
  hinges on.
- **TYPE-5 — Kernel frames valid.** `∀ v. kernel.contains_key(v) ⇒
  is_page_aligned(kernel[v].frame) ∧ spec_is_physical_region(kernel[v].frame,
  page_size())`. (See **SB-1**: the `kctrl` MMIO identity-map path appears able
  to violate this.)
- **TYPE-6 — Page directory base valid.** `is_page_aligned(pgdir) ∧
  spec_is_physical_region(pgdir, page_size())`. The CR3 base is a real,
  page-aligned physical frame.
- **TYPE-7 — User/kernel domains are disjoint.** `user.dom() ∩ kernel.dom() =
  ∅`. A consequence of TYPE-1 + TYPE-2 + the total exclusive partition
  (`spec_is_user_addr(v) ⇔ ¬spec_is_kernel_addr(v)`), but worth stating as an
  invariant callers rely on (a vaddr is classified user *xor* kernel, never
  both).
- **TYPE-8 — Abstraction relation (`internal_inv`).** The concrete
  representation (`pgdir`, `kernel_page_tables`, `kernel_pages`,
  `user_page_tables`) refines `VmemView`: for every present user PTE in some
  `user_page_tables` entry at base `B`, index `i`, the View has
  `user.contains_key(B + i·page_size())` with matching `frame`/`perms`/`cow`,
  and conversely; the analogous correspondence holds for kernel page
  tables/pages and for `pgdir.addr_nat() == self@.pgdir`. `internal_inv()` is
  currently `true` (a stub) and must be strengthened so that **every** function
  body below can be verified without `external_body`. This invariant is the
  obligation that connects the `LinkedList`/`Rc<RefCell>` bookkeeping to the
  abstract maps; it underwrites essentially every `FN-*` postcondition.

---

## 2. Function Contracts

For each entry point: precondition (requires), success postcondition, error
postcondition (state preservation), and the frame condition. Unless noted, all
mutators preserve `inv()` on success and leave `self@ == old(self)@` on error,
and all read-only methods (`&self`) leave `self@ == old(self)@`.

### Constructors / lifecycle

- **FN-1 `new(kernel_pages, kernel_page_tables) -> Result<Self,Error>`.**
  Success: `ret@.user == Map::empty()` (empty user half); `ret@.kernel` is
  exactly the set of supplied kernel pages mapped via the supplied kernel page
  tables (each at its registered vaddr with `rdwr_perms()`); `ret@.pgdir` is a
  fresh valid page-directory base; `ret.inv()`. Consumes both input lists.
  Error: no `Vmem` produced (nothing to preserve). Liveness coupling: the
  result must be `load`-able (TYPE-6 satisfied).
- **FN-2 `clone(from, pgdir_page) -> Result<Vmem,Error>`.** Requires
  `from.inv()`. Success: `new@.kernel == from@.kernel` (kernel half shared
  identically); `new@.user == Map::empty()` (user mappings are *not* copied
  here); `new@.pgdir != from@.pgdir` (a distinct page-directory base — callers
  use this difference to program a different CR3); `new.inv()`. The supplied
  `pgdir_page` becomes `new`'s page directory. Error: no `Vmem` produced.
- **FN-3 `Drop::drop`.** On scope exit every owned resource is released exactly
  once: user frames (refcount decrement / free via `UserFrame` drop), user page
  tables, the `Rc`-shared kernel pages/tables (refcount decrement), and the
  page-directory page. No leak, no double-free. `no_unwind`, opens no
  invariants. (Observable-state obligation; see MOD-4.)

### Activation / inspection

- **FN-4 `load(&self) -> Result<(),Error>`.** Requires `self.inv()`.
  `self@ == old@` (read-only w.r.t. the View). The activation effect (CR3 :=
  `self@.pgdir`) is *global* MMU state outside this View; the only View-level
  fact is preservation. Error path equally preserves `self@`.
- **FN-5 `pgdir(&self) -> &PageDirectory`.** Requires `self.inv()`. Returns a
  borrow whose `addr_nat() == self@.pgdir` (the stable CR3 base). Pure read; no
  mutation.

### Kernel-half mutation

- **FN-6 `map_kpage(&mut self, kpage, vaddr) -> Result<(),Error>`.** Requires
  `self.inv()` ∧ `spec_is_kernel_addr(vaddr)`. Success: `self@ ==
  old@.spec_map_kpage(vaddr, kpage.addr_nat(), rdwr_perms())` (kernel half gains
  the mapping at `rdwr_perms()`; user half and pgdir unchanged); `self.inv()`;
  the `Vmem` now owns `kpage`. Error (`TryAgain` on PDE read failure,
  `NoSuchEntry` if the page table cannot be located, OOM on page-table alloc):
  `self@ == old@`. Note: a backing kernel page table is allocated on demand;
  whether it is is invisible at View level (page tables are not modeled).
- **FN-7 `kctrl(&mut self, vaddr, access, dry_run) -> Result<(),Error>`.**
  Requires `self.inv()` ∧ `spec_is_kernel_addr(vaddr)`. Success with
  `dry_run==true`: validates only, `self@ == old@`. Success with
  `dry_run==false`: `self@ == old@.spec_kctrl(vaddr, access.perms_view())`
  (kernel page's perms updated). Error (`BadAddress` non-kernel, `TryAgain` PDE
  read, `NoSuchEntry` page table absent): `self@ == old@`. **Dry-run⇒commit
  determinism (MOD-7):** the dry-run validation predicate must be a pure
  function of `(self@, vaddr)` equal to the real-run validation predicate, so a
  successful dry run guarantees the matching real run passes validation. (See
  **SB-1**: the real-run "create identity-mapped PTE when absent" branch is not
  reflected by `spec_kctrl`, which assumes the key already exists.)

### User-half mutation

- **FN-8 `map(&mut self, uframe, vaddr, access) -> Result<(),Error>`.** Requires
  `self.inv()`. Success: `spec_is_user_addr(vaddr)` (held on the Ok path);
  `self@ == old@.spec_map(vaddr, uframe.addr_nat(), access.perms_view())` (user
  half gains a non-CoW mapping; kernel half & pgdir unchanged); `self.inv()`;
  the `Vmem` takes ownership of `uframe` (`uframe.leak()`). Error (`BadAddress`
  if `vaddr` not user, `TryAgain` PDE read, OOM): `self@ == old@`, **and the
  supplied `uframe` is dropped (its refcount released) so no frame leaks** —
  fork/alloc rollback paths depend on this balance (MOD-4). (See **SB-2**:
  on a failure *after* a new user page table was allocated, that empty page
  table is not unmapped — an internal resource leak that is invisible at View
  level but breaks the "exactly balanced" intent.)
- **FN-9 `unmap(&mut self, vaddr) -> Result<Option<UserFrame>,Error>`.**
  Requires `self.inv()`. `Ok(Some(frame))`: `old@.user_mapped(vaddr)`,
  `frame.addr_nat() == old@.user[vaddr].frame`, `self@ == old@.spec_unmap(vaddr)`
  (mapping removed), `self.inv()`; returning the `UserFrame` transfers ownership
  back to the caller (dropping it frees/derefs the frame). `Ok(None)`:
  `¬old@.user_mapped(vaddr)` (absent page is a benign no-op),
  `self@ == old@`. Error (`BadAddress` non-user, `TryAgain` PDE read,
  `NoSuchEntry` page table absent): `self@ == old@`. Internal: when a page
  table becomes empty (`nmapped()==0`) it is removed and its PDE unmapped — not
  observable at View level.
- **FN-10 `uctrl(&mut self, vaddr, access) -> Result<(),Error>`.** Requires
  `self.inv()` ∧ `self@.user_mapped(vaddr)`. Success: `self@ ==
  old@.spec_uctrl(vaddr, access.perms_view())` (the full permission triple of
  the page is replaced; frame & CoW bit unchanged); `self.inv()`. Error
  (`BadAddress` non-user, `TryAgain` PDE read, `NoSuchEntry`): `self@ == old@`.

### Copy-on-write transitions

- **FN-11 `mark_user_page_cow(&mut self, vaddr) -> Result<(),Error>`.**
  Requires `self.inv()` ∧ `self@.user_mapped(vaddr)` ∧
  `self@.user[vaddr].perms.write` (page present and writable). Success:
  `self@ == old@.spec_mark_cow(vaddr)` (clears `write`, sets `cow`); `self.inv()`
  (re-establishes TYPE-4). Error: `self@ == old@` — no half-marked state.
- **FN-12 `unmark_user_page_cow(&mut self, vaddr) -> Result<(),Error>`.**
  Requires `self.inv()` ∧ `self@.user_mapped(vaddr)`. Success:
  `self@ == old@.spec_unmark_cow(vaddr)` (sets `write`, clears `cow`) — the exact
  inverse of FN-11 on a writable-origin page; `self.inv()`. Error: `self@ ==
  old@`. **Round-trip (MOD-5/LIVE-3):** `unmark ∘ mark = identity` on a
  present writable page.
- **FN-13 `replace_user_page_cow_frame(&mut self, vaddr, new_frame) ->
  Result<FrameAddress,Error>`** (private; used by `resolve_cow_at`). Requires
  `self.inv()` ∧ `self@.user_mapped(vaddr)`. Success: returns
  `old@.user[vaddr].frame`, and `self@ == old@.spec_resolve_cow(vaddr,
  new_frame.addr_nat())` (repoint to `new_frame`, set `write`, clear `cow`);
  `self.inv()`. Error: `self@ == old@`.
- **FN-14 `resolve_cow_at(&mut self, vaddr) -> Result<bool,Error>`.** Requires
  `self.inv()`. `Ok(true)`: `old@.user_mapped(vaddr)` ∧ `old@.user[vaddr].cow`,
  and `∃ f. is_page_aligned(f) ∧ spec_is_physical_region(f, page_size()) ∧
  self@ == old@.spec_resolve_cow(vaddr, f)` (a privately-owned, writable frame
  now backs the page); `self.inv()`. The existential `f` covers both the
  allocate-and-copy path (fresh frame) and the **last-reference fast path**
  (refcount==1: `f` = the original frame, kept; `unmark` makes it writable). `Ok(false)`:
  `self@ == old@` ∧ `(¬old@.user_mapped(vaddr) ∨ ¬old@.user[vaddr].cow)` — page
  absent, non-user (returns `Ok(false)`), or not CoW. Error (OOM, copy
  failure): `self@ == old@`. **Idempotency (LIVE-2):** after `Ok(true)` the page
  is no longer CoW, so an immediate second call returns `Ok(false)`.
- **FN-15 `resolve_cow_for_region(&mut self, addr, size) -> Result<(),Error>`.**
  Requires `self.inv()`. Success with `size>0`:
  `self@.region_cow_resolved(addr, size)` — *every* user page overlapping
  `[addr, addr+size)` is non-CoW afterward; pages outside the range and
  non-CoW/non-user pages are unchanged; `self.inv()`. Success with `size==0`:
  `self@ == old@` (no-op). Error (`BadAddress` overflow): `self@ == old@`.
  This is the eager privatization that kernel-side physical-alias writers rely on
  (links to GLOBAL-2).

### Pure address classification (instance-independent)

- **FN-16 `is_user_addr(virt_addr) -> bool`.** Total, pure, side-effect-free:
  `ret == spec_is_user_addr(virt_addr)`. No `&self`.
- **FN-17 `is_user_region(start, size) -> bool`.** Total, pure:
  `ret == spec_is_user_region(start, size)` — `true` iff `size>0` ∧ the whole
  `[start, start+size)` lies in `[user_base(), user_end())`, with correct
  rejection of zero size and of end-overflow (the `checked_add(size-1)` `None`
  case ⇒ `false`).
- **FN-18 `is_physical_region(start, size) -> bool`.** Total, pure:
  `ret == spec_is_physical_region(start, size)` — `true` iff `size>0` ∧
  `start+size <= phys_mem_size()`. Safety gate on the unchecked copy path
  (not dead code).
- **FN-S1 `is_kernel_addr` / FN-S2 `is_kernel_region`** (private): `ret ==
  spec_is_kernel_addr(_)` / `spec_is_kernel_region(_)` — the complement
  predicates used by the copy validators.

### Queries (read-only; `self@ == old@`)

- **FN-19 `is_user_page_mapped(&self, vaddr) -> Result<bool,Error>`.**
  `Ok(b)`: `b == self@.user_mapped(vaddr)` (and `Ok(false)` when `vaddr` is not
  user — the function early-returns `BadAddress` only? **no**: it returns
  `Err(BadAddress)` for non-user `vaddr`; for user `vaddr` it returns
  `Ok(present)`). Pure query.
- **FN-20 `try_find_user_pte(&self, vaddr) -> Result<Option<PageTableEntry>,
  Error>`.** `Ok(Some(pte))` iff `self@.user_mapped(vaddr)`, with the decoded
  PTE satisfying `pte.frame_number()·page_size() == self@.user[vaddr].frame`,
  `pte.flags().is_writable() == self@.user[vaddr].perms.write`, and
  `pte.is_cow() == self@.user[vaddr].cow`; `Ok(None)` iff
  `¬self@.user_mapped(vaddr)`. Pure. (The current scaffolding ensures only the
  presence half `(opt is Some) == user_mapped`; the field-level decode
  correspondence is the deeper property callers read and should be captured.)
- **FN-21 `find_user_frame(&self, vaddr) -> Result<FrameAddress,Error>`**
  (private). Requires `self.inv()`. `Ok(f)`: `self@.user_mapped(vaddr)` ∧
  `f.addr_nat() == self@.user[vaddr].frame`. Error if absent.
- **FN-22 `try_find_user_frame(&self, vaddr) -> Result<Option<FrameAddress>,
  Error>`** (private). `Ok(Some(f))`: `self@.user_mapped(vaddr)` ∧
  `f.addr_nat() == self@.user[vaddr].frame`; `Ok(None)`:
  `¬self@.user_mapped(vaddr)`.
- **FN-23 `for_each_user_mapping(&self, f) -> Result<(),Error>`.** Requires
  `self.inv()` ∧ `f` accepts every present user mapping (`∀ v ∈ user.dom(),
  pte. call_requires(f,(v,pte))`). Success (**complete coverage**): `f` was
  invoked and returned `Ok` for *exactly* the page-aligned user vaddrs in
  `self@.user.dom()` — `∀ v ∈ user.dom(). ∃ pte. call_ensures(f,(v,pte),Ok)`;
  no key visited that is not present (the reconstructed `vaddr = base +
  idx·page_size()` is a present user key — relies on TYPE-8). Error: the first
  `Err` from `f` (or an internal overflow `BadAddress`) short-circuits and
  propagates. `self@ == old@` (callback-driven mutation is the caller's
  concern). Iteration order is unspecified; coverage is the guarantee.
- **FN-24 `user_vaddr_to_paddr(&self, vaddr) -> Result<usize,Error>`**
  (`stdio` feature). Requires `self.inv()`. `Ok(p)`:
  `self@.user_mapped(page_base(vaddr))` ∧ `p == self@.user[page_base(vaddr)].frame
  + page_offset(vaddr)` (translation includes the intra-page offset). Pure walk;
  `Err` if the page is unmapped. `self@ == old@`.

### Bulk copy paths (content-level effect; structurally `self@ == old@`)

The View deliberately does not model byte contents (see view_design "Rejected
Alternatives"); these are specified by their **address-validation error specs**
and **structure preservation**. The key positive guarantee is *which inputs are
accepted* (an Ok result certifies the ranges were valid), which is exactly the
safety property callers depend on before trusting untrusted addresses.

- **FN-25 `copy_from_user_unaligned(&self, dst, src, size) ->
  Result<(),Error>`.** Requires `self.inv()`. `Ok` ⇒
  `spec_is_user_region(src, size)` ∧ `spec_is_kernel_region(dst, size)` (so an
  Ok result certifies the validation passed). Errors: `InvalidArgument` iff
  `size==0`; `BadAddress` iff `src` not wholly user **or** `dst` not wholly
  kernel. `self@ == old@` (read-only `&self`; copies into kernel `dst`).
- **FN-26 `copy_to_user_unaligned_unchecked(&mut self, dst, src, size, dry_run)
  -> Result<(),Error>`.** Requires `self.inv()`. `self@ == old@` ∧ `self.inv()`
  in all cases (the structure is unchanged; CoW pre-resolution on the real run
  changes `cow`/`write`/`frame` of pages in `[dst,dst+size)` — note this means
  the *real run* is **not** strictly `self@ == old@` because it calls
  `resolve_cow_for_region`; the contract must reconcile this — see **SB-3b**).
  Error predicate (size==0 ⇒ `InvalidArgument`; src not kernel ⇒ `BadAddress`;
  dst not user ⇒ `BadAddress`; src physical range out of bounds ⇒ `BadAddress`)
  must be a **pure function of `(self@, dst, src, size)`** so a successful
  `dry_run` certifies the real run passes the *same* validation. Real-run
  internal failures (`find_user_frame`, dst physical-region check, `memcpy`)
  **panic** rather than return. **SB-3a:** those panicking checks are skipped in
  dry-run mode, so the documented "dry run ⇒ real run cannot fail validation"
  guarantee is only partial.
- **FN-27 `copy_to_user_unaligned(&mut self, dst, src, size) ->
  Result<(),Error>`.** Requires `self.inv()`. All-or-nothing checked copy: runs
  an internal dry run then the real copy; on `Err` nothing observable changed
  and it never panics; same address requirements as FN-26. `self.inv()` holds.
  (Same `self@` reconciliation caveat as FN-26 re: CoW resolution on the
  committed path.)
- **FN-28 `copy_user_to_user(src_vmem, src, dst_vmem, dst, size) ->
  Result<(),Error>`.** Requires `src_vmem.inv()` ∧ `dst_vmem.inv()`. `Ok` ⇒
  `spec_is_user_region(src, size)` ∧ `spec_is_user_region(dst, size)`. Errors:
  `InvalidArgument` (size==0), `BadAddress` (either range not user),
  `NoSuchEntry` (a source or destination page unmapped — surfaced by the dry
  run before any byte is written). Neither `src_vmem@` nor `dst_vmem@` changes.
  Caller is responsible for CoW pre-resolution on the destination (GLOBAL-2).
- **FN-29 `memset(&mut self, dst, value) -> Result<(),Error>`.** Requires
  `self.inv()` ∧ `self@.user_mapped(dst)`. `self@ == old@` ∧ `self.inv()`
  (content-only: fills the backing frame). Error: `self@ == old@`.

---

## 3. Module-Level Safety Properties

Hold across all operations; several extend beyond this module.

- **MOD-1 — Total exclusive address partition.** For every address `a`,
  `spec_is_user_addr(a) ⇔ ¬spec_is_kernel_addr(a)`. Every vaddr is user xor
  kernel. Foundation for TYPE-7 and for the "user/kernel never share a page"
  assumption the copy validators cite.
- **MOD-2 — Region predicates reject degenerate ranges.** `is_user_region`,
  `is_kernel_region`, `is_physical_region` all return `false` for `size==0` and
  for any range whose inclusive end overflows the address type (the
  `checked_add(size-1) == None` branch). No wrapping range is ever accepted as
  valid.
- **MOD-3 — Validation sufficiency from disjointness.** Because user and kernel
  spaces share no page (MOD-1), checking only the source range (user) and only
  the destination range (kernel), or vice-versa, is sufficient to guarantee a
  copy does not cross the boundary. The copy functions' single-sided region
  checks are sound precisely under MOD-1.
- **MOD-4 — Ownership balance (no leak / no double-free).** `map`/`map_kpage`
  take ownership of the supplied frame/page (domain grows; `uframe.leak()`);
  `unmap` returns the frame (domain shrinks; caller reclaims); `map` on error
  drops the supplied `uframe`; `Drop` releases everything owned exactly once.
  Across a fork-and-rollback sequence the per-frame refcount returns to its
  starting value. (Cross-`Vmem`; see GLOBAL-1. Caveat **SB-2** for the
  page-table leak on `map`'s error path.)
- **MOD-5 — Global CoW read-only invariant.** No present user page is ever
  simultaneously `cow` and `write` (TYPE-4 across all states). Every mutator
  that sets `cow` clears `write` (`spec_mark_cow`) and every mutator that sets
  `write` clears `cow` (`spec_unmark_cow`, `spec_resolve_cow`). This is the
  invariant that makes the fork CoW protocol sound: a shared frame can only be
  written after it is privatized.
- **MOD-6 — Query purity.** Every `&self` method (`load`, `pgdir`,
  `is_user_page_mapped`, `try_find_user_pte`, `find_user_frame`,
  `try_find_user_frame`, `for_each_user_mapping`, `user_vaddr_to_paddr`,
  `copy_from_user_unaligned`) and every classification function leaves `self@`
  unchanged.
- **MOD-7 — Dry-run ⇒ commit determinism.** For `kctrl` and
  `copy_to_user_unaligned_unchecked`, the dry-run validation outcome is a pure
  function of `(self@, args)` and equals the real-run validation outcome, so a
  successful dry run guarantees the matching real run will not fail validation
  (and `copy_to_user_unaligned` exposes this as a no-panic all-or-nothing copy).
  **This property is currently only partially upheld for the copy path — see
  SB-3.**
- **MOD-8 — Termination.** Every loop terminates: the `LinkedList` traversals
  (`new`, `clone`, `map_kpage`, `lookup_*`, `find_user_frame`,
  `try_find_user_frame`, `try_find_user_pte`, `for_each_user_mapping`, `unmap`,
  `Drop`) decrease on the remaining list length / iterator; the byte-walk loops
  in the copy paths and `resolve_cow_for_region` decrease on `size`/`remaining`
  or on the page index up to `last_page` (the `checked_add(PAGE_SIZE)` advance
  guarantees strict progress to the inclusive last page). No unbounded
  recursion exists.

---

## 4. Liveness / Functional Guarantees

- **LIVE-1 — CoW resolution availability.** If `self@.user_mapped(vaddr)` ∧
  `self@.user[vaddr].cow` and a frame can be allocated (or this space holds the
  last reference), `resolve_cow_at` returns `Ok(true)` and the page becomes
  privately writable. After releasing the shared reference, the frame is freed
  only when the last sharer resolves (resource reclamation).
- **LIVE-2 — Idempotent resolution.** A second `resolve_cow_at`/
  `resolve_cow_for_region` over an already-resolved page/region is a no-op
  returning `Ok` (no CoW page remains; `region_cow_resolved` is stable under
  re-application).
- **LIVE-3 — CoW round-trip recoverability.** `mark_user_page_cow` followed by
  `unmark_user_page_cow` restores the exact original mapping
  (`spec_unmark_cow(spec_mark_cow(v)) == original` on a present writable page),
  which fork rollback relies on to undo a half-applied link.
- **LIVE-4 — Unmap of absent page is benign.** `unmap` on a non-present user
  page returns `Ok(None)` (not an error), so lazily-allocated regions can be
  cleaned up uniformly.
- **LIVE-5 — Constructed spaces are usable.** `new`/`clone` produce a `Vmem`
  satisfying TYPE-6, hence `load`-able and translation-consistent
  (`pgdir().addr_nat() == self@.pgdir`).

---

## 5. Abstract State Concepts (mapping to `VmemView`)

- The address space is a pair of partial maps `user` / `kernel` plus a scalar
  `pgdir`; **presence = domain membership**. Mapping adds a key, unmapping
  removes it; all "is mapped?" queries are `contains_key`.
- **Permissions** are the triple `PagePerms{read, write, execute}` per mapping;
  only `write` is currently read by callers, but `uctrl`/`kctrl` set the full
  triple, so the triple is the honest abstraction.
- **CoW** is a per-user-page boolean coupled to `write` by TYPE-4; the three CoW
  transitions (`mark`, `unmark`, `resolve`) are the only ones that touch it.
- **Frames** are `nat` physical base addresses; the View tracks *which* frame
  backs a page, not its bytes. Translation (`user_vaddr_to_paddr`) composes the
  frame base with `page_offset`.
- **Ownership** is reflected *observably* by domain membership rather than an
  explicit refcount field: `map` grows `user`, `unmap` shrinks it and hands the
  frame back, `map`-error leaves `user` unchanged. The `Rc`/`UserFrame`/`Drop`
  machinery is the concrete realization (TYPE-8), not part of the View.
- **Kernel sharing**: `clone` copies `kernel` by value-equality
  (`new@.kernel == from@.kernel`), modeling the shared kernel half; the concrete
  `Rc` sharing is bookkeeping.

---

## 6. Cross-Module Connections

The verified dependency `manager.spec.rs` builds its fork / demand-paging
contracts directly on `VmemView` (`maps_user_run_with`, `links_child_cow`,
`logically_writable`, `link_user_pages_pre`). This module must provide the
per-page primitives those range-shapes are composed from:

- `links_child_cow` (fork) is realized by `for_each_user_mapping` (coverage,
  FN-23) + `map` (FN-8) + `mark_user_page_cow` (FN-11), and depends on
  TYPE-4/MOD-5 to make "logically writable ⇒ CoW in both" sound and on
  `unmap`/`unmark` (FN-9/FN-12) for `rollback_linked_pages`.
- `maps_user_run_with` (alloc_upages) is realized by `is_user_page_mapped`
  (range pre-check), `map`, and `memset`.
- `spec_is_cow_write_fault` page-fault handling is resolved by `resolve_cow_at`
  (FN-14) returning `Ok(true)`/`Ok(false)`.

See `global_properties.md` for the GLOBAL-* statements.

---

## Suspected Bugs

- **SB-1 — `kctrl` MMIO identity-map path can violate TYPE-5 and escapes
  `spec_kctrl`.** In `kctrl` (L2113-2134), when `dry_run==false` and the kernel
  PTE is absent, the code *creates* an identity-mapped entry with
  `frame_addr == vaddr` (`FrameAddress::new(... vaddr ...)`). (a) `spec_kctrl`
  (vmem.spec.rs L407) models only `kernel.insert(v, {perms, ..self.kernel[v]})`,
  i.e. it reads the *existing* `self.kernel[v]` frame and assumes the key is
  already present — it does **not** model inserting a brand-new identity
  mapping. So the real-run postcondition for the "absent PTE" branch is
  unspecified/incorrect. (b) For a high kernel/MMIO vaddr (e.g. `≥ user_end() =
  0xf000_0000`), `frame == vaddr` is far above `phys_mem_size() = 0x800_0000`,
  so `spec_is_physical_region(frame, page_size())` is **false** — inserting such
  a mapping violates TYPE-5. Either the View must special-case identity-mapped
  MMIO frames (relax TYPE-5 / widen `phys_mem_size`'s meaning) or `kctrl`'s
  contract must restrict to already-present kernel pages. Also note the dry-run
  branch reports success for an absent PTE (`Ok(false) => {}`) while the real run
  performs a *create*, so the two passes are not validating the same operation
  (couples with SB-3 / MOD-7).
- **SB-2 — `map` leaks an empty user page table on a late error.** In `map`
  (L485-498) a freshly allocated user page table is pushed to
  `user_page_tables` and mapped in `pgdir` *before* `page_table.map(...)` at
  L505. If that final `map` fails, the function returns `Err` (dropping
  `uframe`, good) but the empty page table is left allocated and PDE-mapped (the
  "NOTE: if we fail beyond this point we should unmap the page table" comment is
  not acted on). At View level `self@ == old@` still holds (empty page tables
  are not modeled), so the error spec is satisfiable, but it is a real physical
  resource leak that contradicts the MOD-4 "exactly balanced" intent. `map_kpage`
  has the same un-acted-on NOTE (L346-348).
- **SB-3 — `copy_to_user_unaligned_unchecked` dry run does not validate the
  destination, weakening the dry-run⇒commit contract.**
  - **SB-3a:** The destination-side checks — `find_user_frame(vaddr)` (is the
    dst page mapped?) and `is_physical_region(dst_phys_addr_raw, copy_size)` —
    are both inside `if !dry_run` (L1593-1614). The dry run only validates the
    *source* physical region. Therefore a successful dry run does **not**
    guarantee the real run won't `kpanic!` on an unmapped destination page or an
    out-of-range destination physical address. The documented contract ("dry run
    ⇒ a later real run cannot fail on validation", caller_analysis FN
    `copy_to_user_unaligned_unchecked`) and MOD-7 are only partially met. This
    also undermines `copy_to_user_unaligned`'s "all-or-nothing, never panics"
    claim (FN-27) when the destination is unmapped.
  - **SB-3b:** The real run calls `resolve_cow_for_region(dst, size)`
    (L1551) which mutates the View (`cow`/`write`/`frame` of dst pages), so the
    committed path does **not** satisfy `self@ == old@` as the scaffolding
    `ensures` claims (L1500-1502). The contract should state that dst CoW pages
    in range become non-CoW/writable (à la FN-15), not full preservation.

> SB-1 / SB-3 may be deliberate design choices (MMIO identity mapping; callers
> that guarantee the destination is pre-mapped and CoW-resolved). They are
> recorded here for the proving phase to investigate, since the current
> scaffolding specs do not match the observed code behavior.

---

## Explicitly Not Verified (with reason)

- **Byte-level memory contents** of `memset` and all copy paths. The View models
  structure, not a global physical-memory byte map; proving content equality
  needs a cross-`Vmem` physical-aliasing model orthogonal to every safety
  property callers depend on (view_design Rejected Alternative 1). Copy/`memset`
  are specified by address-validation + structure preservation only.
- **Global MMU "active"/CR3 state** (`load`'s activation effect). Belongs to a
  global HAL abstraction, not a per-`Vmem` View (Rejected Alternative 2). Only
  `self@ == old@` and `pgdir`-consistency are verified.
- **Concrete list ordering / LRU move-to-front** in `lookup_user_page_table` and
  iteration order of `for_each_user_mapping`. Implementation strategy; callers
  are told not to depend on it. Only functional results (coverage, lookup
  success) are verified.
- **`Rc` refcount exact values / `RefCell` borrow-state dynamics** beyond what
  MOD-4 ownership balance needs.

---

## Assumed External Specs

vstd was searched (`/mnt/toolchain/verus/vstd`) to confirm these gaps. Only
operations genuinely missing a vstd spec are listed; the specification phase
writes the `assume_specification` (these are std/`alloc` library calls, **not**
local code — no local function may be `external_body`).

- **`alloc::collections::LinkedList` — all used operations.** Searched
  `vstd/` and `vstd/std_specs/` for `LinkedList`: **no matches** (only `ExLinkedList`
  is declared in this crate's `vmem.spec.rs`, with no behavioral spec). Missing
  ops actually called: `LinkedList::new`, `pop_front`, `push_back`, `push_front`,
  `front`, `front_mut`, `len`, `iter`, `iter_mut`, `remove`, `position`,
  `cursor_front_mut`, and the `Cursor`/`CursorMut` ops `current`, `move_next`,
  `remove_current`. vstd models `VecDeque` (`std_specs/vecdeque.rs`) but not
  `LinkedList`, so a length/sequence-abstraction spec must be assumed.
- **`core::cell::RefCell` — `new`, `borrow`, `borrow_mut`.** Searched
  `vstd/std_specs/` for `RefCell`: **no matches** (vstd only specifies its own
  `PCell`/`cell` types in `vstd/cell.rs`, not std `RefCell`). `ExRefCell` is
  declared opaque here but has no behavioral spec; the `(PageTableAddress,
  PageTable)` payload accessed through `borrow()`/`borrow_mut()` needs an
  assumed spec.
- **`alloc::rc::Rc::clone` (`Clone for Rc`) and `Rc`-`Deref`.** `vstd/std_specs/
  smart_ptrs.rs` provides `Rc::new`, `Rc::try_unwrap`, `Rc::into_inner`, and
  `Rc::default` only — searched it for `clone`/`Deref`/`borrow`: **no match**.
  The heavy `entry.clone()` on `Rc<RefCell<…>>` and the deref to reach `.borrow()`
  lack a spec.

Operations that already have vstd specs and therefore do **not** need
`assume_specification` (confirmed present, listed to forestall re-adding them):
`usize::checked_add` / `checked_mul` (`std_specs/num.rs`),
`Ord::min` (`std_specs/cmp.rs:143`), `ManuallyDrop::new`/deref
(`std_specs/manually_drop.rs`).
