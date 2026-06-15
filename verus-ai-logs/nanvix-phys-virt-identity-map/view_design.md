# View Design: mm::virt::identity_map

## Abstract Resource

To callers, this module is the **kernel lazy identity map**: a partial,
monotonically-growing set of physical pages that are reachable (identity-mapped)
through the kernel address space, plus a one-shot "is the mapper live yet?"
status. The only externally meaningful operation in scope, `identity_map_page`,
is an *idempotent side effect* — "make sure this physical page is reachable in
the kernel address space" — not a query returning data.

`ensure_pt` and `ensure_pte` are private sub-steps of that operation. Their
page-table / physical-address details are deliberately *not* part of this View
(callers explicitly do not observe them); they are specified at the page-table
structural level over their `Table` argument's own view, and connect back to the
module View only through the `mapped` set (see "Spec Transition Functions").

---

## View Struct

```rust
/// Abstract state of the kernel lazy identity map.
///
/// This is the caller-visible state behind the free functions in
/// `mm::virt::identity_map` (which operate over module-global statics, not a
/// `self` receiver). `identity_map_page` is specified as a transition on this
/// View; the private helpers refine it.
pub struct IdentityMapView {
    /// Whether `init` has published the kernel page directory.
    ///
    /// Before this is `true` (boot page tables still active), every
    /// `identity_map_page` call is a no-op success that leaves `mapped`
    /// unchanged. Callers are required to tolerate this pre-init no-op.
    pub initialized: bool,

    /// Page frame numbers currently reachable through the kernel identity map
    /// (the page's PTE is present). `frame == phys_addr / PAGE_SIZE`.
    ///
    /// A page-aligned physical address `p` is reachable iff
    /// `mapped.contains(p / PAGE_SIZE)`. The set only ever grows (mappings are
    /// never torn down by the in-scope functions).
    pub mapped: Set<nat>,
}
```

Frame numbers (not byte addresses) are used so the set carries no implicit
page-alignment obligation; the `maps()` helper recovers the address-level query.

---

## Well-formedness Invariant

```rust
impl IdentityMapView {
    /// Largest-valid-frame bound: a frame number is addressable iff it is
    /// strictly below this. Corresponds to the hardware `FrameNumber` range
    /// whose violation `ensure_pte`/`identity_map_page` report as
    /// `ErrorCode::BadAddress`. (= physical-address-space size / PAGE_SIZE.)
    pub open spec fn max_frames() -> nat;

    pub open spec fn inv(self) -> bool {
        // Every reachable page denotes a valid physical frame. No partially
        // installed / out-of-range frame is ever recorded as mapped, mirroring
        // the all-or-nothing failure guarantee.
        forall|f: nat| #[trigger] self.mapped.contains(f) ==> f < Self::max_frames()
    }
}
```

`inv()` is intentionally weak: pre-allocation of PDEs over `[0, MEMORY_SIZE)`
after `init` is a property of the *kernel page directory's* structural state, not
of the page-reachability set, so it is asserted in the helper specs over the
`Table` view rather than baked into this View's invariant.

---

## Spec Helpers (on the View, reusable in specs)

```rust
impl IdentityMapView {
    /// Address-level reachability query: is the page covering `phys_addr`
    /// identity-mapped in the kernel address space?
    pub open spec fn maps(self, phys_addr: int) -> bool {
        self.mapped.contains((phys_addr / PAGE_SIZE) as nat)
    }
}
```

---

## Spec Transition Functions

### `identity_map_page` (external contract — the one callers see)

```rust
impl IdentityMapView {
    /// Effect of identity-mapping the page with frame number `frame`.
    /// - Pre-init: no-op (callers must tolerate this).
    /// - Live: the frame becomes reachable. `Set::insert` is idempotent, so
    ///   re-mapping an already-mapped page is automatically a no-op success
    ///   (captures the "idempotent" caller expectation with no special case).
    pub open spec fn spec_identity_map_page(self, frame: nat) -> IdentityMapView {
        if self.initialized {
            IdentityMapView { mapped: self.mapped.insert(frame), ..self }
        } else {
            self
        }
    }
}
```

Intended use on the exec function (`phys_addr: PageAligned<PhysicalAddress>`,
let `frame = phys_addr@ / PAGE_SIZE`):

- **Success** ⇒ `self@ == old(self)@.spec_identity_map_page(frame)` and
  `self@.maps(phys_addr@)` (map-on-success: the covering page is reachable).
- **Failure** ⇒ `self@ == old(self)@` (all-or-nothing: nothing is installed or
  consumed; `initialized` unchanged).
- `initialized` is never flipped by this function (frame condition: it only
  reads the global state).

### `ensure_pte` (private sub-step — refines the `mapped` set)

`ensure_pte(pt, pte_idx, phys_addr)` is the step that actually makes a page
reachable. Its post-state is expressed at the page-table level over its `pt`
argument's own view, and corresponds to inserting `phys_addr / PAGE_SIZE` into
`mapped`:

- **Success** ⇒ `pt@[pte_idx]` is present and identity-maps `phys_addr`
  (`frame == phys_addr / PAGE_SIZE`); idempotent if already present. This is the
  page-table witness for `self@.maps(phys_addr)` after `identity_map_page`.
- **Failure** (`InvalidArgument` bad read / `BadAddress` frame out of range,
  i.e. `frame >= max_frames()`) ⇒ `pt@` unchanged (no PTE written).

### `ensure_pt` (private sub-step — structural precondition, *not* in `mapped`)

`ensure_pt(pd, pde_idx) -> usize` ensures a page table *exists* for `pde_idx`.
This is below the page-reachability abstraction and does not change `mapped`; it
is specified purely over the `pd` argument's `Table` view plus its return value:

- **Success** ⇒ `pd@[pde_idx]` is present afterwards, and the returned `usize` is
  the physical base of that page table (freshly allocated ⇒ all PTEs absent /
  zeroed, or the pre-existing one). Idempotent: a present PDE is returned without
  allocation.
- **Failure** (`InvalidArgument` / `OutOfMemory` no BSS slot / `BadAddress` PT
  frame out of range) ⇒ no PDE installed; `pd@` unchanged.

The returned physical address is internal plumbing (fed to `Table::from_address`
by `identity_map_page`); it never appears in any caller-facing clause, so it is
not modeled in the View.

---

## Design Rationale (per field, with substitution test)

| Field | Why it's needed | Substitution test |
|-------|-----------------|-------------------|
| `mapped: Set<nat>` | The whole point of the module: "which physical pages are reachable in kernel space." Used by `maps()`, `spec_identity_map_page`, and the map-on-success / idempotence ensures. | ✅ Any reimplementation (different paging scheme, different lazy strategy, eager mapping) still maintains *some* set of reachable pages. The concept is the resource itself, not a strategy. |
| `initialized: bool` | Callers are explicitly required to tolerate the pre-init no-op; the function's observable behavior differs across this boundary, so specs must name it. | ✅ Borderline but justified: the boot architecture (boot page tables → published kernel PD) is fundamental to the kernel, and callers *depend* on the no-op-before-ready behavior. Any rewrite still has a "mapper not live yet ⇒ no-op" phase. Modeled as an abstract `bool`, not the underlying `KERNEL_PD_PADDR == 0` static. |

Both fields use mathematical types (`Set`, `bool`, `nat`) and live entirely in
spec world; no `usize`, pointer, or flag-bit detail leaks in.

---

## Rejected Alternatives

1. **`kernel_pd_paddr: int`, `kernel_cr3: int` fields** (mirroring the statics).
   Rejected: implementation-specific physical addresses. Callers never observe
   them; only the *fact* of initialization matters. Collapsed into the single
   `initialized: bool`. Fails the substitution test (a rewrite might not even use
   CR3 / a single PD root).

2. **`pde_present: Set<nat>` / `page_tables: Map<nat, int>`** (per-PDE page-table
   allocation, base addresses). Rejected for the module View: the caller analysis
   states callers explicitly *don't care* whether a new page table was allocated
   vs reused, nor any page-table physical address. PDE presence is a structural
   fact of the kernel PD, specified locally over `ensure_pt`'s `Table` argument
   view, not promoted into the abstraction boundary. Fails substitution
   (page-directory/page-table structure is an x86 hardware strategy detail).

3. **Return-address modeling for `ensure_pt`** (e.g., a field recording each
   PDE's PT base). Rejected: the `usize` is pure internal plumbing that "never
   surfaces to callers." Modeling it would be code-as-spec.

4. **`mapped: Set<int>` of byte / page-aligned addresses** instead of frame
   numbers. Rejected: encoding page-alignment into set membership invites
   redundant alignment side-conditions in every clause. Frame numbers are the
   canonical page identity; the `maps(phys_addr)` helper recovers address-level
   queries when needed.

5. **TLB-shootdown / TLB-validity state.** Rejected: not caller-observable; TLB
   invalidation is an internal correctness obligation of `ensure_pte`, not part
   of the page-reachability resource. A rewrite could batch or defer
   invalidation without changing the abstract contract.

6. **PTE/PDE flag-bit fields (present, RW, user/supervisor, etc.).** Rejected:
   callers explicitly don't care about exact flag bits. "Reachable" (present +
   identity) is the only abstract property; the concrete flags are HOW, not WHAT.

7. **A "monotone / append-only" history field.** Rejected: monotonicity (the
   in-scope functions never unmap) is a *property to prove across a transition*
   (`old(self)@.mapped.subset_of(self@.mapped)`), expressible from the existing
   `mapped` field, not a separate piece of state.
