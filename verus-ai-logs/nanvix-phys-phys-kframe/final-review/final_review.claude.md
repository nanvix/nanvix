# Final Verification Review — `mm::phys::kframe` (`KernelFrame`)

Reviewer: independent strict final review (Claude)
Branch: `verus-ai/phys-kframe` (base `verus-ai/phys-upool`)
Scope (in-scope targets ONLY): `KernelFrame::new`, `KernelFrame::drop`, `KernelFrame::base`.
Method: read-only re-derivation (grep / cat / git diff / ast_consistency.py). `make` NOT
re-run (parallel reviewer + in-progress build); relied on shared pre-run logs for
verification/build, independently re-derived all guardrail/AST/TCB facts.

## Checklist

- [x] Verification PASS — `make verify-kernel MODULE=mm::phys` exit 0, 0 errors (shared log).
- [x] No `admit()` anywhere in the 3 kframe files.
- [x] No `assume(...)` anywhere in the 3 kframe files.
- [x] Every `external_body` in kframe files is listed in `tcb-allowed.md` (only `clear`).
- [x] `assume_specification` (`Address::from_raw_value`) listed in `tcb-allowed.md`.
- [x] No new trust boundary introduced beyond the FIXED TCB.
- [x] No `// VERUS REWRITE` comments present → no AST-rewrite mismatches.
- [x] AST consistency check PASS (`6 functions, 1 struct match`).
- [x] No `#[verifier::trusted]`, `#[verifier::external]`, `exec_allows_no_decreases`, or `rlimit`.
- [x] `new` Ok-path address-identity ensures present and caller-usable.
- [x] `base` accessor ensures present (`result@ == self@`).
- [x] `drop` ensures `phys_view().inv()` + `no_unwind` + `opens_invariants none`.
- [x] All in-scope caller expectations covered by requires/ensures.
- [x] Do-not-modify spec/view defs untouched (none referenced/altered in kframe files).
- [x] cfg-gates are import/include only (no exec-function gating).
- [x] No real code defect in the in-scope functions (bugs.md correctly absent).

## Spec Quality

**`new(base: FrameAddress) -> Result<Self, Error>`**
- `Ok(frame) => frame@ == base@`: address identity. Directly caller-written — the
  constructors (`manager.rs:354/430`) use it verbatim to assert
  `allocated_frames.contains(frame@)` and the page-stride contiguity predicate. WHAT,
  not HOW; survives reimplementation. Good.
- `Err(_) => true`: tautological on its face, but **justified**. (a) `base` is `Copy`,
  so it is structurally never consumed — the "caller still owns/must-free `base` on
  error" guarantee the caller analysis demands is delivered by the type system, not
  needing a spec clause. (b) The view_design's proposed `phys_view() == old(phys_view())`
  frame condition is **not expressible**: `phys_view()` is a 0-arg uninterpreted spec
  fn with no `old()` snapshot mechanism (same documented limitation as `drop`/`frame::free`).
  (c) `new` does not touch allocator state at all (page tables come from a BSS pool, no
  recursive frame alloc), and there is no `requires phys_view().inv()` to re-establish,
  so no stronger error-path fact is available. The inline rationale comment states this
  accurately. Acceptable.
- The identity-map side effect (`identity_map_page`) is deliberately unspecified — no
  caller depends on it abstractly (confirmed by caller_analysis.md). Correct omission per
  "declarative not operational".

**`base(&self) -> FrameAddress`**: `result@ == self@`. Textbook trivial-accessor spec
(spec-design Part 1). Field *is* the view, so it collapses correctly. No state change
(immutable borrow). Good.

**`drop(&mut self)`**: `phys_view().inv()`, `no_unwind`, `opens_invariants none`. Single-
state monotone fact — **justified** by the documented `phys_view()` limitation (no
`old()` against a global uninterpreted fn), mirroring the approved `frame::free` and
`UserFrame::drop` shims. The exact refcount/allocated→free transition is genuinely
inexpressible here; the caller-meaningful guarantees (invariant preserved, never
unwinds, opens nothing) are exactly what make `Vec<KernelFrame>::clear()` a sound bulk
free and enable RAII rollback. Discharged directly by `free`'s every-path ensures. Good.

No anti-patterns found: no operational/code-as-spec clauses, no subsumed/over-spec
clauses, no missing loop invariants (no loops in scope). Error-path treatment is as
rigorous as the (limited) abstraction permits.

## Caller Coverage (Covered 7/7, Missing: none)

In-scope abstract caller expectations from `caller_analysis.md`:

| # | Function | Expectation | Status |
|---|----------|-------------|--------|
| 1 | new | Ok: `frame@ == base@` (address identity) | Covered (ensures) |
| 2 | new | Err: no ownership transfer; `base` still caller's to free | Covered (Copy semantics + `Err(_)=>true`; not consumable by construction) |
| 3 | base | `result@ == self@`, exact owned FrameAddress | Covered (ensures) |
| 4 | base | pure read, no state change | Covered (`&self`, ensures-only) |
| 5 | drop | releases frame, preserves `phys_view().inv()` | Covered (ensures) |
| 6 | drop | never panics / never unwinds | Covered (`no_unwind`) |
| 7 | drop | opens no invariant | Covered (`opens_invariants none`) |

Non-spec expectation: "frame is now identity-mapped" (new) — caller_analysis explicitly
states *no caller depends on it abstractly*; correctly left out of the contract.
Out-of-scope (`clear`/`deref`/`deref_mut`): TCB / unverified exec, not part of this target.

## Proof Completeness

- `admit()` count in kframe.rs / kframe.spec.rs / kframe.proof.rs: **0**. Locations: none.
- `external_body` NOT in TCB: **0**. Locations: none.
  - Only `external_body` in the files: `kframe.rs:140` (`clear`) — listed in `tcb-allowed.md`.
- `assume(...)`: **0**.
- kframe.proof.rs contains no lemmas (`verus! { }` empty) — correct: `new` identity follows
  from the `View` impl, `base` is trivial, `drop` is discharged by the `free` shim contract.

## TCB Compliance: **YES**

- `KernelFrame::clear` (`external_body`) → present in `tcb-allowed.md`
  (§ "kframe.rs::KernelFrame::clear"). ✔
- `<PageAligned<T> as Address>::from_raw_value` (`assume_specification` in kframe.spec.rs)
  → present in `tcb-allowed.md` (§ "assume_specification — sys::mm::Address trait method"). ✔
- `KernelFrame::deref` / `deref_mut`: listed in TCB but in code are **plain unverified exec
  fns with no contract and no attribute** (not `external_body`) — out of the 3 in-scope
  targets; no trust boundary added. ✔
- No `external_body`/`assume_specification` exists in the kframe files beyond the two above.
  TCB is FIXED and was not expanded.

## Guardrails Compliance

kframe.rs + kframe.spec.rs + kframe.proof.rs:

```
admit:0  assume:0  external_body:1 (clear, in TCB)  assume_specification:1 (in TCB)  cfg-gated-exec:0
```

(cfg directives present are `#[cfg(verus_keep_ghost)]` on imports/includes only — allowed,
not exec-gating. Module-aggregate shared log: assume=0 admit=0 trusted=0 no_decreases=0,
external_body=28 all TCB-listed, cfg_gate=9.) `admit:0` and `assume:0` ⇒ no guardrail blocker.

## AST Consistency: **PASS**

- `python3 ast_consistency.py src/kernel/src/mm/phys/kframe.rs count` →
  `✅ Consistent: 6 functions, 1 structs match.`
- `grep "// VERUS REWRITE"` over the 3 files → 0 hits, so no rewrite to audit for
  semantic equivalence. Exec code is byte-faithful to the base branch.

## Verification: **PASS**

Shared pre-run log `final-review/verify-kernel.log`: `make verify-kernel MODULE=mm::phys`
exit 0, verification cached, 0 errors. kframe entry in cheating-detail.txt is solely
`kframe.rs:141 clear: external_body` (TCB). `KernelFrame::new` carries a `#[verus_spec]`
and is verified (the lone "new" in the coverage-unverified list is `upool.rs`'s
`UserFrame::new`/`Upool::new`, not the in-scope `KernelFrame::new`). `make build` exit 0.

## Bug Summary

Total recorded: 0. True Bugs: 0.

`bugs.md` is correctly absent. Per the bug-reporting skill, a report requires a *real code
defect* independent of calling context — not a verification limitation. The in-scope bodies
are correct: `new` returns `Ok(Self { base })` only after a successful align-check +
identity-map, returning early (and never consuming the `Copy` `base`) on failure; `base` is
a pure field read; `drop` calls `free(self.base)` and logs (never propagates) errors. No
arithmetic/logic/ownership defect exists. The single-state `drop` ensures and `new`'s
`Err(_)=>true` are deliberate, documented abstraction limits (uninterpreted `phys_view()`),
not bugs. Nothing should have been recorded.

## Issues (highest priority first)

None blocking. Minor observations (informational, no action required):

1. (Cosmetic) `tcb-allowed.md` lists `deref`/`deref_mut` under "Allowed external_body" but
   they are actually unannotated unverified exec fns (no `external_body`). Harmless
   over-listing; does not expand the trust boundary. Could be reworded for accuracy.
2. (Informational) `new`'s `Err(_) => true` is the weakest admissible error-path clause; it
   is justified here (Copy input + inexpressible `old(phys_view())`), but readers should
   note the no-consumption guarantee rests on the type system rather than the contract text.

## Result: PASS

All checklist items pass; zero blockers (admit=0, assume=0, no non-TCB external_body, no AST
mismatch, verification 0 errors). Spec quality, caller coverage, TCB compliance, guardrails,
AST consistency, and bug reconciliation all clear.
