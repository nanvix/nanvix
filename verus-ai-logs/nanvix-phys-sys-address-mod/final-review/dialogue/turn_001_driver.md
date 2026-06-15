## Turn 1: Full checklist walkthrough — `sys::mm::address::mod` (the `Address` trait)

### Progress
- Done (PASS/FIXED): ALL checklist items (see below).
- Current: Specification → "No subsumed ensures" — **was the only FAIL; now FIXED.**
- Remaining: none.

### Scope reminder
In-scope items are the three `pub trait Address` **method declarations** in
`src/libs/sys/src/sys/mm/address/mod.rs`: `from_raw_value`, `into_raw_value`,
`is_aligned`. They carry `#[verus_spec]` contracts but have **no exec bodies**,
so the local proof obligation is empty — obligations fall on each
`impl Address for …` implementor, verified/trusted in its own module.
Spec vocabulary lives in `mod.spec.rs`; `mod.proof.rs` is an empty `verus!{}`.

### Tools run by this reviewer (ground-truth evidence)
- `make verify-sys` → **6 verified, 0 errors, exit 0, status CLEAN**
  (`cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0`).
- `make verify` (cross-module) → exit 0 for every crate:
  bitmap 70/0 (cfg_gate=5 pre-existing), **sys CLEAN**, nanvix-slab 35/0
  (cfg_gate=1 pre-existing), bump-allocator (external_body=2 pre-existing, in
  `tcb-allowed.md`), kernel 47/0 (external_body=24, cfg_gate=6 — pre-existing
  TCB, in `tcb-allowed.md`). No new cheating introduced by this module.
- `fn_coverage.py mod.rs mod.rs` → Source exec fns: **0** (trait declarations,
  no bodies). Missing: 0. Extra: 0.
- `ast_consistency.py mod.rs` → **"✅ All exec functions consistent"**, 0/0.
- `spec_drift.py git-diff … --before 7c2fdcf95` (pre-work baseline, where
  `mod.spec.rs`/`mod.proof.rs` were empty) → **Contract drift: 0; Ensures
  removed: 0; Requires added: 0; Functions added: 3.** Purely additive.
- Cheating greps in the module dir: every `external_body` / `assume_specification`
  / `trusted` hit is inside a **comment**; no live attribute. Confirmed by the
  CLEAN verify-sys cheating report.

---

## Checklist verdicts

### Caller Analysis
- [x] PASS — pub functions callers searched: `find_callers_lsp.py` returns 0/0
  (tree-sitter cannot resolve trait methods as free fns); supplemented by
  whole-tree manual analysis in `caller_analysis.md`. Tool-verified limitation,
  not an absence of callers (`sys` is depended on by 50+ crates).
- [x] PASS — caller expectations (success + failure) documented per in-scope
  method (`caller_analysis.md` §Caller Expectations).
- [x] PASS — abstract resource identified: one pointer-sized integer,
  `0 ≤ addr ≤ usize::MAX`.
- [x] PASS — pre-existing specs assessed (inherent `VirtualAddress::new` /
  `from_raw_value`; trait methods previously unspecced; kernel
  `assume_specification` boundary noted).

### View Design
- [x] PASS — substitution test: scalar `int` projection (`spec_addr`) survives a
  full reimplementation; documented in `view_design.md`.
- [x] PASS — all caller-observable state represented (the numeric address).
- [x] PASS — no implementation-specific fields (scalar `int` only).
- [x] PASS — `inv()` (`addr_inv` = `0 ≤ spec_addr ≤ usize::MAX`) encodes a real,
  non-trivial constraint.
- [x] PASS — mathematical types used (`int`). Addresses-keep-usize exception is
  respected at the exec boundary (`raw_addr: usize`, `result: usize`).

### Specification
- [x] PASS — every in-scope exec function has requires/ensures: `fn_coverage.py`
  → 0 source exec fns; the 3 trait declarations each carry an `ensures`.
- [x] PASS — caller coverage: 3/3 in-scope methods map to a caller expectation
  in `caller_analysis.md`.
- [x] PASS — view consistency: specs reference `spec_addr` / `addr_is_aligned`
  (and previously `addr_inv`); consistent with `view_design.md`.
- [x] PASS — no tautological ensures: `from_raw_value` `Err` arm is the concrete
  `e.code == ErrorCode::BadAddress`, not `Err(_) => true`.
- [x] **FIXED** — No subsumed ensures. **Original defect:** `from_raw_value`'s
  `Ok` arm was `spec_addr(&a) == raw_addr as int && addr_inv(&a)`. `addr_inv(&a)`
  = `0 ≤ spec_addr(&a) ≤ usize::MAX` is **derivable** from
  `spec_addr(&a) == raw_addr as int` because `raw_addr: usize` forces
  `0 ≤ raw_addr as int ≤ usize::MAX` → strictly subsumed.
  **Fix applied (mod.rs:61–67):** dropped the `&& addr_inv(&a)` conjunct, leaving
  `Ok(a) => spec_addr(&a) == raw_addr as int`, with a comment recording the
  derivation. **Verification of fix:** `make verify-sys` → 6 verified, 0 errors,
  CLEAN; `spec_drift` vs pre-work baseline → 0 contract drift (no original
  guarantee weakened — the dropped conjunct is logically recoverable by callers).
- [x] PASS — no subsumed ensures elsewhere: `into_raw_value`
  (`result as int == spec_addr(&self)`) and `is_aligned` (`result is Ok` +
  `Ok_0 == addr_is_aligned(spec_addr(self), align)`) are each independent and
  non-derivable.
- [x] PASS — error paths meaningful: `from_raw_value` `Err` pins `BadAddress`
  (match style). `is_aligned` is intentionally total (`result is Ok`), justified
  by `Alignment` being a closed enum of powers of two.
- [x] PASS — no `assume_specification` for workspace-internal code (0 live in module).
- [x] PASS — vstd searched before any `assume_specification` (N/A — none added).
- [x] PASS — specs written for the caller: `into_raw_value` identity is exactly
  the fact the kernel pins via `assume_specification` today.
- [x] PASS — trait obligations satisfied: contracts match the trait-level
  semantic contract; `VirtualAddress` impl re-verifies (verify-sys 6/0).
- [x] PASS — spec completeness (advisory): trait-level nondeterminism on
  `from_raw_value` failure matches heterogeneous implementor domains
  (`VirtualAddress` infallible; `PhysicalAddress`/`PageAligned` fallible).
- [x] PASS — loop invariants: no loops in scope.
- [x] PASS — no cheating on module's own functions: admit=0, assume=0,
  external_body=0, trusted=0 (all keyword greps land in comments only).
- [x] PASS — no specs weakened: `spec_drift.py` → 0 contract drift.
- [x] PASS — bug awareness: `bugs.md` present; no code defects (declarations only).
- [x] PASS — cross-module regression: `make verify` → all crates exit 0.
- [x] PASS — verification: `make verify-sys` 6/0 CLEAN. (No `make build` target
  in this repo; compilation is exercised by the verify run, which `cargo check`s
  the crate.)

### Proving
- [x] PASS — no specs weakened (`spec_drift` clean).
- [x] PASS — zero remaining `admit()` (grep: 0 live).
- [x] PASS — zero `external_body` (0 live; nothing to check vs `tcb-allowed.md`).
- [x] PASS — zero assume/assume_specification (0 live).
- [x] PASS — no cfg-gated exec code: the two `#[cfg(verus_keep_ghost)]` gate only
  `include!` of spec/proof files — canonical ghost-include, not exec branches.
- [x] PASS — cheating audit counts reported (all 0; locations: none).
- [x] PASS — claimed Verus limitation reproducer: N/A (no exec rewrites).
- [x] PASS — exec rewrites minimal/equivalent: none; no `// VERUS REWRITE`.
- [x] PASS — cross-module regression (`make verify` clean).
- [x] PASS — verification 0 errors, 0 warnings.

### Cheating Elimination
- [x] PASS — zero `admit()`.
- [x] PASS — zero `assume()`.
- [x] PASS — zero trusted functions.
- [x] PASS — zero `exec_allows_no_decreases_clause`.
- [x] PASS — zero cfg-gated exec code (only ghost `include!`).
- [x] PASS — zero `external_body` (none; none needing a TCB entry).
- [x] PASS — AST consistency: `ast_consistency.py` → consistent, 0/0.
- [x] PASS — all exec rewrites have VERUS REWRITE + reproducer: N/A (none).
- [x] PASS — each surviving `external_body` in `tcb-allowed.md`: N/A (zero in module).
- [x] PASS — no specs weakened (`spec_drift` clean).
- [x] PASS — cross-module regression (`make verify` clean).
- [x] PASS — verification 0 errors, 0 warnings.

Note on `uninterp spec fn spec_addr<T>` (`mod.spec.rs:41`): **not a violation of
this checklist.** It is not `external_body`/`assume`/`assume_specification`/
`trusted`/`admit` (the cheating checker reports all 0 and `make verify-sys` is
CLEAN). It introduces **no axiom** — `mod.proof.rs` is empty, nothing feeds it;
its properties are pinned operationally by the trait-method exec contracts that
implementors discharge with their own concrete `closed view()`. It is **not** on
the `verus-constraints` Forbidden-Patterns table (admit / assume / external_body
/ trusted / external / exec_allows_no_decreases_clause / rlimit), and it has
direct verified precedent in this codebase (`kernel .../aligned/page.spec.rs:31`
plus 7 other in-tree `uninterp spec fn` declarations). It is architecturally
forced here (an `Address: View<V=int>` supertrait is impossible because
per-implementor `View` impls are `cfg(verus_keep_ghost)`-gated, and an in-crate
`spec_addr<T: Address>` bound forms a definition cycle — documented in
`bugs.md §1`). No checklist item requires its elimination.

### Bug Recording
- [x] PASS — `bugs.md` exists (records "no code bugs" + non-bug findings).
- [x] PASS — each bug a real defect: N/A (none recorded; declarations only).
- [x] PASS — What/Why/How-Verus-Helped/Severity/Suggested-Fix: N/A (no bugs).
- [x] PASS — no `external_body` used to mask a defect (none in module).
- [x] PASS — provenance recorded (specification phase).

---

### Fix Request
None outstanding. The single FAIL ("No subsumed ensures") was **FIXED in place**:
- File: `src/libs/sys/src/sys/mm/address/mod.rs`, `from_raw_value` `ensures`.
- Change: removed the subsumed `&& addr_inv(&a)` conjunct from the `Ok` arm.
- Verify command run: `make verify-sys` → 6 verified, 0 errors, CLEAN;
  `make verify` → all crates exit 0; `spec_drift` → 0 contract drift;
  `ast_consistency` → consistent.

All checklist items are PASS or FIXED with tool evidence. → RESOLVED.
