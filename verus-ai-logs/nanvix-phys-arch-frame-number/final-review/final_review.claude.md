# Final Review — `arch-frame-number` (independent, strict)

- **Module:** `arch::x86::mem::paging::frame::number`
- **Reviewer:** Independent final review (Claude)
- **Date:** 2026-06-15
- **Branch:** `verus-ai-prove` (base == verus branch)
- **Review type:** Read-only. No source/spec/proof code was modified.

## Files & in-scope targets

- Source: `src/libs/arch/src/x86/mem/paging/frame/number.rs`
- Spec:   `src/libs/arch/src/x86/mem/paging/frame/number.spec.rs`
- Proof:  `src/libs/arch/src/x86/mem/paging/frame/number.proof.rs`

In-scope items ONLY: `FrameNumber::from_raw_value`, `FrameNumber::into_raw_value`,
and the type `FrameNumber` (`View::view`, `inv`, `spec_max`). Out of scope:
`NULL`/`MAX` consts and the two unit tests (confirmed untouched — AST MATCH, see §5).

---

## Task 1 — Spec Quality (external-top API contracts)

### `from_raw_value(value: usize) -> Option<Self>` (number.rs:56–68)

```
value as int <= Self::spec_max() ==> (result is Some && (result->Some_0)@ == value as int),
value as int >  Self::spec_max() ==> result is None,
```

- **Bidirectional / complete error path.** The two implications partition the
  entire input domain on the abstract predicate `value <= spec_max()`. Each side
  maps to a *definite* result (`Some`/`None`), so this is not a one-sided error
  spec. Liveness is satisfied: when `value <= spec_max()` the function *must*
  return `Some` (no spurious `None`).
- **Success carries the value.** `(result->Some_0)@ == value as int` pins the
  abstract index to the input — this is the round-trip half the callers depend on.
- **Declarative, not operational.** It states *what* (in-range ⇒ accept and
  preserve; out-of-range ⇒ reject), independent of the range-check-and-wrap body.
  It could be written from the signature + module purpose alone (passes the
  independence test).
- **No tautology / no subsumption.** Neither clause is trivially true nor implied
  by the other.
- **Style note (non-blocking):** expressed as two `==>` clauses keyed on the
  bound rather than a `match result { Some/None }`. Because the two predicates
  are mutually exclusive and exhaustive and both arms are explicit, completeness
  is preserved; a `match` form would be marginally more idiomatic but is not
  required. The `is Some`/`is None` partition is the "bidirectional failure
  condition expressed as an abstract predicate" recommended by the error-path
  principles.

**Verdict: OK.**

### `into_raw_value(self) -> usize` (number.rs:79–87)

```
result as int == self@,
0 <= self@ <= Self::spec_max(),
```

- **Value-preserving total projection.** `result as int == self@` is the exact
  round-trip identity callers need; totality is structural (returns `usize`
  unconditionally, no `Option`, no panic path).
- **The bound clause is necessary, not redundant.** Although `0 <= self@ <=
  spec_max()` is literally `inv()`, a consumed `self`'s type invariant is **not**
  automatically available to callers — it must be surfaced in `ensures`. This is
  the single most important caller guarantee (overflow-safe `result << FRAME_SHIFT`
  per caller_analysis L66–68/L93–95). Restating it here is correct and required,
  not an anti-pattern "subsumed property".
- The ghost `proof! { use_type_invariant(self); }` (number.rs:85) is the correct
  idiom to bring `inv()` into scope to discharge the bound; it is erased in normal
  builds.

**Verdict: OK.**

### Type `FrameNumber` — View + inv (number.spec.rs)

- `view(&self) -> int` is `closed` (hides the `usize` field), abstract value =
  frame index as `int`. Matches the sibling address-tower views. **OK.**
- `inv(): 0 <= self@ <= Self::spec_max()` as `#[verifier::type_invariant]` — the
  minimal universal property every caller relies on. **OK.**
- `spec_max()` is `pub open spec fn` with a **concrete** definition
  `(mem::MAX_ADDRESS / mem::FRAME_SIZE - 1) as nat` (see §"Design soundness").

**Task 1 verdict: OK (no blockers).**

---

## Task 2 — Caller Coverage

Mapping every expectation in `caller_analysis.md` to a contract:

| # | Caller expectation (caller_analysis.md) | Contract that covers it | Status |
|---|------------------------------------------|--------------------------|--------|
| 1 | `from_raw_value` returns `Some` exactly when `value <= MAX` (L46–47) | from_raw_value clause 1 (`<= spec_max ==> Some`) | Covered |
| 2 | `from_raw_value` returns `None` for `value > MAX`, never silent truncation (L48–50) | from_raw_value clause 2 (`> spec_max ==> None`) | Covered |
| 3 | `from_raw_value` success round-trips the value (L51–53, L96) | `(result->Some_0)@ == value as int` | Covered |
| 4 | `into_raw_value` totality (never panics, no `Option`) (L69, L98) | return type `usize`, no `requires`, unconditional `ensures` | Covered |
| 5 | `into_raw_value` value-preserving / round-trip identity (L63–64, L96) | `result as int == self@` | Covered |
| 6 | `into_raw_value` result bounded by `MAX` ⇒ overflow-safe `<< FRAME_SHIFT` (L66–68, L93–95) | `0 <= self@ <= Self::spec_max()` | Covered |
| 7 | Type invariant: every `FrameNumber` ∈ `0..=MAX` (L74–79, L91) | `#[verifier::type_invariant] inv()` | Covered |

Derived round-trip identity `from_raw_value(v).map(into_raw_value) == Some(v)` for
`v <= MAX` and `== None` for `v > MAX` follows from #1/#3/#5 — covered without a
separate clause.

**Covered: 7 / 7. Missing: none.**

**Task 2 verdict: OK (no blockers).**

---

## Task 3 — Proof Completeness

In-scope files (`number.rs`, `number.spec.rs`, `number.proof.rs`):

- `admit()` occurrences: **0**
- `external_body` occurrences: **0**

(`number.proof.rs` is `verus! { }` — empty; no proof obligations remain because
the contracts are discharged in-body / by the type invariant.)

ANY in-scope `admit()` would be a blocker — there are none. ANY in-scope
`external_body` not in tcb-allowed.md would be a blocker — there are none.

> Crate-wide, `make verify-arch` reports `admit=1 external_body=3`, but the
> cheating-detail attributes **all** of them to OTHER modules
> (`paging/table.proof.rs:8 lemma_entry_roundtrip: admit`,
> `paging/mod.rs:80 invlpg`, `paging/table.rs:209 read`, `:246 write`) — none in
> the frame/number files. See §6/§7.

**Task 3 verdict: OK (no blockers).**

---

## Task 4 — TCB Compliance

The in-scope files contain **0** `external_body` functions, so there is nothing to
reconcile against `tcb-allowed.md` — vacuously compliant.

Note: `tcb-allowed.md` lists kernel-side placeholders
(`::arch::mem::paging::FrameNumber` `external_type_specification ExFrameNumber`,
`FrameNumber::from_raw_value` / `into_raw_value` `assume_specification`) — these
live in the **kernel** crate as temporary downstream contracts that *this* arch
verification supersedes. They are not in-scope arch files and impose no obligation
here.

**Every in-scope external_body is allowed: YES (count = 0).**

**Task 4 verdict: OK (no blockers).**

---

## Task 5 — AST Consistency

Tool: `scripts/ast_consistency.py --base-ref dev` (the `dev` branch holds the
original pre-Verus source; bodies confirmed identical by inspection).

```
✅ Consistent: 4 functions, 1 structs match.
FrameNumber::from_raw_value           MATCH
FrameNumber::into_raw_value           MATCH
test_frame_number_from_raw_value_max  MATCH
test_frame_number_from_raw_value_zero MATCH
FrameNumber (struct)                  MATCH
Consistent: ✅ YES (matched=4 mismatched=0 missing=0 extra=0)
```

- The only exec-adjacent addition is the **ghost** `proof! { use_type_invariant(self); }`
  in `into_raw_value`; it is stripped by the checker and erased in normal builds —
  exec body is byte-equivalent to the original `self.0`.
- `// VERUS REWRITE` comments: **none present** — nothing to inspect for semantic
  equivalence. No exec rewrites were performed.

**Task 5 verdict: PASS (no MISMATCH).**

---

## Task 6 — Verification

Command: `cd /home/ruize/nanvix-phy-specs && make verify-arch`

```
Exit code : 0
verification: cached (no recompilation), — (exit 0)
status: CHEATING_DETECTED   <-- crate-wide counters; all instances out-of-scope (see §7)
```

- Exit code **0** → verification PASSES.
- Result was served from cache because the working tree equals the verified commit
  (no source modifications). The committing run recorded
  `arch::all (48 verified, 0 errors, ...)` — **0 errors**.
- The `CHEATING_DETECTED` banner is crate-wide; the four flagged sites are all in
  the `paging/table` and `paging/mod` modules, none in frame/number (§7).

**Error count (in-scope and crate): 0.**

**Task 6 verdict: PASS.**

---

## Task 7 — Guardrails Compliance (cheating dimensions, in-scope files only)

| Dimension | Count (in-scope) | Locations |
|-----------|-----------------:|-----------|
| `admit()` | **0** | — |
| `assume(...)` | **0** | (the word "assumed" at number.spec.rs:25 is a comment only) |
| `external_body` | **0** | — |
| `assume_specification` | **0** | — |
| cfg-gated **exec** code | **0** | the two `#[cfg(verus_keep_ghost)]` at number.rs:9,11 gate `include!` of the `.spec.rs`/`.proof.rs` ghost files — the standard, allowed pattern; they do NOT gate any exec branch/expr |
| `uninterp spec fn` | **0** | ("uninterp" appears only in comments at number.spec.rs:11,24) |

`admit > 0` or `assume > 0` would be a blocker — both are **0**.

**Task 7 verdict: OK (no blockers).**

---

## Task 8 — Bug Reconciliation

- `bugs.md` for this module: **confirmed absent** (`find ... -name bugs.md` → none).
  No bugs were recorded.
- **Is there a real defect that SHOULD have been recorded?** No.
  - `from_raw_value`: `if value > Self::MAX { return None; } Some(Self(value))` — a
    correct, total range check; `MAX = MAX_ADDRESS/FRAME_SIZE - 1` is a compile-time
    constant that does not underflow for the real architecture constants. No overflow,
    off-by-one, missing-bounds, or unchecked-cast issue.
  - `into_raw_value`: pure field read bounded by `inv()`. No defect.
- Per the bug-reporting skill, the correct content for a bugs file would be `"None"`.
  The file is simply absent rather than containing `"None"` — a minor process/log gap,
  **not** a code defect and **not** a blocker (no real bug exists to record).

**Task 8 verdict: OK (no blockers). Observation: bugs.md missing; should contain "None" for hygiene.**

---

## Task 9 — Spec Drift

Command: `spec_drift.py git-diff src/libs/arch/.../number.rs --before HEAD`

```
Functions with changes: 0
Contract drift (review required): 0  (ensures removed: 0, requires added: 0)
✅ No contract drift detected.   (exit 0)
```

Additional baseline check against the original pre-Verus source (`dev` branch):
the original `number.spec.rs`/`number.proof.rs` were empty and the two functions
had **no** specs (confirmed by caller_analysis.md L100–106). The Verus effort only
**added** `ensures` (and the type invariant) — pure strengthening. No `requires`
were added to the API (callers' burden unchanged), no `ensures` removed or relaxed.

**Original guarantees were not weakened.**

**Task 9 verdict: PASS (no blockers).**

---

## Design soundness — shipped `open spec_max()` vs `view_design.md`

`view_design.md` (L87–113) describes an **older** design:
`pub uninterp spec fn spec_max()` + a trusted `assume_specification[FrameNumber::MAX]`
binding the exec `MAX` to it.

The **shipped** design instead uses a concrete, interpreted definition:

```rust
pub open spec fn spec_max() -> nat { (mem::MAX_ADDRESS / mem::FRAME_SIZE - 1) as nat }
#[verifier::type_invariant] pub open spec fn inv(&self) -> bool { 0 <= self@ <= Self::spec_max() }
```

Assessment — **the divergence is sound and strictly better; it does not matter
adversely, it improves the result:**

1. **Stronger (not weaker).** `spec_max()` is `open` and interpreted directly
   from the same `mem::MAX_ADDRESS` / `mem::FRAME_SIZE` constants the exec `MAX`
   uses. The binding `MAX as int == spec_max()` is therefore **discharged by
   verification** (the proof passes), not **assumed** via `assume_specification`.
   One external-bottom trust assumption is eliminated.
2. **Required for guardrail compliance.** The doc's `uninterp spec fn spec_max()`
   is explicitly **banned** by the verus-constraints skill ("all spec functions
   must have concrete definitions"). The shipped `open` definition is the compliant
   realization; had the doc's design shipped verbatim it would itself be a
   guardrail violation. So the shipped code corrected the design.
3. **Verification confirms interpretability.** Because `make verify-arch` passes
   with the `open` definition, Verus can resolve the `mem::*` constants in spec
   context and prove both `from_raw_value`'s success/failure split and
   `into_raw_value`'s bound without any trusted MAX contract.
4. **Minor style note (non-blocking):** `spec_max()` is a `pub` spec fn on
   `impl FrameNumber` beyond `view`/`inv` (spec-design Part 3 #5 prefers helpers on
   the View type). Here the View is a bare `int` (cannot carry methods) and
   downstream crates reach the bound via the exported `FrameNumber` type, so this
   is the only sensible placement — acceptable.

**Action item (documentation only, non-blocking):** `view_design.md` is stale —
it still documents the `uninterp` + `assume_specification` approach. It should be
updated to reflect the shipped `open` definition. This is a doc lag, not a code
or soundness issue.

---

## Summary of verdicts

| Task | Item | Verdict |
|------|------|---------|
| 1 | Spec quality (from/into/View/inv) | OK |
| 2 | Caller coverage (7/7) | OK |
| 3 | Proof completeness (admit=0, external_body=0 in-scope) | OK |
| 4 | TCB compliance (0 in-scope external_body) | OK |
| 5 | AST consistency (4 fns + 1 struct MATCH; no REWRITE) | PASS |
| 6 | Verification (`make verify-arch` exit 0, 0 errors) | PASS |
| 7 | Guardrails (all dimensions 0 in-scope) | OK |
| 8 | Bug reconciliation (no real defect; bugs.md absent) | OK |
| 9 | Spec drift (0 drift; only strengthening) | PASS |

**Blockers found: 0.**

Non-blocking observations:
- `bugs.md` is absent; for hygiene it should contain `"None"`.
- `view_design.md` is stale (documents the superseded `uninterp` + `assume_specification`
  design); update it to the shipped `open spec_max()`.

RESULT: PASS
