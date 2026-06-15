# Final Verification Review — `sys::mm::address::virt` (`VirtualAddress`)

- **Reviewer:** Independent strict final verification (review-only; no source/spec/proof files modified)
- **Date:** 2026-06-15
- **Branch:** `verus-ai/sys-virt-address` (base `verus-ai/hal-platform-microvm`), HEAD `ec800a83f`
- **Module files:**
  - `src/libs/sys/src/sys/mm/address/virt.rs`
  - `src/libs/sys/src/sys/mm/address/virt.spec.rs`
  - `src/libs/sys/src/sys/mm/address/virt.proof.rs`
- **In-scope functions:** the type `VirtualAddress` (its `View` + `inv`),
  `VirtualAddress::new`, the **inherent** `VirtualAddress::from_raw_value`, and
  `<VirtualAddress as Address>::into_raw_value` (body-verification blocked by a
  documented Verus front-end limitation; held via a consumer-side
  `assume_specification`).

---

## Checklist

- [x] **1. Spec quality** — contracts for `new`, inherent `from_raw_value`, `into_raw_value` identity, `inv`, `view` are correct, complete, declarative; no tautological/one-sided/missing-error-path issues; closed `view` / open `inv` correct.
- [x] **2. Caller coverage** — every caller expectation (round-trip identity, from_raw_value identity, into_raw_value purity, Ord/Eq agreement) is covered by a spec or justified. Covered 4/4.
- [x] **3. Proof completeness** — `admit()` = 0, `external_body` = 0 across all three module files.
- [x] **4. TCB compliance** — no `external_body` in the module; no NEW trust boundary introduced inside the module. The `into_raw_value` boundary is a consumer-side `assume_specification` recorded in `tcb-allowed.md`.
- [x] **5. AST consistency** — PASS (18 functions + 1 struct MATCH, 0 mismatch). No `// VERUS REWRITE` comments exist.
- [x] **6. Verification** — `make verify-sys` exit 0, status CLEAN. `make build` no-op target; `sys` crate compiles (cargo exit 0).
- [x] **7. Guardrails compliance** — admit=0, assume=0, external_body=0, assume_specification=0, cfg-gated exec=0. Only legitimate `cfg(verus_keep_ghost)` include-gating and `cfg(target_pointer_width)` platform conditionals present.
- [x] **8. Bug reconciliation** — `bugs.md` = "None" is accurate; `into_raw_value` non-verification correctly classified as a Verus limitation (not a bug).
- [x] **9. Spec drift** — vs base branch `verus-ai/hal-platform-microvm`: ✅ no contract drift (only strengthening: 1 function added). vs HEAD: tool false-positive (file byte-identical to HEAD; explained below).

---

## Spec Quality

**`View` (`virt.rs:333-340`)** — `type V = int; closed spec fn view(&self) -> int { self.0 as int }`.
- `closed` correctly hides the `VirtualAddress(usize)` newtype while exposing a usable `int`. ✅
- `type V = int` (rather than `usize`) is a deliberate, documented choice for cross-tower uniformity (`PhysicalAddress`/`PageAligned`/`FrameAddress`/`FrameNumber` all use `int`); the usize-ness is recovered in `inv()`. Per `spec-design` `usize` is *preferred* for addresses, but the tower-wide consistency rationale (view_design.md) is sound and not a defect. ✅

**`inv` (`virt.spec.rs:13-15`)** — `pub open spec fn inv(&self) -> bool { 0 <= self@ <= usize::MAX as int }`.
- Concrete definition (no `uninterp`). ✅
- `open` is correct — callers unfold the bound in arithmetic/round-trip proofs. ✅
- Weakest universally-true, caller-useful invariant; no spurious validity/alignment/page-index invariant invented (matches the "no extra state" caller analysis). ✅

**`new` (`virt.rs:53-60`)** — `ensures result@ == value as int, result.inv()`.
- Faithful-storage identity is the operative, non-tautological guarantee. ✅
- `result.inv()` is technically derivable from `result@ == value as int` + `value: usize` (anti-pattern #9 "subsumed"). This is a **deliberate, benign** convenience clause (view_design.md §new) so callers needing `inv()` do not re-derive it. Minor, not a defect.
- Infallible (returns `Self`); no error path required. ✅ `const fn` shape preserved.

**inherent `from_raw_value` (`virt.rs:71-78`)** — `ensures result@ == raw_addr as int, result.inv()`.
- Identical identity contract to `new`, matching the "interchangeable" caller expectation. ✅ Same minor `result.inv()` redundancy note.

**`into_raw_value` identity** — desired `ensures result as int == self@`.
- NOT body-verified in this module (documented Verus front-end limitation; see Verification/Bug sections). The identity is preserved as a consumer-side `assume_specification` in `src/kernel/src/hal/mem/types/address/phys.spec.rs:107-113` (`ensures result as int == addr@`), which is recorded in `tcb-allowed.md`. The contract is the correct caller guarantee, not a weakening.

**Error paths:** all three in-scope functions are infallible (`-> Self` / `-> usize`); the fallible trait `Address::from_raw_value` (always `Ok`) is out of scope. No missing error-path ensures. ✅

**Verdict:** Spec quality PASS (two minor, deliberate `result.inv()` redundancies — acceptable).

---

## Caller Coverage

Source: `caller_analysis.md`. Covered **4 / 4**.

| Caller expectation | Status | Where covered / justification |
|---|---|---|
| Round-trip `new(a).into_raw_value() == a` | ✅ Covered | `new` ensures `result@ == value as int`; `into_raw_value` (consumer assume_spec) ensures `result as int == self@`. Composition gives identity. |
| `from_raw_value(a).into_raw_value() == a` | ✅ Covered | inherent `from_raw_value` ensures `result@ == raw_addr as int`; same composition. |
| `into_raw_value` purity / non-consuming | ✅ Justified | `VirtualAddress: Copy` (type-system guarantee; `spec-design` says skip type-system guarantees). Return value is a pure functional equality on `self@`. |
| `Ord`/`Eq` agree with the integer | ✅ Justified | Derived `Ord/Eq/PartialOrd/PartialEq` (out of scope, AST-MATCH). `View = self.0 as int` makes `a < b <==> a@ < b@` expressible; agreement is a consequence, not an in-scope contract. |

**Missing:** none.

---

## Proof Completeness

Searched all three module files (`grep -nE "admit|assume\(|external_body|..."`):

- **`admit()` count: 0** (locations: none). Any `admit()` > 0 would be a BLOCKER — not present.
- **`external_body` count: 0** (locations: none — only descriptive comment text at `virt.rs:266` referencing the consumer-side boundary). Any `external_body` not in `tcb-allowed.md` would be a BLOCKER — not present.
- `virt.spec.rs` and `virt.proof.rs` contain no forbidden constructs (`virt.proof.rs` is an empty `verus! { }` block).

Confirmed independently by `verify.sh` cheating check: `admit=0 external_body=0`.

In-scope verified functions confirmed present (NOT in `coverage-unverified.txt`): `VirtualAddress::new`, inherent `VirtualAddress::from_raw_value`. `into_raw_value` is (expectedly) not body-verified here.

---

## TCB Compliance

- No `external_body` exists in the module → nothing in this module requires a `tcb-allowed.md` entry.
- **No NEW trust boundary is introduced inside this module.** `virt.rs` / `virt.spec.rs` / `virt.proof.rs` contain zero `assume_specification` / `external_body` / `axiom`.
- The `into_raw_value` trust boundary lives on the **consumer side** (`kernel`'s `phys.spec.rs:107-113`) and is recorded in `tcb-allowed.md` (line 266: `<VirtualAddress as Address>::into_raw_value` → `result as int == addr@`) and `verus-unsupported.md`. ✅

**Verdict:** TCB compliance PASS.

---

## Guardrails Compliance (exact counts)

Across `virt.rs`, `virt.spec.rs`, `virt.proof.rs`:

| Dimension | Count | Locations |
|---|---:|---|
| `admit()` | 0 | — |
| `assume(...)` | 0 | — |
| `external_body` | 0 | — (comment-only mention at `virt.rs:266`) |
| `assume_specification` | 0 | — (comment-only mention at `virt.rs:266`) |
| cfg-gated **exec** code | 0 | — |

Reported-but-legitimate conditionals (judged, not cheating):
- `#[cfg(verus_keep_ghost)]` at `virt.rs:9,11` — gates the `include!("virt.spec.rs")` / `include!("virt.proof.rs")` of **ghost spec/proof files only**. Standard, non-semantic; does not gate exec code. **Legitimate.**
- `#[cfg(target_pointer_width = "32")]` at `virt.rs:39` (a `static_assert::assert_eq_size!`) and `virt.rs:308` (`impl From<VirtualAddress> for u32`) — pre-existing platform conditionals, unchanged (AST-MATCH), out of scope. **Legitimate.**

Independent confirmation from `verify.sh`:
`cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0` → status CLEAN.

**Verdict:** Guardrails PASS. (admit=0, assume=0 → no BLOCKER; no external_body → no TCB BLOCKER.)

---

## AST Consistency (PASS)

`ast_consistency.py virt.rs summary` / `count` (auto-detect base on `verus-ai/*`):

```
Consistent: ✅ YES (matched=18 mismatched=0 missing=0 extra=0)
✅ Consistent: 18 functions, 1 structs match.   (exit 0)
```

All 18 functions and the `VirtualAddress` struct MATCH the base exec code; 0 mismatches. No `// VERUS REWRITE` / `VERUS BUG FIX` / `VERUS DEVIATION` comments exist in the module. Exec code is byte-faithful to the original after stripping ghost annotations.

`fn_coverage.py`: Source exec fns 15 / Verus exec fns 15 — Matched 15, Missing 0, Extra 0.

**Verdict:** AST consistency PASS.

---

## Verification

### `make verify-sys` — PASS (exit 0)

```
=== Results ===
  cached (no recompilation)
  Exit code : 0
=== Cheating Pattern Check ===
  ✅ No cheating detected.
=== Function Coverage ===
  2/254 exec functions have contracts.
=== Summary ===
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 2/254 exec functions have contracts
  status: CLEAN
VERIFY_EXIT=0
```

- **Exact error count: 0.** Status CLEAN, no cheating.
- The 2/254 verified functions are exactly the in-scope body-verified targets `VirtualAddress::new` and inherent `VirtualAddress::from_raw_value` (confirmed absent from `coverage-unverified.txt`). `into_raw_value` is intentionally not body-verified here (held consumer-side). The remaining 252 are out-of-scope `sys` functions.

### `make build` — PASS

`make build` resolves to a target with no recipe (`make: Nothing to be done for 'build'`; the project's real aggregate target is `all` / `./z`, exit 0). The relevant build check — that the verus-annotated exec code still compiles under a normal (non-Verus) build — was confirmed by building the `sys` crate:

```
src/libs/sys$ cargo build → Finished `dev` profile ... (exit 0)
```

**Verdict:** Verification PASS; build PASS.

---

## Spec Drift

### vs base branch `verus-ai/hal-platform-microvm` (authoritative — top-level entry specs are always compared to base)

```
- Functions with changes: 3
- Contract drift (⚠ review required): 0
- Functions added: 1
✅ No contract drift detected.   (exit 0)
```

No original guarantee weakened. The only change is **strengthening** (new specs added on `new` / `from_raw_value`). ✅

### vs HEAD (tool false-positive — investigated and dismissed)

`spec_drift.py git-diff --before HEAD` reported "2× ensures removed" on `from_raw_value`. This is a **tool artifact**, not real drift:
- `git status --porcelain` shows the module file is **unmodified**; `git diff HEAD -- virt.rs` is **empty** (working tree byte-identical to HEAD).
- The module has **two** functions named `from_raw_value` — the inherent one (`virt.rs:76`, carries the specs) and the trait one (`virt.rs:188`, no specs, just wraps in `Ok`). The name-keyed differ conflates them, so it appears one "lost" its ensures.
- Since the file is identical to HEAD, no drift is possible. The authoritative base-branch comparison (above) is clean.

**Verdict:** Spec drift PASS — no original guarantee weakened.

---

## Bug Summary

`bugs.md` states **"None"** — confirmed accurate against the final code.

- `new`, inherent `from_raw_value`, and `into_raw_value` are pure, infallible newtype-identity operations and exactly match their caller-relied contracts (round-trip identity). No True Bug, no Context-Dependent issue, no False Positive.
- The single unverifiable item, `<VirtualAddress as Address>::into_raw_value`, is correctly classified as a **Verus front-end limitation** (whole-impl trait verification pulls the unsupported `usize as *const u8` casts of sibling `as_ptr`/`as_mut_ptr` into scope), **not a code bug**. It is documented in `verus-unsupported.md` and its identity contract preserved by the consumer-side `assume_specification` in `tcb-allowed.md`. Classification is correct. ✅

---

## Issues (priority order)

**Blockers:** none.

**Minor / informational:**
1. *(Minor, accepted)* `into_raw_value` is not body-verified inside `sys`; its `result as int == self@` identity is an unverified consumer-side `assume_specification` (`kernel/.../phys.spec.rs`). This is a documented, sanctioned Verus limitation (`verus-unsupported.md`, `tcb-allowed.md`), within the stated scope of this effort — not a review blocker, but the honest residual trust surface to be discharged when the `Address` trait becomes verifiable.
2. *(Cosmetic)* `result.inv()` in the `ensures` of `new` and inherent `from_raw_value` is logically subsumed by `result@ == value/raw_addr as int` plus `value/raw_addr: usize`. Retained deliberately as a caller convenience (documented in `view_design.md`); harmless.
3. *(Tooling note)* `spec_drift.py git-diff --before HEAD` yields a false "ensures removed" due to the two same-named `from_raw_value` symbols; the authoritative base-branch comparison is clean. No action needed on the code.

---

## Result: PASS

All nine checklist items pass. No `admit()`, no `assume`, no in-module `external_body`/`assume_specification`, no cfg-gated exec code; AST consistency MATCH; `make verify-sys` exit 0 / CLEAN; `sys` builds; no spec drift vs the base branch; `bugs.md` "None" is accurate. The sole residual trust surface (`into_raw_value` identity) is a documented, pre-approved Verus front-end limitation held on the consumer side and recorded in `tcb-allowed.md` — within scope and not a blocker.
