# Final Verification Review — `hal::platform::microvm` (`gva_to_gpa`)

**Reviewer:** Independent strict final verification (Claude / Copilot CLI)
**Date:** 2026-06-15
**Scope:** ONLY `gva_to_gpa` in `src/kernel/src/hal/platform/microvm/`
**Branch:** `verus-ai/hal-platform-microvm`

---

## Checklist

- [x] Spec quality acceptable per spec-design (external-top contract correct & complete)
- [x] Caller coverage complete (5/5 caller expectations mapped to spec/ensures/lemma)
- [x] Zero `admit()` in scope
- [x] Zero `assume(...)` in scope
- [x] Zero unlisted `external_body` in scope (zero `external_body` at all in scope)
- [x] TCB compliance — no NEW trust boundary introduced for in-scope code
- [x] AST consistency PASS (all 28 functions MATCH, incl. `gva_to_gpa`)
- [x] No `// VERUS REWRITE` comments present
- [x] Verification CLEAN / exit 0
- [x] Spec not weakened (spec_drift: 0 contract drift)
- [x] No exec code mutation (diff is additive-only: +96 lines, 0 deletions)
- [x] Bugs reconciled (no bugs.md; no bugs found)

**No blockers found.**

---

## Spec Quality

The external-top contract on `gva_to_gpa` is correct, complete, and understandable.

```rust
#[verus_spec(result =>
    ensures
        result == gva,                                                   // identity (directly usable)
        result as nat == (MicrovmTranslationView {}).spec_gva_to_gpa(gva as nat),  // View vocabulary
)]
#[inline(always)]
pub fn gva_to_gpa(gva: usize) -> usize { gva }
```

- **Bound to exec code:** ✅ Both `ensures` clauses are on the real exec function (annotated in place; no copy).
- **Sufficient to reject bugs:** ✅ `result == gva` pins identity exactly; a buggy non-identity body would fail.
- **Declarative & simpler than code:** ✅ `spec_gva_to_gpa` is the identity map abstraction; `injective()` is a `forall` over the map, higher-level than the body.
- **Written for the caller:** ✅ `result == gva` is the directly-usable fact `book_mmio_regions` needs; the `spec_gva_to_gpa`/`injective()` forms tie into the View vocabulary for downstream proofs.
- **Totality:** ✅ No `requires` — total for every `usize`, mirroring the no-panic exec function.

### `nat` vs `usize` assessment (requested)

The spec models addresses as `nat` (`spec_gva_to_gpa(self, gva: nat) -> nat`), while spec-design point 7 and view_design both say to **prefer `usize`** for pointer addresses. **Assessment: acceptable, not a real issue.**

Rationale:
1. The **strong, directly-usable caller fact is stated in `usize`**: `result == gva`. The `nat` clause is purely the secondary "View vocabulary" tie-in, not the load-bearing fact.
2. The conversion `gva as nat` is **lossless** (usize → nat is exact and total), so the `nat` model introduces no soundness gap and cannot under- or over-approximate the identity.
3. Identity is injective over all of `nat`, so the `injective()` property is if anything more general than a `usize`-bounded statement — no caller-relevant weakening.
4. The view_design explicitly documents the choice ("addresses are modeled as `nat` — raw, non-negative machine addresses live in spec world") as deliberate.

This is a minor stylistic deviation from the "prefer `usize`" guidance, but because the binding caller contract is expressed in `usize` and the `nat` lift is exact, it does not affect correctness, completeness, or caller usability. **Not a blocker; not even a required fix.**

---

## Caller Coverage — 5/5

Single call site: `mm/phys/mod.rs:128` in `book_mmio_regions`
(`let mmio_addr: usize = crate::hal::platform::gva_to_gpa(start);`). No trait obligations (free function).

| # | Caller expectation (caller_analysis.md) | Covered by | Status |
|---|------------------------------------------|------------|--------|
| 1 | **Totality** — returns for any `usize`, never panics | No `requires` on the exec contract (unconditional `ensures`); `spec_gva_to_gpa` is a total spec fn | ✅ |
| 2 | **Purity / determinism** — result depends only on `gva` | `result as nat == spec_gva_to_gpa(gva as nat)` ties result to a pure spec function (deterministic by construction) | ✅ |
| 3 | **Identity (`result == gva`)** — address-preserving | `ensures result == gva` | ✅ |
| 4 | **Injectivity** — distinct inputs → distinct frames (no aliasing/double-booking) | `MicrovmTranslationView::injective()` + `lemma_translation_injective` (proof.rs, discharged, no admit) | ✅ |
| 5 | **Valid address encoding** — output acceptable to `from_raw_value`/`from_mmio_address` | Identity over `usize` ⇒ output occupies the same representable `usize` range as input; `result == gva` makes this structural. Enforced at the caller boundary (documented in view_design "Note on valid address encoding"). | ✅ |

**Missing: none.** Coverage filtering of tracked RAM (`frame::is_covered`) is correctly *out of scope* (explicitly the caller's responsibility per caller_analysis.md), so it is rightly not modeled.

---

## Proof Completeness

- **`admit()` count in scope: 0.** The only textual match is a comment in `mod.proof.rs:10` ("Body is `admit()` during the specification phase…") — no actual `admit()` call. `lemma_translation_injective` has an **empty body** and is discharged directly from the `open` identity definition of `spec_gva_to_gpa`.
- **`external_body` count in scope: 0.** Confirmed via grep over all three files (`mod.rs`, `mod.spec.rs`, `mod.proof.rs`): zero occurrences.
- Locations checked: `mod.rs` (gva_to_gpa attribute only), `mod.spec.rs` (View + spec fns), `mod.proof.rs` (injectivity lemma).

No BLOCKERS.

---

## TCB Compliance

`gva_to_gpa` is **NOT** in `tcb-allowed.md`, and correctly carries **no** `external_body`/`assume_specification`/`axiom`. No new trust boundary was introduced for in-scope code. The crate-wide `external_body=25` reported by the cheating summary are entirely pre-existing/out-of-scope (the pre-approved TCB in `mm/phys`, `bump_allocator`, `hal/mem`), none in the microvm module. ✅

---

## Guardrails Compliance (in-scope module, exact counts)

| Metric | In-scope count | Notes |
|--------|---------------:|-------|
| `admit()` | **0** | 1 textual hit is a comment only |
| `assume(...)` | **0** | "assume" hits in `mod.rs` are doc-comment prose ("It assumes the stdout device…") |
| `external_body` | **0** | grep over all 3 files |
| `assume_specification` | **0** | none |
| cfg-gated **exec** | **0** | The only `#[cfg(verus_keep_ghost)]` uses are `include!("mod.spec.rs")`/`include!("mod.proof.rs")` — the standard spec/proof inclusion pattern, not exec gating. `use vstd::prelude::*;` is correctly **un-gated**. |

`admit > 0` or `assume > 0` would be a BLOCKER — neither present. ✅

---

## AST Consistency — PASS

`python3 scripts/ast_consistency.py src/kernel/src/hal/platform/microvm/mod.rs` → all 28 functions report **MATCH**, including `gva_to_gpa`. Exit 0. No MISMATCH. No `// VERUS REWRITE` comments anywhere in the module. Git diff of `mod.rs` confirms the exec body (`gva`) is byte-for-byte unchanged; only the `#[verus_spec]` attribute, the `use vstd::prelude::*;`, and the cfg-gated `include!`s were added (+16 lines, 0 deletions).

---

## Verification — PASS

`make verify-kernel MODULE=hal::platform::microvm` → **status: CLEAN, exit 0.**

```
verification: cached (no recompilation), — (exit 0)
cheating: assume=0 external_body=25 admit=0 trusted=0 no_decreases=0 cfg_gate=7
coverage: 1/31 exec functions have contracts
status: CLEAN
```

The `external_body=25` / `cfg_gate=7` are **crate-wide** and pre-existing/out-of-scope (verified above: the three in-scope files add 0 of each). In-scope `assume=0`, `admit=0`, `trusted=0`. Coverage `1/31` is expected — only `gva_to_gpa` is in scope.

**spec_drift (`git-diff … --before HEAD`):** 0 functions changed, 0 contract drift, 0 ensures removed, 0 requires added → **no spec weakening.** ✅

---

## Bug Summary

`bugs.md` does **not** exist for this target. No bugs were found during this review: `gva_to_gpa` is a trivially-correct identity function whose contract (`result == gva`) is proven directly, and no caller expectation is violated. **No bugs found, and none to reconcile.**

---

## Issues (highest priority first)

1. *(Minor / informational, non-blocking)* Spec models addresses as `nat` rather than the spec-design-preferred `usize`. Acceptable because the binding caller fact (`result == gva`) is in `usize` and the `gva as nat` lift is exact (see Spec Quality). No action required.

No correctness, completeness, TCB, AST, or verification issues.

---

## Result: **PASS**

All PASS criteria satisfied:
- Zero `admit()`, zero `assume(...)`, zero unlisted (in fact zero) `external_body` in scope.
- AST consistency PASS; no exec mutation (additive-only diff).
- Verification CLEAN / exit 0; no spec weakening (spec_drift clean).
- All 5/5 caller expectations covered (identity, totality, purity/determinism, injectivity, valid encoding).
- No bugs outstanding.
