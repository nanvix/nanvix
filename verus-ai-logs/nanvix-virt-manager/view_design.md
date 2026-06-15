# View Design: `mm::virt::manager` (`VirtMemoryManager`)

## Abstract Resource

To callers, `VirtMemoryManager` is a **stateless singleton service** that
mediates page-mapping operations on virtual address spaces (`Vmem`) and hands
out kernel-side pages/frames drawn from the global physical-frame pool. It is
obtained via `VirtMemoryManager::get_mut()` and threaded as `&mut self`, but
`struct VirtMemoryManager;` is **zero-sized**: it owns no fields and carries no
observable state of its own (caller analysis §"Manager statelessness").

Consequently the abstraction boundary for this module is **not** a field-bearing
manager View. Every observable effect lands on one of:

1. the **`Vmem` argument(s)** a function receives — modeled by the *inherited*
   `VmemView` already designed and implemented for the `mm::virt::vmem` module;
2. the **returned owned RAII values** (`KernelPage`, `Vec<KernelFrame>`,
   `Vec<UserFrame>`, the new `Vmem` from `new_vmem`) — modeled by their own
   type-level views; and
3. the **global physical-frame pool** behind `PhysMemoryManager` — a *global*
   resource, deliberately left **outside** any single View (parallel to the way
   `VmemView` excludes the global CR3 register and global physical memory; see
   Rejected Alternatives).

So this phase delivers three things: (a) a **unit** manager View, (b) a
re-evaluation of the **inherited `VmemView`** against the manager's callers, and
(c) the **manager-level spec vocabulary** (pure architectural predicates +
composite transitions over `VmemView`) that the manager's contracts will speak.

---

## View Struct

The manager holds no state, so its View is a marker (unit) type. The
substitution test forces this: a field-bearing manager View could only mirror
the global `PhysMemoryManager` or a registry of live `Vmem`s — neither of which
the manager *owns* or a caller *observes through `&mut self`*.

```rust
verus! {

/// Abstract state of the (stateless) virtual-memory manager singleton.
///
/// `VirtMemoryManager` is zero-sized: it owns no mappings, no frames, and no
/// page tables. All abstract state a caller reasons about lives in the `Vmem`
/// argument(s) (`VmemView`) and in the returned owned values. This marker view
/// therefore carries no fields — it exists only so callers can write `self@`
/// and so the manager satisfies the module's View/`inv()` contract.
pub struct VirtMemoryManagerView;

} // verus!
```

Spec functions to be implemented in `manager.spec.rs` (signatures + intent
designed here; bodies reference impl state and are filled in the specification
phase):

- `view(&self) -> VirtMemoryManagerView` — `pub closed spec fn`. Maps the
  zero-sized exec singleton to the unit View. Closed by convention; there is no
  internal field mapping to leak.
- `internal_inv(&self) -> bool` — `pub closed spec fn`, placeholder `true`. The
  manager has no internal exec fields, so there is no implementation-consistency
  obligation to encode. (The "manager is initialized" property guarded by
  `MEMORY_MANAGER_INIT` is established by `init`/`get_mut`, both out of scope,
  and is not a precondition of the in-scope methods — they already hold a valid
  `&mut self`.)
- `inv(&self) -> bool` — `pub open spec fn`, `self.internal_inv()` (i.e. `true`).
  There is no caller-visible manager-level well-formedness; the well-formedness
  that matters is per-`Vmem` (`vmem.inv() == vmem@.inv()`) and is asserted on the
  `Vmem` arguments in each method's `requires`/`ensures`, **not** on `self`.

Because `self@` is unit, every method trivially satisfies `self@ == old(self)@`;
that clause is therefore omitted from the manager's contracts (it carries no
information). The manager View appears in specs only to satisfy the module
convention.

---

## Inherited View: `VmemView` (re-evaluated against manager callers)

The central abstraction the manager's specs reference is the upstream
`VmemView` (from `mm::virt::vmem`, already implemented in `vmem.spec.rs`):

```rust
pub struct PagePerms   { pub read: bool, pub write: bool, pub exec: bool }
pub struct UserPageView { pub frame: nat, pub perms: PagePerms, pub cow: bool }
pub struct KernelPageView { pub frame: nat, pub perms: PagePerms }
pub struct VmemView {
    pub user:   Map<nat, UserPageView>,   // present user pages: vaddr -> mapping
    pub kernel: Map<nat, KernelPageView>, // present kernel pages: vaddr -> mapping
    pub pgdir:  nat,                       // page-directory physical base (CR3)
}
```

with the free architectural predicates (`spec_is_user_addr`,
`spec_is_user_region`, `spec_is_physical_region`, `is_page_aligned`,
`page_base`, `page_offset`, `PAGE_SIZE`, `USER_BASE`, `USER_END`) and the
permission map `AccessPermission::perms_view() -> PagePerms`.

**Substitution-test re-evaluation of each inherited field, from each manager
caller's perspective** (the skill requires checking the inherited View against
*all* callers, not just the original one):

| Inherited field | Manager callers that observe it | Survives substitution? |
|---|---|---|
| `user: Map<nat, UserPageView>` | `alloc_upages` (insert a contiguous run), `try_unmap_upage` (remove), `ctrl_upage` (perms), `link_user_pages` (copy parent→child as CoW), `try_resolve_cow_fault` (privatize one page), `load_elf` (grow), `new_vmem` (child starts empty) | ✅ every address space maps user vaddrs→frames regardless of page-table representation |
| `UserPageView.frame` | CoW sharing in `link_user_pages` (child frame == parent frame), privatization in `try_resolve_cow_fault` (new frame ≠ old), released frame on `try_unmap_upage` | ✅ a mapping is backed by *some* physical frame; the walk mechanism is irrelevant |
| `UserPageView.perms` | `alloc_upages`/`ctrl_upage` set `access.perms_view()`; the CoW invariant hinges on `write` | ✅ permissions are architectural, not layout-specific |
| `UserPageView.cow` | toggled by `link_user_pages` (mark) and `try_resolve_cow_fault` (clear); decides whether a write faults | ✅ CoW is a semantic state every implementation must track |
| `kernel: Map<…>` | `new_vmem` shares the parent's kernel half into the child; otherwise read-only here | ✅ kernel↦frame mapping exists in any design |
| `pgdir: nat` | `new_vmem` guarantees the child's base ≠ parent's | ✅ every address space has a root with a physical base |

**Verdict:** the inherited `VmemView` is **complete and sound for all manager
callers** — no field is renamed, removed, or added. It already exposes exactly
the user/kernel mapping structure, frames, permissions, CoW marks, and
page-directory base that the nine manager entry points need. What the manager
phase *adds* is not new View state but new **vocabulary** (below) that composes
the existing `VmemView` transitions into the larger, range- and fork-shaped
operations the manager performs.

---

## Manager-level Spec Vocabulary

These are the abstraction the manager contributes. They are **free
`pub open spec fn`s in `manager.spec.rs`** over the inherited `VmemView` and
architectural types — *not* extra `pub spec fn`s on `impl VirtMemoryManager`
(the exec type keeps only `inv`/`view`), and not edits to `VmemView` (owned by
the vmem module). They are the building blocks for the `requires`/`ensures` of
the in-scope functions.

### Pure architectural predicates (no `&self`, instance-independent)

```rust
verus! {

/// A page fault qualifies as a copy-on-write write fault: a user-mode write to
/// a present page. Pure decode of the x86 page-fault error-code bits
/// (`is_present`/`is_write`/`is_user` are pure `const fn`s in arch::cpu::excp).
/// Whether the *page* is actually CoW is then read from `vmem@` separately.
pub open spec fn spec_is_cow_write_fault(ec: ErrorCode) -> bool {
    ec.is_present() && ec.is_write() && ec.is_user()
}

/// The `n` page-aligned vaddrs `base, base+PAGE_SIZE, …, base+(n-1)*PAGE_SIZE`.
pub open spec fn page_run(base: nat, n: nat) -> Set<nat> {
    Set::new(|v: nat| exists|i: nat| i < n && v == base + i * PAGE_SIZE)
}

} // verus!
```

### Range / fork predicates over `VmemView`

```rust
verus! {

/// Precondition for `alloc_upages`: the whole run `[base, base+n*PAGE_SIZE)`
/// lies in user space and is currently unmapped (else BadAddress/ResourceBusy).
pub open spec fn region_user_unmapped(s: VmemView, base: nat, n: nat) -> bool {
    spec_is_user_region(base, n * PAGE_SIZE)
    && forall|v: nat| #[trigger] page_run(base, n).contains(v)
            ==> !s.user.contains_key(v)
}

/// Success shape of `alloc_upages`: exactly the run was added, each page mapped
/// with `perms` and not CoW; everything outside the run is untouched. Frame
/// choice is an allocator detail and stays existential (not pinned here).
pub open spec fn maps_user_run_with(
    old: VmemView, new: VmemView, base: nat, n: nat, perms: PagePerms,
) -> bool {
    // domain grew by exactly the run
    &&& new.user.dom() == old.user.dom().union(page_run(base, n))
    // pages outside the run are bit-for-bit preserved
    &&& forall|v: nat| #[trigger] old.user.contains_key(v)
            ==> new.user.contains_key(v) && new.user[v] == old.user[v]
    // each new page has the requested perms and is private (not CoW)
    &&& forall|v: nat| #[trigger] page_run(base, n).contains(v) ==> {
            &&& new.user.contains_key(v)
            &&& new.user[v].perms == perms
            &&& !new.user[v].cow
        }
    // kernel half and page directory are unchanged
    &&& new.kernel == old.kernel && new.pgdir == old.pgdir
}

/// Success shape of `link_user_pages` on the (parent, child) pair. Every present
/// user page of the entry parent is shared into the child at the same vaddr and
/// frame; logically-writable pages become CoW in BOTH parent and child;
/// genuinely read-only pages are shared read-only (not CoW). The child gains
/// exactly the parent's user domain (it had none overlapping on entry — see
/// `link_user_pages_pre`). Kernel halves and page directories are unchanged, and
/// the parent's user frames/domain are preserved (only its CoW marks may flip).
pub open spec fn links_child_cow(
    p_old: VmemView, p_new: VmemView, c_old: VmemView, c_new: VmemView,
) -> bool {
    // parent domain & frames preserved; perms unchanged except CoW marking
    &&& p_new.user.dom() == p_old.user.dom()
    &&& forall|v: nat| #[trigger] p_old.user.contains_key(v) ==> {
            let o = p_old.user[v]; let nw = p_new.user[v];
            &&& nw.frame == o.frame
            &&& (logically_writable(o) ==> nw.cow)          // marked CoW (inv ⟹ !nw.perms.write)
            &&& (!logically_writable(o) ==> nw == o)        // genuinely RO: untouched
        }
    // child gains exactly the parent's user pages, same frames, CoW iff writable
    &&& c_new.user.dom() == c_old.user.dom().union(p_old.user.dom())
    &&& forall|v: nat| #[trigger] p_old.user.contains_key(v) ==> {
            let o = p_old.user[v]; let cv = c_new.user[v];
            &&& cv.frame == o.frame
            &&& cv.cow == logically_writable(o)
        }
    // pre-existing, non-overlapping child pages are preserved
    &&& forall|v: nat| #[trigger] c_old.user.contains_key(v) && !p_old.user.contains_key(v)
            ==> c_new.user.contains_key(v) && c_new.user[v] == c_old.user[v]
    // kernel halves and page directories unchanged on both
    &&& p_new.kernel == p_old.kernel && p_new.pgdir == p_old.pgdir
    &&& c_new.kernel == c_old.kernel && c_new.pgdir == c_old.pgdir
}

/// A mapping is *logically writable* (CoW-eligible at fork time): writable in
/// hardware OR already CoW (left read-only with the CoW bit set by an earlier
/// fork). Mirrors the docstring of `link_user_pages`.
pub open spec fn logically_writable(p: UserPageView) -> bool {
    p.perms.write || p.cow
}

/// Caller precondition for `link_user_pages`: the child has no user mappings
/// overlapping the parent's (relied on by the rollback heuristic).
pub open spec fn link_user_pages_pre(parent: VmemView, child: VmemView) -> bool {
    forall|v: nat| #[trigger] parent.user.contains_key(v) ==> !child.user.contains_key(v)
}

} // verus!
```

> When a logically-writable parent page is marked CoW, `nw.cow` holds and the
> inherited `VmemView::inv()` (`cow ==> !perms.write`) forces its hardware-write
> bit off — so the write→read-only rewrite is implied, not restated. The
> specification phase may pin the exact parent rewrite using the inherited
> `spec_mark_cow` transition; the caller-visible facts (child shares the same
> frame at the same vaddr; logically-writable pages are CoW in both) are fully
> captured above.

For the single-page operations the manager already has exact inherited
transitions and needs **no new** composite:

| Manager op | Inherited `VmemView` transition reused |
|---|---|
| `try_unmap_upage` (true) | `old@.spec_unmap(v)` |
| `ctrl_upage` | `old@.spec_uctrl(v, access.perms_view())` |
| `try_resolve_cow_fault` (true) | `exists f. old@.spec_resolve_cow(v, f)` |
| `new_vmem` child kernel/user/pgdir | direct field reads (`kernel`, `user.is_empty()`, `pgdir`) |

---

## Spec sketches per top-level entry point

(Contracts are written in the specification phase; these sketches prove every
piece of vocabulary above is exercised and that `VmemView` suffices.)

- **`new_vmem(&self, vmem: &Vmem) -> Result<Vmem, Error>`** — `requires
  vmem.inv()`. `Ok(new)` ⟹ `new.inv()` ∧ `new@.kernel == vmem@.kernel` ∧
  `new@.user == Map::empty()` ∧ `new@.pgdir != vmem@.pgdir`. `vmem` is `&`, so
  `vmem@` unchanged. `Err` ⟹ no new space; `vmem@` unchanged.

- **`link_user_pages(&mut self, parent, child) -> Result<(), Error>`** —
  `requires parent.inv() && child.inv() && link_user_pages_pre(parent@, child@)`.
  `Ok(())` ⟹ `links_child_cow(old(parent)@, parent@, old(child)@, child@)` ∧
  `parent.inv() && child.inv()`. `Err` ⟹ **full rollback**:
  `parent@ == old(parent)@ && child@ == old(child)@`.

- **`try_resolve_cow_fault(&mut self, vmem, fault_addr, error_code)
  -> Result<bool, Error>`** — `requires vmem.inv()`; let
  `v = page_base(fault_addr)`. `Ok(true)` ⟹ `spec_is_cow_write_fault(error_code)`
  ∧ `old(vmem)@.user.contains_key(v)` ∧ `old(vmem)@.user[v].cow` ∧
  `exists f. spec_is_physical_region(f, PAGE_SIZE) && vmem@ == old(vmem)@.spec_resolve_cow(v, f)`.
  `Ok(false)` ⟹ `(!spec_is_cow_write_fault(error_code) || !spec_is_user_addr(v)
  || !old(vmem)@.user.contains_key(v) || !old(vmem)@.user[v].cow)` ∧
  `vmem@ == old(vmem)@`. `Err` ⟹ it *was* a CoW fault but resolution failed;
  `vmem@ == old(vmem)@`. (Note: invalid/non-user addresses yield `Ok(false)`,
  never `Err` — caller analysis §`try_resolve_cow_fault`.)

- **`try_unmap_upage(&mut self, vmem, vaddr) -> Result<bool, Error>`** —
  `requires vmem.inv()`. `Ok(true)` ⟹ `old(vmem)@.user.contains_key(vaddr@)` ∧
  `vmem@ == old(vmem)@.spec_unmap(vaddr@)`. `Ok(false)` ⟹
  `!old(vmem)@.user.contains_key(vaddr@)` ∧ `vmem@ == old(vmem)@` (benign,
  idempotent). `Err` ⟹ best-effort; abstract state may be partially changed but
  `vmem.inv()` holds.

- **`alloc_upages(&mut self, vmem, vaddr, access, clear, nframes, uframes)
  -> Result<(), Error>`** — `requires vmem.inv() && uframes@.len() == 0 &&
  uframes.capacity() >= nframes`. (Buffer-contract violation ⟹ `InvalidArgument`,
  no side effects.) `Ok(())` ⟹
  `maps_user_run_with(old(vmem)@, vmem@, vaddr@, nframes as nat, access.perms_view())`
  ∧ `vmem.inv()` ∧ `uframes@.len() == 0`. `Err` ⟹ **complete rollback**:
  `vmem@ == old(vmem)@` ∧ `uframes@.len() == 0`. (Precondition
  `region_user_unmapped(old(vmem)@, vaddr@, nframes)` is the static
  characterization; the runtime `BadAddress`/`ResourceBusy` checks discharge the
  dynamic part — see `spec-design` "static vs dynamic".)

- **`ctrl_upage(&mut self, vmem, vaddr, access) -> Result<(), Error>`** —
  `requires vmem.inv() && vmem@.user.contains_key(vaddr@)` (page already mapped;
  it does not allocate). `Ok(())` ⟹
  `vmem@ == old(vmem)@.spec_uctrl(vaddr@, access.perms_view())`. `Err` ⟹
  `vmem@ == old(vmem)@`.

- **`alloc_kpage(&mut self, clear) -> Result<KernelPage, Error>`** — no `Vmem`.
  `Ok(kpage)` ⟹ `kpage` owns a valid, page-aligned physical frame
  (`spec_is_physical_region(kpage@.frame, PAGE_SIZE)`; cleared if `clear`),
  released on `Drop`. `Err` ⟹ nothing allocated. Manager View unaffected.
  (Frame validity is a property of `KernelPage`'s own type view; the global pool
  is external — see Rejected Alternatives.)

- **`alloc_kpages(&mut self, clear, count, kframes) -> Result<(), Error>`** —
  `requires kframes@.len() == 0 && kframes.capacity() >= count`. `Ok(())` ⟹
  `kframes@.len() == count` ∧ each entry a valid page-aligned physical frame
  (cleared if `clear`). `Err` ⟹ `kframes@.len() == 0` (allocated frames
  dropped/freed; no leak).

- **`load_elf(&mut self, vmem, elf) -> Result<(VirtualAddress,
  PageAligned<VirtualAddress>), Error>`** — `requires vmem.inv()`.
  `Ok((entry, args_vaddr))` ⟹ `vmem.inv()` ∧ user domain grew
  (`old(vmem)@.user.dom().subset_of(vmem@.user.dom())`) ∧ kernel half & pgdir
  unchanged ∧ `spec_is_user_addr(entry@)` ∧ `spec_is_user_addr(args_vaddr@)` ∧
  `is_page_aligned(args_vaddr@)`. `Err` ⟹ **no rollback guarantee** — partial
  mappings inside `vmem` are acceptable because the caller discards the whole
  `vmem` (`?`); we still guarantee `vmem.inv()`. (Per-segment layout is an ELF
  detail the caller does not depend on — caller analysis §`load_elf`.)

---

## Design Rationale

**Why a unit manager View (substitution test on the *whole* View).** Rewriting
`VirtMemoryManager` with a completely different allocation algorithm changes
nothing observable through `&mut self`, because the type is zero-sized and every
effect is routed to a `Vmem` argument, a returned value, or the global pool. A
field-bearing manager View would therefore mirror state the manager does not
own — failing the substitution test by construction. The caller analysis reaches
the same conclusion: "the View must model the address space / frame ownership,
not the (empty) manager."

**Why reuse `VmemView` unchanged.** The manager is a thin orchestration layer
over `Vmem`'s primitives (`map`, `unmap`, `uctrl`, `mark_cow`, `resolve_cow`,
`clone`, …). Its callers observe address-space *effects*, which `VmemView`
already abstracts exactly (mapping structure + frames + perms + CoW + pgdir).
Re-checking each field against all nine entry points (table above) found no gap
and no leak, so the honest action is **keep all, rename none, add none**.

**Why the new vocabulary lives as free spec fns over `VmemView`.** The manager
performs *range* (`alloc_upages`) and *fork* (`link_user_pages`) operations that
have no single-page inherited transition. Encoding their success/precondition
shapes as named predicates (`maps_user_run_with`, `region_user_unmapped`,
`links_child_cow`, `link_user_pages_pre`, `logically_writable`) keeps each
contract declarative and lets the spec phase reuse the same names in `Ok`
clauses and rollback (`Err ⟹ old@ == new@`) reasoning. They take `VmemView` by
value (not `&self` on the manager), and are defined in `manager.spec.rs` so the
vmem module is not modified — respecting the skill's "no extra pub spec fns on
`impl MyType`" rule for the exec manager type.

**Why `spec_is_cow_write_fault` is a free predicate.** Classifying an
`ErrorCode` (present ∧ write ∧ user) is a pure function of the architectural
error-code bits — instance-independent, no `&self` — exactly like the inherited
`spec_is_user_addr` family. It belongs as a free spec fn, not as View state.

**Frames/RAII are observed through domain membership, not refcounts.** As in the
vmem design, ownership balance (no leak / no double-free) is enforced by Rust's
type system (`UserFrame`/`KernelFrame`/`KernelPage` by-value transfer + `Drop`)
and reflected abstractly by `user`-domain changes and by `kframes@.len()` /
returned-value validity. No refcount field is introduced.

---

## Rejected Alternatives

1. **A field-bearing manager View mirroring `PhysMemoryManager`** (e.g.
   `free_frames: Set<nat>`, `refcounts: Map<nat,nat>`). *Rejected.* The pool is
   *global* mutable state reached via `PhysMemoryManager::get_mut()`, not owned
   by the manager nor reachable through its `&mut self`. Threading a ghost pool
   through a zero-sized `&mut self` models fiction and couples the View to the
   allocator's `Rc`/free-list strategy — failing the substitution test. It also
   provides no fact any *caller* observes: `alloc_kpage` callers see only the
   returned page and `Ok`/`Err`. Pool depletion is a liveness property handled
   (if ever) by a separate global frame-pool abstraction. This mirrors
   `VmemView` rejecting a global CR3 field and global physical-memory contents.

2. **Modeling kernel-frame allocation as a manager-View transition.**
   *Rejected.* `alloc_kpage`/`alloc_kpages` touch no `Vmem` and produce owned
   RAII values whose validity (page-aligned, within `PHYS_MEM_SIZE`, cleared)
   is a property of `KernelPage`/`KernelFrame`'s **own** type views. Pinning
   *which* frame is chosen would over-specify an allocator detail callers
   explicitly do not care about (caller analysis §`alloc_kpage`).

3. **A registry of live `Vmem`s inside the manager View** (e.g.
   `spaces: Map<id, VmemView>`). *Rejected.* The manager never holds a
   collection of address spaces; callers pass exactly the `Vmem`(s) they own
   into each call. The relevant state is the argument's `VmemView`, asserted in
   `requires`/`ensures`, not a manager-held set. A registry would duplicate
   caller-owned state and break View consistency.

4. **Encoding "manager initialized" (`MEMORY_MANAGER_INIT`) in `inv()`.**
   *Rejected for the in-scope methods.* Initialization is established by `init`
   and checked by `get_mut` (both out of scope). Every in-scope method receives
   an already-valid `&mut self`; none re-checks the flag, so the property is not
   a caller obligation here and would be a non-load-bearing clause. It stays an
   `init`/`get_mut` concern.

5. **Pinning `link_user_pages`' parent CoW transition with exact
   `spec_mark_cow` composition in *this* phase.** *Deferred, not rejected.* The
   caller-visible facts — child shares the parent's frame at the same vaddr, and
   every logically-writable page is CoW in both parent and child — are captured
   now by `links_child_cow`. The precise parent-permission rewrite (writable →
   read-only + CoW) is best expressed using the inherited `spec_mark_cow`
   transition during specification, when impl bodies confirm the exact ordering;
   designing it speculatively now risks over-constraining.

6. **Byte-level memory contents / data-copy effects** (e.g. `clear` zeroing,
   `load_elf` segment bytes, CoW copy). *Rejected*, identical to the vmem View's
   reasoning: the in-scope obligations are *structural/safety* (valid ranges,
   all-or-nothing rollback, CoW pre-resolution, no-leak). `clear` and segment
   contents are specified as content-only; modeling bytes would require a global
   physical-memory model orthogonal to every property the callers depend on.
   Deferrable to a future content-aware View.
