## Turn 1: Full Checklist Walkthrough — specification phase, `arch-x86-pte`

### Progress
- Done (PASS): all 17 checklist items below.
- Current: full sweep complete.
- Remaining: none.

### Scope
In-scope (verification-order) functions — the only ones that may be annotated:
- `PageTableEntryFlags::new`
- `PageTableEntry::new`
- `PageTableEntry::is_present`
- `PageTableEntryFlags::is_present`

Files: `pte.rs`, `pte.spec.rs`, `pte.proof.rs` (proof file is empty: `verus! { }`).

---

### Item-by-item verdicts

**1. Every in-scope exec fn has requires/ensures — PASS**
Read `pte.rs`; `grep ensures` → lines 87, 180, 309, 407.
- `PageTableEntryFlags::new` (L86): `ensures result@ == spec_pte_flags_new(present, …, dirty)`.
- `PageTableEntryFlags::is_present` (L179): `ensures result == self@.present`.
- `PageTableEntry::new` (L308): `ensures result@ == spec_pte_new(flags@, frame@), result.inv()`.
- `PageTableEntry::is_present` (L406): `ensures result == self@.flags.present`.
No `requires` needed: both constructors are total/infallible; `FrameNumber`'s bound comes from its type invariant (`use_type_invariant(frame)`, proof L314). `fn_coverage.py` confirms 15/15 source exec fns matched (no missing/extra); the other 11 are out of scope.

**2. Caller coverage — PASS**
Cross-checked `caller_analysis.md` "Caller Expectations" against specs:
- Flags::new: callers need `is_present()==(present==Present)` and `cow` default `NotCopyOnWrite`. `spec_pte_flags_new` sets `present=spec_present_set(present)` and `cow=false`; `is_present` spec projects `self@.present`. ✓ All seven argument bits recorded (rejects an arg-dropping `new`). ✓
- Entry::new: callers need `is_present()==flags.is_present()`, faithful `(flags,frame)`. `spec_pte_new` makes `result@.flags==flags@`, `result@.frame==frame@`; entry `is_present` delegates to `self@.flags.present`. ✓ (`frame_number()`/`flags()` accessors are out of scope.)
- Entry::is_present / Flags::is_present: present-bit projections — match `map`/`unmap`/`ensure_pte`/`fill` control-flow expectations. ✓

**3. View consistency — PASS**
Specs reference View fields (`self@.present`, `self@.flags.present`, `result@`) and `inv()`. Views match `view_design.md`: `PteFlagsView` (8 bools), `PteView{flags,frame:int}`, both `closed`. `PageTableEntry::inv` enforces `0 <= self@.frame <= FrameNumber::spec_max()`; `Flags::inv == true` (vacuous, by design — no cross-field coupling). Implementation imports `spec_present_set…spec_d_set` from the `paging` module (reused from the `pde` sibling) instead of redefining — documented in `pte.spec.rs` L7–18 to avoid glob-reexport collisions; semantically identical to the view-design listing. `new` includes `result.inv()` so callers get the frame bound directly.

**4. No tautological ensures — PASS**
All ensures are equalities (`result@ == …`, `result == self@…`). No `Err(_) => true` / `_ => true`.

**5. No subsumed ensures — PASS**
`Entry::new`'s `result.inv()` is not subsumed: it is the property being established (frame bound on the result), derivable only by invoking the `FrameNumber` argument's type invariant — useful to expose to callers. `result@ == spec_pte_new(...)` is independent.

**6. Error paths have meaningful ensures — PASS (N/A)**
No in-scope fn returns `Result`/`Option`. Constructors are infallible; queries return `bool` with exact-value ensures. Vacuously satisfied.

**7. No assume_specification for workspace-internal code — PASS**
`grep -nE "assume_specification" pte.{rs,spec.rs,proof.rs}` → none.

**8. vstd searched before assume_specification — PASS (N/A)**
No `assume_specification` in this module.

**9. Specs written for the caller — PASS**
Specs are stated purely over View projections (`@.present`, `@.flags.present`) and spec transitions (`spec_pte_new`, `spec_pte_flags_new`) — directly usable in kernel caller proofs (`page_table.rs`, `identity_map.rs`), no leakage of `PteWord`/`FrameNumber` internals.

**10. Trait obligations satisfied — PASS**
`TableEntry::{from_raw,raw}` round-trip is a documented boundary obligation but **out of scope** (caller_analysis §Trait Obligations). The in-scope `new` spec is consistent with that obligation (faithful `(flags,frame)`).

**11. Spec completeness (advisory) — PASS**
The four in-scope ensures fully pin each observable result bit (including the defaulted `cow=false`). No unintended nondeterminism.

**12. Loop invariants — PASS (N/A)**
`grep -nE "\b(for|while|loop)\b" pte.rs` → none (only the `TableEntry` impl line). No loops to annotate.

**13. No cheating on module's own functions — PASS**
`grep -nE "admit|assume|external_body|trusted|assume_specification"` over `pte.rs`/`pte.spec.rs`/`pte.proof.rs` → exit 1 (zero matches).
Crate-wide counts from `make verify-arch` (`assume=0 external_body=2 admit=1 trusted=0`) all belong to a different module — `cheating-detail.txt`:
- `x86/mem/paging/table.proof.rs:8 lemma_entry_roundtrip: admit`
- `x86/mem/paging/table.rs:209 read: external_body`
- `x86/mem/paging/table.rs:246 write: external_body`
None are in the `pte` module. This module is clean.

**14. No specs weakened — PASS**
`spec_drift.py check arch-x86-pte` (baseline 2462d3636ab1 → HEAD): "Functions with changes: 0; Contract drift: 0; Ensures removed: 0; Requires added: 0. ✅ No contract drift detected." The new specs add strength over the upstream signature-only `assume_specification` placeholders in `identity_map.spec.rs` (which had no `ensures`); nothing weakened.

**15. Bug awareness — PASS**
No `bugs.md` present; prover summary reports "No bugs found." Manual read of `pte.rs` reveals no fundamentally incorrect/contradictory code in scope.

**16. Cross-module regression — PASS**
- `make verify-arch` → exit 0 (all arch modules, cached, 0 errors).
- `make verify` (kernel, depends on arch) → exit 0, cached, "55/1026 contracts", 0 errors. Pre-existing kernel admit=36/external_body=12 unchanged.

**17. Verification + build — PASS**
- `make verify-arch` → Exit code 0.
- `./z build -- all` → `[OK] Build complete.` (exit 0).

---

### Verdict
All 17 checklist items PASS with concrete tool/code evidence. No fix requests. Specification phase for `arch-x86-pte` is RESOLVED.
