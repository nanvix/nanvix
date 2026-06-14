# View Design: `mm::virt::identity_map`

## Abstract Resource

To callers, this module is the **kernel's identity map**: a single global,
write-and-supervisor mapping that answers one question — *"is this physical page
reachable at its own physical address (V == P) in the kernel address space?"* —
and, when asked, makes it so without consuming frame-allocator memory.

It is a process-wide **singleton** (backed by the `KERNEL_PD_PADDR` / `KERNEL_CR3`
statics plus a BSS page-table pool). The in-scope functions are free functions,
not methods on an exec struct, so the View models the *global* identity-map state
rather than the contents of any one passed-in `Table`. How that global ghost
state is threaded (ghost global vs. tracked token) is a specification-phase
concern; this document fixes only the abstraction.

## View Struct

```rust
/// Abstract state of the kernel identity map.
///
/// This is a ghost model of global state — there is no owning exec struct.
pub struct IdentityMapView {
    /// Whether the lazy identity mapper has been initialized (i.e. `init` has
    /// published the kernel page directory). Before initialization the boot
    /// page tables are still active and every `identity_map_page` is a
    /// successful no-op, so this flag selects which transition applies.
    pub initialized: bool,

    /// The set of identity-mapped pages, each identified by its page-aligned
    /// physical base address. Membership means: the page is **present,
    /// writable, supervisor-only**, and reachable at its own physical address
    /// in the kernel address space. Permissions are uniform across all mapped
    /// pages, so they are encoded by membership rather than stored per page.
    ///
    /// Implementation note: realized as `Set<int>` (not `Set<usize>`) in
    /// `identity_map.spec.rs`, because `PhysicalAddress@` and
    /// `PageAligned<PhysicalAddress>@` both view as `int`. This keeps
    /// `mapped.contains(phys_addr@)` and `spec_page_base(..)` typechecking
    /// directly and matches `FrameAllocView`'s address element type.
    pub mapped: Set<int>,,
}
```

### Spec functions (signatures designed here; bodies are spec-phase work)

```rust
impl /* global identity map */ {
    /// Maps concrete (global) state to the abstract identity map.
    /// Public so callers can name the abstract state; closed so the mapping
    /// from `KERNEL_PD_PADDR`, the page directory, and the page-table pool
    /// does not leak.
    pub closed spec fn view(/* ghost global */) -> IdentityMapView;
}

impl IdentityMapView {
    /// Implementation-consistency invariant (e.g. the abstract `mapped` set
    /// agrees with the present PDE/PTE bits, page-table pool bounds, etc.).
    /// Cannot be written until implementation bodies are visible — left as a
    /// placeholder; the specification phase fills it in.
    pub closed spec fn internal_inv(self) -> bool {
        true
    }
}
```

## Well-formedness Invariant

```rust
impl IdentityMapView {
    pub open spec fn inv(self) -> bool {
        &&& self.internal_inv()
        // Every recorded page is identified by a page-aligned base address.
        &&& (forall|p: usize| self.mapped.contains(p) ==> is_page_aligned(p))
        // Before initialization no lazy mapping has been installed; coverage
        // is provided entirely by the boot page tables.
        &&& (!self.initialized ==> self.mapped =~= Set::empty())
    }
}
```

`is_page_aligned(p) := p % PAGE_SIZE == 0` is a free spec helper (constant
resolved in the spec phase). Both conjuncts are abstraction-level truths any
implementation maintains, not statements about a particular layout.

Monotonicity ("a mapped page is never unmapped or remapped") is a property of
*transitions*, so it lives in `ensures` (e.g. `old@.mapped.subset_of(self@.mapped)`),
not in `inv()`.

## Spec Transition / Query Functions

```rust
impl IdentityMapView {
    /// Page is reachable at its own physical address right now.
    /// Before init, the boot tables make every page reachable; after init,
    /// reachability is exactly membership in `mapped`. This is the headline
    /// resource callers obtain from `identity_map_page` on `Ok`.
    pub open spec fn accessible(self, page: usize) -> bool {
        !self.initialized || self.mapped.contains(page)
    }

    /// Unconditional install of one page (idempotent). Models the leaf step
    /// `ensure_pte` realizes: it always adds the page to the map.
    pub open spec fn spec_install_page(self, page: usize) -> IdentityMapView {
        IdentityMapView { mapped: self.mapped.insert(page), ..self }
    }

    /// Full effect of `identity_map_page`: install the page when the mapper is
    /// initialized, otherwise a no-op (boot tables already cover it). Either
    /// way the page ends up `accessible`.
    pub open spec fn spec_map_page(self, page: usize) -> IdentityMapView {
        if self.initialized {
            self.spec_install_page(page)
        } else {
            self
        }
    }
}
```

### How each in-scope function uses the View

| Function | View-level effect | Notes |
|----------|-------------------|-------|
| `identity_map_page(phys_addr)` | `self@ == old@.spec_map_page(base(phys_addr@))`; on `Ok`, `self@.accessible(base(phys_addr@))` | `base(a) := align_down(a, PAGE_SIZE)`; equals `a` for the page-aligned input. On `Err`, `self@ == old@` and the page need not be accessible. |
| `ensure_pte(pt, pte_idx, phys_addr)` | `self@ == old@.spec_install_page(base(phys_addr))` (idempotent) | The leaf step that realizes V==P; `identity_map_page` wraps it with the `initialized` guard. |
| `ensure_pt(pd, pde_idx)` | `self@.mapped == old@.mapped` (identity at the page abstraction) | Allocating/installing a page table establishes *internal structure* only — its PTEs start absent, so no page becomes mapped. Returns an opaque page-aligned `pt_paddr`. Idempotent: a present PDE leaves state unchanged. Also called by `init` to pre-allocate. |

The two private helpers deliberately have **no PDE/PTE-index fields in the
View**: the caller analysis states callers "don't care about the page-directory /
page-table index split." `ensure_pt` contributes nothing to `mapped`; only
`ensure_pte` does. Their composition inside `identity_map_page` is exactly
`spec_map_page`.

## Design Rationale

**`initialized: bool`** — The contract has a genuine pre-init mode: before `init`
publishes the kernel PD, `identity_map_page` succeeds as a no-op and the page is
reachable through boot tables. This flag is the minimal abstract state needed to
state that conditional transition (`spec_map_page`) and to make `accessible`
total. *Substitution test:* any lazy-mapper implementation has a "not yet set up,
boot tables cover memory" phase versus an "active" phase; the flag survives a
rewrite. It is never a stand-in for the concrete `KERNEL_PD_PADDR == 0` check —
callers observe a *mode*, not an atomic.

**`mapped: Set<usize>`** — The core resource: which physical pages are
identity-mapped (present + writable + supervisor). Callers reason purely in terms
of set membership ("is my frame reachable?"), and idempotence/monotonicity are
naturally `insert` / `subset_of` on a `Set`. *Substitution test:* every possible
implementation — two-level x86 tables, a flat array, a different paging scheme —
must track *which pages are mapped*; the set is implementation-agnostic. Elements
are page-aligned base addresses (`usize`) because callers already hold
`PageAligned<PhysicalAddress>` and the skill keeps addresses as `usize`; this
avoids introducing division/frame-number arithmetic at the abstraction surface.

**Permissions encoded by membership, not a field** — These functions only ever
create writable, supervisor mappings, so per-page permission state would be a
constant. Folding "writable + supervisor" into the meaning of `mapped`
membership keeps the View minimal while still letting `KernelFrame::clear` /
`memset` rely on writability.

**`accessible` as a derived query** — Gives callers a single predicate for the
postcondition they actually want ("the frame is reachable now") that is uniform
across the pre-init and post-init cases, so caller proofs don't branch on
`initialized`.

**`inv()` conjuncts** — `is_page_aligned` over `mapped` is the abstraction's
well-formedness (a "page" is a page-aligned unit). The `!initialized ==> mapped
empty` conjunct is the cross-field consistency that justifies `spec_map_page`'s
no-op branch (nothing is silently dropped when not initialized). Both are
caller-visible truths and hold for any implementation.

**Field usage (minimality check)** — `initialized` is used in `spec_map_page`,
`accessible`, and `inv`; `mapped` is used in every spec function. No field is
dead.

| Criterion | Result |
|-----------|--------|
| Substitution | ✅ both fields survive a full rewrite |
| Caller-only | ✅ no PDE/PTE indices, flag bits, BSS-pool, or CR3 mechanics leak |
| Complete | ✅ accessibility, idempotence, monotonicity, pre-init no-op all expressible |
| Minimal | ✅ two fields, both referenced by specs |
| No code-as-spec | ✅ describes *which pages are mapped*, not *how* |

## Rejected Alternatives

- **Two-level `Map<TableIndex, Map<TableIndex, usize>>` (PD → PT → frame).**
  Mirrors the x86 page-table structure and the `ensure_pt`/`ensure_pte` split.
  Rejected: the caller analysis explicitly says external callers don't care about
  the PD/PT index split or `Table::from_address` mechanics — this is an
  abstraction leak (Over-Faithful anti-pattern). The page-granular `Set` captures
  everything callers observe.

- **`mapped: Set<nat>` keyed by frame number (`addr / PAGE_SIZE`).** Equivalent
  power, but forces division semantics into the abstraction and diverges from the
  `PageAligned<PhysicalAddress>` (a `usize`) callers actually pass. Page-aligned
  `usize` base addresses are more directly usable in caller specs.

- **Per-page permission/flags field (e.g. `Map<usize, Perms>`).** All mappings
  are uniformly writable + supervisor, so this would store a constant. Encoded by
  `mapped` membership instead.

- **A `pt_pool_used: nat` / remaining-BSS-slots field.** Callers explicitly don't
  care about page-table-pool bookkeeping; `OutOfMemory` is observed only as an
  `Err`. Exhaustion is a failure outcome, not abstract state callers reason about.
  Belongs (if anywhere) inside `internal_inv()`, not the public View.

- **`kernel_pd_paddr: usize` / `cr3` fields.** These are concrete handles to the
  paging structures, not caller-observable abstract state. Callers never name the
  kernel PD address; replacing it (or the CR3 root) with a different mechanism
  must not change the spec. Leaking them fails the substitution test.

- **A separate per-`Table` View for `ensure_pt`/`ensure_pte`.** Tempting because
  they take a `Table` argument, but they are private sub-steps of a single global
  operation; giving them independent View types would fragment the abstraction and
  re-expose the index structure. They are specified as refinements of
  `spec_install_page` / the identity transition on the one global
  `IdentityMapView`.

- **No `initialized` flag (treat init as just another mapping).** Then
  `identity_map_page`'s pre-init no-op success could not be stated precisely, and
  `accessible` would wrongly require `mapped` membership during early boot. The
  flag is the minimal addition that captures the documented pre-init transparency.
