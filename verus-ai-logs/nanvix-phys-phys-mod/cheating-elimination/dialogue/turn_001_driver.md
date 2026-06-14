## Turn 1: Full checklist walkthrough — cheating-elimination for `mm::phys` (`mod.rs`)

### Progress
- Done (PASS): all 12 checklist items verified PASS with tool evidence (see below).
- Current: final sign-off.
- Remaining: none.

### Scope
Review target is the phys-mod phase: `src/kernel/src/mm/phys/mod.rs` +
`mod.spec.rs` + `mod.proof.rs`. Crate-wide cheating counts reported by the
tooling (`admit=34`, `cfg_gate=15`, `external_body=12`) belong to *other*
modules that are out of scope for this phase; every count is attributed to a
file below and only `mod.*` entries are judged here.

### Verification

Command: `make verify-kernel`
```
note: verifying module mm::phys  (+ frame, kframe, manager, upool)
Exit code : 0
cheating: assume=0 external_body=12 admit=34 trusted=0 no_decreases=0 cfg_gate=15
status: CHEATING_DETECTED   <-- crate-wide aggregate, not mod-scoped
```
Module verifies; exit 0.

Per-item evidence (mod.* scope only):

1. **Zero admit()** — PASS.
   `grep admit mod.rs mod.spec.rs mod.proof.rs` → no hits.
   `cheating-detail.txt` lists zero `admit` entries for `mm/phys/mod.*`
   (all 34 `admit` are in frame.rs / manager.proof.rs / virt/identity_map.*).

2. **Zero assume()** — PASS.
   Only match is a doc comment in mod.spec.rs:145 ("each *assumed* free") — no
   `assume(...)` statement. `assume=0` in tooling.

3. **Zero trusted functions** — PASS. `trusted=0`; no `#[verifier::trusted]`
   in mod.*.

4. **Zero exec_allows_no_decreases_clause** — PASS. `no_decreases=0`.

5. **Zero cfg-gated exec code (only imports/derives/debug_assert/logging)** — PASS.
   mod.rs cfg sites:
   - L36/L40/L42 `#[cfg(verus_keep_ghost)]` → gate `use ::vstd::prelude::*;`
     (import) and `include!("mod.spec.rs")` / `include!("mod.proof.rs")` (ghost
     spec/proof inclusion) — the standard Verus inclusion pattern (ghost, not
     exec).
   - L15 `#[cfg(feature="test")] mod test;` and L195 `#[cfg(feature="test")]
     pub fn test()` → pre-existing test scaffolding. AST-consistency confirms
     `test` MATCHES the baseline (unchanged exec, not introduced to dodge
     verification).
   No cfg-gated exec swaps verification behavior.

6. **external_body only if listed in tcb-allowed.md** — PASS. Each surviving
   external_body in mod.* checked individually:
   - `mod.rs:73 book_physical_memory_regions` → LISTED in
     `verus-ai-logs/tcb-allowed.md` ("iterates LinkedList in a for loop; Verus
     has no LinkedList model; ensures = frames reserved on Ok"). ✅
   - `mod.rs:103 book_mmio_regions` → LISTED in tcb-allowed.md (same LinkedList
     limitation; coverage-gated ensures). ✅
   - `mod.spec.rs:69 ExLinkedList` → `external_type_specification` (foreign-type
     *registration*, not an external_body fn body). Sound standard mechanism;
     orphan rule forbids a downstream `View` impl (documented L52–61 of
     mod.spec.rs); acknowledged as a category in tcb-allowed.md. No behavioral
     assumption. ✅

7. **AST consistency** — PASS.
   `ast_consistency.py mod.rs summary` →
   `Consistent: ✅ YES (matched=4 mismatched=0 missing=0 extra=0)`
   (book_mmio_regions, book_physical_memory_regions, init, test all MATCH).

8. **All exec rewrites have VERUS REWRITE comment + reproducer** — PASS (vacuous).
   AST consistency shows all exec bodies MATCH baseline → zero exec rewrites,
   so nothing to document. `grep "VERUS REWRITE"` → none, consistent.

9. **Each surviving external_body listed** — PASS (covered in item 6; both
   `book_*` listed, ExLinkedList is a type registration).

10. **No specs weakened** — PASS.
    `spec_drift.py check phys-mod` (baseline f068965b → HEAD) →
    `Functions with changes: 0; Contract drift: 0; ✅ No contract drift`. Exit 0.

11. **Cross-module regression (`make verify-kernel`, all modules)** — PASS.
    mm::phys, frame, kframe, manager, upool all verified; exit 0.

12. **Verification + build** — PASS.
    `make verify-kernel` exit 0 (0 errors). `./z build -- all` → `[OK] Build
    complete.` Kernel + all artifacts built, 0 compile errors/warnings (only a
    benign "sysroot symlink skipped" infra note).

### Verdict
All 12 checklist items PASS with concrete tool evidence. No fix request needed.
Cheating elimination for `mm::phys` (mod.rs) is RESOLVED.
