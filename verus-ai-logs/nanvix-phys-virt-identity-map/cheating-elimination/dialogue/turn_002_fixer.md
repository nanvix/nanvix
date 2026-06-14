## Response to Turn 2 / cheating_report_1 (external_body: 1, admit: 3)

### What I Changed

**ELIMINATED the external_body** — `src/kernel/src/mm/virt/identity_map.spec.rs:142`.

Before:
```rust
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageTableBss(PageTableBss);
```
After:
```rust
#[verifier::external_type_specification]
pub struct ExPageTableBss(PageTableBss);
```
`PageTableBss` is a field-less unit struct (`pub struct PageTableBss;` in
`page_table_allocator.rs`), so — exactly like `ExError(crate::Error)` /
`ExErrorCode(...)` in `error/lib.rs` — it has no opaque interior to hide and the bare
`external_type_specification` is accepted. The `external_body` attribute was redundant.

Verified: `make verify-kernel` → whole-crate `external_body` 12 → **11**, module
`mm::virt::identity_map` still **9 verified, 0 errors**. No other file touched for this.

### The 3 admits (identity_map.rs:533/627/718) — reproduced again, still blocked

Removed `ensure_pt`'s admit alone (no logical Ok/Err contradiction in *its* contract)
and ran `make verify-kernel MODULE=mm::virt::identity_map`:
```
error: precondition not satisfied  identity_map.rs:534:44  pd.read(pde_idx)
   --> table.rs:205  index@ < crate::mem::PAGE_TABLE_LENGTH   (failed precondition)
error: precondition not satisfied  identity_map.rs:549:9   alloc_as
   --> bump_allocator lib.rs:350  bump_view(self).inv()      (failed precondition)
verification results:: 8 verified, 1 errors  (exit 101)
```
- `bump_view(PAGE_TABLE_ALLOCATOR).inv()` is **unestablishable**: `bump_view` is
  `uninterp` (lib.spec.rs:177); `FixedSizeBumpAllocator::new()` (lib.rs:243) has **no
  `ensures`** and there is **no broadcast axiom/lemma** exposing `inv()`. Asserting it
  would be an `assume` in disguise. `bump_allocator` is out of scope.
- The `index@ < PAGE_TABLE_LENGTH` read precondition needs either a new `requires` on
  the helper (a contract change) or the `TableIndex` type invariant, which is
  cross-crate-unusable (`Verus Internal Error: missing type invariant function`).

`ensure_pte` / `identity_map_page` additionally carry **result-dependent opposite**
postconditions over the **parameterless** `uninterp identity_map_view()` —
`Ok => V.mapped.contains(p)` AND `Err => !V.mapped.contains(p)` (resp. `accessible`).
For one fixed `V` exactly one branch reduces to `=> false`; both exits are reachable.
Mathematically unsatisfiable for any constant; a concrete
`{initialized:false, mapped:∅}` redefinition fails the identical postconditions.

Sound elimination requires the deferred cross-module proving-phase ghost-token
framework (Tracked/PointsTo over `KERNEL_PD_PADDR`/`PAGE_TABLE_ALLOCATOR` threaded
through out-of-scope callers `init`/`KernelFrame::new`, and/or a sound contents
postcondition + permission token on `arch::…::Table::write`) — every realization
forbidden by the task hard rules (no exec signature change, no spec weakening, no new
`external_body` outside `tcb-allowed.md`, no new `assume_specification`, `table.rs` and
callers off-limits). This matches the independently-verified `STOP`/`BLOCKED` verdict and
the deferral every sibling `mm::phys::*` singleton free function uses.

Restored the 3 `admit()`s byte-identically; AST consistency:
`Consistent: 14 functions, 1 structs match`.

### Verification

```
make verify-kernel  → exit 0, 0 verification errors
  cheating: assume=0 external_body=11 admit=31 trusted=0 no_decreases=0 cfg_gate=15
  (module mm::virt::identity_map: 9 verified, 0 errors; external_body 0)
```

### Result: NEEDS_GUIDANCE

The `external_body` is **FIXED** (1 → 0). The 3 `admit()`s remain a verified
impossibility under the module-scoped hard rules — escalated to the proving-phase /
cross-module owner per the existing `STOP` (`BLOCKED`) adjudication and
`verification_todo.md`.
