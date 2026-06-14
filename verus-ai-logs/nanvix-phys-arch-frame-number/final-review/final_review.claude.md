# Final Comprehensive Review — `arch-frame-number` (`FrameNumber`)

**Reviewer:** Independent strict final review (Claude)
**Module:** `src/libs/arch/src/x86/mem/paging/frame/number.{rs,spec.rs,proof.rs}`
**In-scope items:** `FrameNumber::from_raw_value`, `FrameNumber::into_raw_value`, and the type
`FrameNumber` (its `View` + `inv` + the supporting `spec_max`).
**Method:** Read all source/spec/proof + analysis docs + the four mandated skills. Independently
re-derived the correctness arguments and re-ran `ast_consistency.py`, including against the true
pre-verification baseline. I relied on the orchestrator's `make verify-arch` PASS (exit 0) and did
**not** run `make` myself.

---

## Spec Quality

The shipped contract is small, declarative, and correct.

**`View`** — `type V = int; closed spec fn view(&self) -> int { self.0 as int }`.
- Scalar `int` abstraction = "the frame index". Matches the sibling address-tower views
  (`PhysicalAddress@ : int`, etc.). `closed` correctly hides the newtype→`int` mapping so callers
  reason only through the transitions. Not a mirror of an internal bookkeeping field — the value
  *is* the entire caller-observable state, so this is the right level of abstraction.

**`spec_max()`** — `pub open spec fn spec_max() -> nat { (mem::MAX_ADDRESS / mem::FRAME_SIZE - 1) as nat }`.
- This is the actual shipped form and it **diverges from `view_design.md`**, which proposed
  `uninterp spec fn spec_max()` plus a trusted `assume_specification[FrameNumber::MAX]`. The shipped
  form is the **stronger, no-trust** choice: it *defines* the bound as the same arithmetic the exec
  constant computes, so the `spec_max ↔ MAX` binding is **discharged by verification** instead of
  assumed. This removes an external-bottom trust assumption entirely. **Assessment: the shipped
  approach is strictly better and fully acceptable; the divergence from `view_design.md` is a
  documentation lag, not a defect.** (See Issues #2.)
- Soundness of the `nat` cast: `MAX_ADDRESS = usize::MAX`, `FRAME_SIZE = 4096`. Therefore
  `MAX_ADDRESS / FRAME_SIZE = usize::MAX / 4096 ≥ 1`, so `… - 1 ≥ 0` and the `as nat` cast cannot
  underflow. **Sound.** (Confirmed by reading `x86/mem/constants.rs:97,113`.)
- Mirrors exec `MAX` exactly: exec `pub const MAX = mem::MAX_ADDRESS / mem::FRAME_SIZE - 1` is the
  identical expression. Both `MAX_ADDRESS` and `FRAME_SIZE` carry `#[verus_verify]`, so Verus
  evaluates them concretely and proves `Self::MAX as int == spec_max() as int`. Since
  `make verify-arch` passes, this binding is in fact discharged. Boundary in `from_raw_value`
  (`value > Self::MAX`, usize) therefore coincides exactly with the spec boundary
  (`value as int <= spec_max()`, int) — **no off-by-one.**

**`inv()`** (`#[verifier::type_invariant]`) — `0 <= self@ <= Self::spec_max()`.
- Exactly strong enough: it implies both the in-range `ensures` of `into_raw_value` and the
  overflow-safety bound callers need, and no stronger (no spurious non-null/alignment clause —
  correctly, since `NULL` = frame 0 is valid and indices carry no alignment). Provable at every
  construction site (see Proof Completeness). Correct invariant strength.

**`from_raw_value` ensures** — bidirectional, non-tautological:
```
value as int <= spec_max() ==> result is Some && (result->Some_0)@ == value as int
value as int >  spec_max() ==> result is None
```
- Both directions present (iff-style success/failure split) — satisfies the error-path "bidirectional
  failure condition" and "liveness" principles. Pins the success value (`@ == value as int`), so it is
  not a one-sided or tautological spec. **Adversarial test:** a no-op returning `None` always fails
  clause 1; a truncating impl violates `@ == value as int`; an always-`Some` impl violates clause 2.
  Spec rejects all three. Weakest precondition (no `requires`) — accepts any `usize`, correct since
  the range check is inherently dynamic input validation.

**`into_raw_value` ensures** — `result as int == self@` and `0 <= self@ <= spec_max()`.
- Value-preserving projection plus the in-range fact (restating `inv`) that underwrites the caller's
  `<< FRAME_SHIFT`. Total (no `requires`, no `Option`). Declarative. The second clause is a
  deliberate, caller-useful restatement of `inv` so callers get the bound without separately invoking
  `use_type_invariant` — not redundant subsumption in a harmful sense; it is the documented contract
  the kernel placeholder assumed.

**Readability:** comments explain `closed`, the `nat` choice, and why the bound is a type invariant.
Clear enough for human audit.

**Minor spec-design observations (non-blocking):**
- `spec_max()` is an extra `pub spec fn` on `impl FrameNumber` beyond `inv`/`view`, which the
  spec-design skill discourages ("put helpers on `MyTypeView`"). Justified here because the View is a
  primitive `int` with nowhere to host helpers, and downstream kernel crates must reach the bound via
  the exported type. Documented in `view_design.md`. Acceptable deviation.
- The exec file uses attribute style (`#[verus_spec]`, `#[verus_verify]`, `proof!{}`) rather than the
  `verus!{}` block style the verus-constraints skill prefers. This is the established repo-wide
  convention for in-place exec annotation across the whole `arch` crate; the `.spec.rs`/`.proof.rs`
  files correctly use `verus!{}`. Convention-consistent; non-blocking.

---

## Caller Coverage (Covered 4/4)

From `caller_analysis.md`, the four caller expectations and their covering clauses:

| # | Caller expectation | Covered by | Status |
|---|--------------------|-----------|--------|
| 1 | Round-trip identity: `from_raw_value(v).map(into_raw_value) == Some(v)` for `v ≤ MAX` | `from_raw_value` ensures (`Some_0@ == value as int`) ∘ `into_raw_value` ensures (`result as int == self@`) | ✅ |
| 2 | Out-of-range rejection: `v > MAX ⇒ None` | `from_raw_value` ensures clause 2 (`value as int > spec_max() ==> result is None`) | ✅ |
| 3 | In-range bound enabling overflow-safe `<< FRAME_SHIFT` | `into_raw_value` ensures `0 <= self@ <= spec_max()` (+ `inv` type invariant making it unconditional) | ✅ |
| 4 | Totality of `into_raw_value` (no panic, no `Option`, value-preserving) | `into_raw_value` is total (returns `usize`, no `requires`) + `result as int == self@` | ✅ |

Note on #3: this module's job is to *export the bound*; the actual no-overflow proof of
`value * FRAME_SIZE ≤ usize::MAX` lives in the PTE/PDE callers. The bound supplied here is sufficient
(`self@ ≤ usize::MAX/4096 − 1` ⇒ `self@ << 12 < usize::MAX`). Obligation correctly met at this layer.

**Missing: none.**

---

## Proof Completeness

- `admit()` in module: **0** (grep across `.rs`/`.spec.rs`/`.proof.rs`).
- `external_body` in module: **0**.
- `into_raw_value` uses `proof! { use_type_invariant(self); }` to bring `inv(self)` into scope, then
  returns `self.0`. This is the correct, sound way to discharge the `0 <= self@ <= spec_max()`
  postcondition for a `#[verifier::type_invariant]` type. No proof escape.
- `from_raw_value`'s `Some(Self(value))` construction discharges the type invariant at the
  construction site: on that path `value ≤ Self::MAX` (= `spec_max()`) and `value ≥ 0` (usize), so
  `inv` holds. Verus checks this at struct construction; `make verify-arch` PASS confirms it.
- `number.proof.rs` is intentionally empty (`verus! { }`) — no proof obligations needed beyond the
  inline `use_type_invariant`. Legitimate, not a stub hiding an `admit`.

**No blockers.** No `admit()` anywhere; no `external_body` in the module.

---

## TCB Compliance

The 3 arch-wide `external_body` functions, all **outside** this module, are each present in
`tcb-allowed.md`:

| `external_body` | Location | In `tcb-allowed.md`? |
|-----------------|----------|----------------------|
| `invlpg` | `x86/mem/paging/mod.rs:80` | ✅ (inline-asm hardware boundary) |
| `Table::<E>::read` | `x86/mem/paging/table.rs:209` | ✅ (int→ptr volatile page-table read) |
| `Table::<E>::write` | `x86/mem/paging/table.rs:246` | ✅ (int→ptr volatile page-table write) |

None reside in `number.rs`. The in-scope module introduces **zero** `external_body`, so there is
nothing for it to justify. **TCB compliant.**

---

## AST Consistency: **PASS**

- No `// VERUS REWRITE` or `// VERUS DEVIATION` comments anywhere in `frame/` (grep confirmed).
- `ast_consistency.py` vs the true **pre-verification baseline** (commit `9591070fc`, before any
  spec was added): `✅ Consistent: 4 functions, 1 structs match`.
- Manual exec diff vs baseline: `from_raw_value` body (`if value > Self::MAX { return None } Some(Self(value))`),
  the `FrameNumber(usize)` struct, and `const MAX` are byte-for-byte unchanged. The only addition to
  `into_raw_value` is the ghost-only `proof! { use_type_invariant(self); }` (erased under normal
  `cargo build`); the exec `self.0` is unchanged.
- **No semantic mismatch.** Specs faithfully describe the unchanged exec behavior.

---

## Guardrails Compliance (exact counts, this module)

| Guardrail | Count | Verdict |
|-----------|------:|---------|
| `admit()` | 0 | OK |
| `assume(...)` | 0 | OK |
| `external_body` | 0 | OK |
| `assume_specification` | 0 | OK |
| `axiom` / `verifier::trusted` / `verifier::external` | 0 | OK |
| cfg-gated exec (`#[cfg(verus_keep_ghost)]`) | 2 — both are the standard `include!("number.spec.rs")` / `include!("number.proof.rs")` | OK (no gated exec logic) |
| `exec_allows_no_decreases_clause` | 0 | OK |

No `admit`, no `assume`. **No blockers.**

---

## Verification

Per orchestrator-provided authoritative data (not re-run here to avoid concurrent-build corruption):
`make verify-arch` → exit 0, **PASS**. Arch-wide cheating: assume=0, admit=0, trusted=0,
no_decreases=0, cfg_gate=0, external_body=3 (all in TCB list, none in this module). spec_drift vs
HEAD = 0; fn_coverage = 4/4 exec fns. **Verification: PASS.**

---

## Bug Summary

`bugs.md` does **not** exist for this module → 0 bugs recorded. I independently assessed whether any
real defect in the in-scope functions should have been recorded per the bug-reporting skill:

- `from_raw_value`: range-check-then-wrap is correct; boundary matches `spec_max()` exactly; `None`
  on overflow is honest rejection (no silent truncation). No bug.
- `into_raw_value`: `use_type_invariant(self)` is the correct mechanism to expose the bound; plain
  field read is value-preserving and total. No bug.
- `spec_max` `nat` cast: cannot underflow (`MAX_ADDRESS/FRAME_SIZE ≥ 1`). No bug.
- No `external_body` exists in the module, so none can be masking a defect.

**Conclusion: correctly 0 bugs.** Recording "None" (or leaving `bugs.md` absent) is appropriate.

---

## Issues (highest priority first)

**No blockers.** Only minor/cosmetic notes:

1. **(Minor, doc lag)** `view_design.md` still describes the *rejected* design (`uninterp spec_max`
   + `assume_specification[FrameNumber::MAX]`). The shipped spec uses the stronger, no-trust
   `open spec fn spec_max()` with no `assume_specification`. The shipped code is *better*; the design
   doc should be updated to match to avoid confusing future auditors. Documentation-only.

2. **(Minor, style)** `spec_max()` is an extra `pub spec fn` on `impl FrameNumber` beyond `inv`/`view`
   (spec-design discourages this). Justified by the primitive `int` View and downstream reachability;
   documented. No action required.

3. **(Informational)** `NULL` (associated const) carries `ensures Self::NULL@ == 0` and is technically
   outside the named in-scope set. Its type invariant is trivially discharged (`0 ≤ 0 ≤ spec_max()`,
   `spec_max() ≥ 0` as `nat`) and the ensures is correct. No concern.

4. **(Informational)** The exec file uses attribute-style annotations rather than `verus!{}` blocks —
   the repo-wide convention for in-place exec files. Non-blocking.

---

## Result: **PASS**

**Justification:** Every checklist dimension is satisfied with zero blockers.
- Spec quality: contracts are correct, bidirectional, non-tautological, declarative, and the shipped
  `spec_max` is a *stronger* (trust-free) realization than the design doc proposed; `spec_max ↔ MAX`
  binding and the `nat` cast are independently verified sound.
- Caller coverage: 4/4, none missing.
- Proof completeness: 0 `admit`, 0 `external_body` in module; `use_type_invariant` used correctly.
- TCB: all 3 arch `external_body` are outside this module and listed in `tcb-allowed.md`.
- AST consistency: PASS against the true pre-spec baseline; exec byte-for-byte unchanged save a
  ghost-only `proof!` block; no `// VERUS REWRITE`, no semantic mismatch.
- Guardrails: admit=0, assume=0, external_body=0, assume_specification=0, cfg-gated exec=0.
- Verification: `make verify-arch` PASS (authoritative).
- Bugs: correctly 0 — no masked defect, no recordable bug in scope.

The only findings are documentation/style minutiae that do not affect soundness or completeness.
