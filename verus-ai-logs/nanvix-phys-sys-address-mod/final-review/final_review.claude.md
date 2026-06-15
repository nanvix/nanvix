# Final Verification Review — `sys-address-mod`

**Module:** `src/libs/sys/src/sys/mm/address/mod.rs` (`pub trait Address`)
**In-scope target functions:** `is_aligned`, `into_raw_value`, `from_raw_value`
(all three are **trait method declarations** with no executable bodies — bodies
live in out-of-scope implementors `VirtualAddress` / `PhysicalAddress` /
`PageAligned<T>` / `PageTableAligned<T>`).
**Branch:** `verus-ai-prove`
**Reviewer:** independent strict final verification (tools re-run, prior claims not trusted).

---

## Spec Quality

The three in-scope contracts are external-top API contracts on the trait. Each
was assessed against the spec-design skill.

- **`from_raw_value(raw_addr: usize) -> Result<Self, Error>`**
  ```
  ensures match result {
      Ok(a)  => a@ == raw_addr as int,                       // round-trip
      Err(e) => e.code == crate::error::ErrorCode::BadAddress // error code pinned
  }
  ```
  - Uses a single `match` over the result → both arms complete by construction
    (avoids the "Separate Ok/Err" anti-pattern). 
  - Success arm gives the round-trip fact callers need (`a@ == raw`).
  - Error arm is a meaningful error path (pins `ErrorCode::BadAddress`), not a
    one-sided / tautological spec.
  - **Deliberate weakening of the error arm:** it pins only the *error code*, not
    a bidirectional `Err ⇔ raw > max_addr` range predicate. This is justified
    (see Caller Coverage) — `PhysicalAddress::from_raw_value` rejects sparse but
    in-range addresses, so a uniform range predicate would be an *untruthful*
    contract. Per spec-design (dynamic/per-implementor validity → keep the `Err`
    arm, do not turn it into a `requires`/range predicate), the drop is correct.

- **`into_raw_value(self) -> usize`**
  ```
  ensures result as int == self@
  ```
  Exact, total, lossless projection. Declarative, caller-facing, no
  implementation leakage. Correct and complete for a trivial projection.

- **`is_aligned(&self, align: Alignment) -> Result<bool, Error>`**
  ```
  ensures result matches Ok(aligned) && aligned == spec_addr_is_aligned(self@, align)
  ```
  - `result matches Ok(aligned)` additionally encodes a **liveness** guarantee
    (valid `Alignment` ⇒ never `Err`), which matches the caller analysis ("`Err`
    reserved for genuinely invalid alignments; current impls never error").
  - `spec_addr_is_aligned(v, align) := v % crate::mm::spec_align_value(align) == 0`
    is a concrete `pub open spec fn` (NOT `uninterp`), reusing the existing
    `spec_align_value` companion. Declarative — states *what* (divisibility),
    not *how* (bitmask).

**No** tautological or subsumed `ensures`. **No** `assume_specification` on
workspace-internal code (the only spec helper references the pre-existing
`crate::mm::spec_align_value`). The newly-added `spec_addr_is_aligned` helper is
bound to an exec contract (`is_aligned`), so it is not a floating spec.

**Verdict: PASS.**

---

## Caller Coverage

Source: `caller_analysis.md`. Enumerated caller expectations:

| # | Expectation | Origin | Bound contract | Status |
|---|-------------|--------|----------------|--------|
| 1 | `from_raw_value` round-trip `a@ == raw` | tests (build-then-check), `PageAligned`/`PhysicalAddress` round-trips | `Ok(a) => a@ == raw_addr as int` | ✅ Covered |
| 2 | `from_raw_value` `BadAddress` on out-of-range | kernel tests (`max_addr()+1` → `Err`), `?`-propagation in blanket impls | `Err(e) => e.code == ErrorCode::BadAddress` | ✅ Covered |
| 3 | `into_raw_value` lossless `result as int == self@` | `MemoryRegion::new`, `PageAligned`/`PhysicalAddress` round-trips | `result as int == self@` | ✅ Covered |
| 4 | `is_aligned` predicate `self@ % align == 0` | `PageAligned`/`PageTableAligned` construction | `Ok(aligned) && aligned == spec_addr_is_aligned(self@, align)` | ✅ Covered |

**Covered: 4 / 4.**

**Missing: none.**

**Intentionally-dropped property:** the bidirectional range arm
`Err ⇔ raw > spec_max_addr` (and the supporting `spec_max_addr` /
`spec_addr_valid` / `addr_wf` machinery). Documented in `view_design.md`
(§"Specification-phase update"). Drop is **justified** on two grounds, both
sanctioned by spec-design:
1. **Untruthful across implementors** — `PhysicalAddress::from_raw_value`
   validates via `is_valid_physical_address` (sparse memory) and can reject
   in-range `raw`, so `Err ⇔ raw > max_addr` is false. Dynamic/per-platform
   validity is not a uniform caller-visible predicate → keep the `Err` arm.
2. **Out of scope** — surfacing `spec_max_addr::<T>()` would force a new trait
   spec method or an `ensures` on the out-of-scope `max_addr`, violating "do not
   touch unlisted functions".

The retained `Err` arm (error-code pinning) still satisfies what callers rely on
(`?`-propagation of `BadAddress`). The supertrait `Ord`/`Eq` agreement with `@`
is a property of the `int` view itself, needing no extra contract.

**Verdict: PASS** (all expectations covered or justifiably dropped).

---

## Proof Completeness

- **`admit()` count: 0** (locations: none in any of the 3 module files).
- **`external_body` count: 0** (locations: none in any of the 3 module files).

The three target functions are bodiless trait method declarations, so there are
no proof obligations of their own — no proof bodies, no loop invariants, hence no
`admit()` placeholders to eliminate. `mod.proof.rs` is an empty `verus! { }`
shell.

**Verdict: PASS** (0 admit — no blocker; 0 external_body — no blocker).

---

## TCB Compliance

No `external_body` exists in any of the 3 module files, so there is nothing to
reconcile against `tcb-allowed.md`. No new trust boundary introduced. The
TCB allow-list contains no entry for this module — and none is required.

**Verdict: PASS.**

---

## Guardrails Compliance (exact counts across the 3 module files)

| Dimension | Count | Notes |
|-----------|-------|-------|
| `admit` | **0** | — |
| `assume` | **0** | — |
| `external_body` | **0** | — |
| `assume_specification` | **0** | — |
| cfg-gated **exec** | **0** | the 2 `#[cfg(verus_keep_ghost)]` lines in `mod.rs` (lines 9, 11) wrap `include!("mod.spec.rs")` / `include!("mod.proof.rs")` — they gate spec/proof *includes*, not exec code. Correctly classified as non-exec. |

`admit == 0` and `assume == 0` → no blocker. Crate-wide cheating scan
(`verify-sys`) independently reports `cfg_gate=0` for the `sys` crate.

**Verdict: PASS.**

---

## AST Consistency

`python3 ast_consistency.py --base-ref verus-ai-prove src/libs/sys/src/sys/mm/address/mod.rs summary`

```
Consistent: ✅ YES (matched=0 mismatched=0 missing=0 extra=0)
```

`matched=0` is expected — the file contains only trait method *declarations*
(no exec bodies for the tool to hash). `mismatched=0`, `missing=0`, `extra=0`.

No `// VERUS REWRITE` comments present anywhere in the 3 files (none needed; no
exec bodies). The only source-line change (diff vs `192f966ee^`) besides
annotations is the de-duplication of the redundant `use ::vstd::prelude::*;`
import (replaced by the conventional `use vstd::prelude::*;` placed with the
spec/proof includes) — an import normalization, semantically inert, documented
in `view_design.md`, and invisible to the AST exec hash. `spec_drift.py` reports
**0 contract drift** (0 ensures removed, 0 requires added).

**Verdict: PASS.**

---

## Verification

`cd /home/ruize/nanvix-phy-specs && make verify-sys` (forced fresh recompile):

```
verification results:: 6 verified, 0 errors
  Exit code : 0
  ✅ No cheating detected.
  status: CLEAN
```

**Cheating-pattern summary line:**
```
cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
```

0 errors, status CLEAN, all cheating dimensions zero.

**Verdict: PASS.**

---

## Bug Summary

`bugs.md` records **None**, with the rationale that all three targets are
bodiless trait declarations carrying only contracts (no proof obligations). I
reconciled this against the final code:

- No `admit()` / `assume()` / `external_body` survive (confirmed by grep + the
  fresh `verify-sys` cheating scan) → no hidden/unproven obligations to classify.
- Each contract is satisfiable by every implementor; `PhysicalAddress`'s stricter
  sparse validity only narrows the open `Err` arm → no contradiction → no
  Context-Dependent or True-Bug finding.
- The one incidental exec change (removing the duplicate `vstd` import that broke
  the non-Verus `cargo build` under `warnings = "deny"`) is documented in
  `view_design.md`; it is a build-hygiene fix, not a logic bug, and leaves the
  AST exec hash unchanged.

No undocumented bugs found. `bugs.md` is valid as written.

**Verdict: PASS** (no surviving failures to classify).

---

## Issues (highest priority first)

None. No blockers, no minor issues.

(Observation, non-blocking) The `is_aligned` contract asserts `result` is always
`Ok` — a liveness strengthening beyond the bare type. It is truthful for all
known implementors (a validated `Alignment` enum is never invalid), matches the
caller analysis, and any implementor that could `Err` would simply fail to verify
against the trait contract. No action required.

---

## Result: PASS
