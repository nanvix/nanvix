# Cheating Elimination Report: virt-identity-map

## Scope

In-scope functions (the only functions for this module):
`identity_map_page`, `ensure_pt`, `ensure_pte` — all in
`src/kernel/src/mm/virt/identity_map.rs`.

Module-scoped gate: `make verify-kernel MODULE=mm::virt::identity_map`
(the cheating scan copies only `identity_map.rs`; `.spec.rs`/`.proof.rs` are not
scanned, and `external_type_specification` items are exempt).

## Cheating Counts (before → after)

Module-scoped (`mm::virt::identity_map`, what the gate measures):

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 3 | 3 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 1 | 0 | **1** |
| assume_specification | 0* | 0* | 0 |
| cfg-gated exec | 0 | 0 | 0 |

The stricter hard-cheating gate (`cheating_report_1.md`) scans `identity_map.spec.rs`
too and flagged the `#[verifier::external_body]` on `ExPageTableBss`
(`identity_map.spec.rs:142`). **Eliminated this pass:** removed the `external_body`
attribute, leaving `#[verifier::external_type_specification] pub struct
ExPageTableBss(PageTableBss);`. `PageTableBss` is a field-less unit struct (`pub struct
PageTableBss;`), so — like `ExError(crate::Error)` — the opaque-body attribute is
unnecessary; Verus accepts the bare external type spec. Whole-crate `external_body`
dropped 12 → 11, module verification unchanged (9 verified, 0 errors).

`*` The cheating scanner copies only `identity_map.rs`, so the two
`assume_specification`s and one `external_type_specification` in
`identity_map.spec.rs` are outside the scanned set (and `assume_specification`
is not one of the scanner's counted categories). They are pre-existing
external-bottom **dependency placeholders**, not introduced in this pass — see
"Out-of-scan spec items" below.

Whole-crate (unchanged baseline, for reference): `admit=31 external_body=12
cfg_gate=15` — all from other, out-of-scope `mm::virt`/`mm::phys` modules.

## Items Eliminated

- **`external_body` on `ExPageTableBss`** (`identity_map.spec.rs:142`) — **ELIMINATED**.
  Removed the `#[verifier::external_body]` attribute; the bare
  `#[verifier::external_type_specification] pub struct ExPageTableBss(PageTableBss);`
  verifies because `PageTableBss` is a field-less unit struct (no opaque interior to
  hide). Confirmed via `make verify-kernel` (whole-crate `external_body` 12 → 11, 0
  verification errors).

The three in-scope `admit()`s were attempted and are **genuinely blocked**
(see verification TODOs). Specs were **not** weakened, no `external_body` was
added, no `assume_specification`/`axiom` was added, and the exec code was left
byte-identical to the base branch.

## Why the three admits cannot be eliminated (escalation ladder, evidence-backed)

`identity_map_view()` is a **parameterless** `uninterp spec fn`
(`identity_map.spec.rs:36`): it denotes one fixed-but-opaque constant `V`, and
`requires` only gives `V.inv()`. Two of the functions assert **result-dependent,
opposite** facts over this constant:

- `ensure_pte`: `Ok => V.mapped.contains(p)` *and* `Err => !V.mapped.contains(p)`.
- `identity_map_page`: `Ok => V.accessible(p)` *and* `Err => !V.accessible(p)`.

For a single fixed unknown `V`, the exec return decision (PTE read, frame-number
range, `pd_paddr==0` init test) is not tied to that fixed set, so **both branches
cannot be discharged** — a logical impossibility, not a missing lemma. A constant
`open spec` redefinition (`initialized:false, mapped:empty`) satisfies one branch
but makes the opposite branch literally `false`, confirming the impossibility is
intrinsic.

Empirically reproduced this pass by deleting all three `admit()`s and running
`make verify-kernel MODULE=mm::virt::identity_map` → **6 verified, 3 errors**
(exit 101):
- `:534` precondition `pd.read` (`pde_idx@ < PAGE_TABLE_LENGTH`) — dischargeable.
- `:549` precondition `alloc_as` needs `bump_view(&PAGE_TABLE_ALLOCATOR).inv()` —
  `bump_view` is `uninterp` over a module `static`; **unprovable** without a token.
- `:618` postcondition `ensure_pte` `mapped.contains` — **unprovable** (constant view).
- `:631` precondition `pt.read` — dischargeable.
- `:706` postcondition `identity_map_page` `accessible` — **unprovable** (constant view).

Sound elimination would require making the view depend on runtime singleton state
via a ghost token, every realization of which is forbidden by the task's hard rules:
(1) `vstd` atomic-ghost / `PointsTo` → exec type/signature change to module statics
and out-of-scope callers (`init`, `boot_init`, `memset`/`memcpy`); (2) a trusted
`external_body` accessor (`mm::phys::frame::instance` pattern) → not in
`tcb-allowed.md`; (3) `assume_specification`/`axiom` on `AtomicUsize::load` /
`Table::write` → human-approved only, and `tcb-allowed.md` records the `Table::write`
contents postcondition as deliberately-deferred-because-unsound. `table.rs` and the
out-of-scope callers are also off-limits.

Searched `vstd` (atomics, layout, sets) and verified dependency specs
(`bump_allocator`, `arch::…::paging::table`): no existing spec links a parameterless
`uninterp` view to interior-mutable singleton state.

## Out-of-scan spec items (`identity_map.spec.rs`, pre-existing, not module-gated)

- `#[verifier::external_type_specification] struct ExPageTableBss(PageTableBss)` —
  the standard opaque-external-type idiom. The `#[verifier::external_body]` attribute
  it previously carried was **removed this pass** (see "Items Eliminated"): `PageTableBss`
  is a field-less unit struct, so no opaque body needs hiding and the bare external type
  spec verifies.
- `assume_specification <[T]>::as_ptr` — std slice accessor (no `vstd` spec; called
  by `slot.as_ptr()`); external-bottom placeholder.
- `assume_specification FixedSizeBumpAllocator::<N,A,S>::new` — the dependency
  constructor has no `#[verus_spec]` in `bump_allocator`; needed so the
  `PAGE_TABLE_ALLOCATOR` static initializer translates; external-bottom placeholder.

These are dependency contracts (external-bottom), superseded when their modules are
specced; removing them breaks translation of `identity_map.rs`. They were not added
in this pass and are out of the in-scope-function set.

## Verification TODOs (verus-ai-logs/nanvix-phys-virt-identity-map/verification_todo.md)

- `ensure_pt` (identity_map.rs:533): `alloc_as` precondition
  `bump_view(self).inv()` over a module `static` + fresh-alloc `as_ptr` alignment —
  need the deferred ghost-pointer/allocator token.
- `ensure_pte` (identity_map.rs:627): `Ok/Err` `mapped.contains` over the
  parameterless `uninterp` view — need the page-table permission token tying
  `pt.write` to `identity_map_view().mapped`.
- `identity_map_page` (identity_map.rs:718): `Ok/Err` `accessible` — need the same
  token plus a spec link from `pd_paddr==0` to `identity_map_view().initialized`.

## AST Consistency
- Zero mismatches confirmed: **YES**
  (`ast_consistency.py --base-ref verus-ai-prove-bottom-up identity_map.rs count`
  → "Consistent: 14 functions, 1 structs match"). The only diff vs the base branch
  is the 3 ghost `proof! { admit(); }` lines, which are stripped before hashing;
  exec AST is unchanged.

## Verification status
- `make verify-kernel MODULE=mm::virt::identity_map` → 9 verified, 0 errors (exit 0),
  status CHEATING_DETECTED (3 documented admits).
- `make verify` (full crate) → exit 0, 0 verification errors, no regression
  (global cheating counts unchanged from baseline).

## Result: BLOCKER

The 3 in-scope `admit()`s are a logically-impossible-to-discharge proof gap under
the parameterless-`uninterp`-view design, resolvable only via the deferred
proving-phase ghost-token framework whose every realization is forbidden by this
task's hard rules (no exec signature changes, no new `external_body` outside TCB, no
new `assume_specification`, no touching out-of-scope callers/`table.rs`). Honestly
handed off in `verification_todo.md`.
