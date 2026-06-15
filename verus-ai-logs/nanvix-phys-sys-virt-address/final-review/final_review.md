# Final Comprehensive Review: sys-virt-address

> Consolidated from two independent sub-agent reviews:
> - `final_review.claude.md` (claude-opus-4.8) — Result: **PASS**
> - `final_review.codex.md`  (gpt-5.3-codex)   — Result: **FAIL** (caller-coverage dissent)
>
> Both agents agree on every mechanical/objective check (verification, cheating
> counts, AST consistency, TCB, spec drift, bugs). They diverge only on whether
> two caller expectations (`into_raw_value` purity, `Ord`/`Eq` agreement) count
> as "missing specs". Adjudication below resolves this in favor of **covered/
> justified** — both are out-of-scope/type-system-guaranteed, not in-scope
> functions lacking specs. **Consolidated result: PASS.**

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py`, 39/64/102/548 refs recorded in caller_analysis.md
- [x] Caller expectations (success + failure) documented for each pub function — all in-scope constructors/projection are infallible; no failure path exists
- [x] Abstract resource identified — thin infallible newtype over a pointer-sized `usize`
- [x] Pre-existing specs assessed (if any exist from upstream verification) — inline `View` skeleton present; reviewed and kept

### View Design
- [x] Every field passes the substitution test — single scalar `addr: int` survives any rewrite of the newtype
- [x] All caller-observable state represented — the one observable quantity (the address) is modeled
- [x] No implementation-specific fields — `closed view` hides the `usize`; only the abstract `int` is exposed
- [x] inv() encodes real constraints — `0 <= self@ <= usize::MAX` (the usize-range bound the projection/round-trip rely on)
- [x] Mathematical types used (int; addresses exception accepted) — `type V = int`, tower-wide consistency, usize-ness recovered in `inv()`

### Specification
- [x] Every in-scope exec function has requires/ensures — `new`, inherent `from_raw_value` body-verified with ensures; `into_raw_value` identity held consumer-side (documented limitation)
- [x] Caller coverage — every expectation covered or justified (see adjudication)
- [x] View consistency — specs reference `self@`/`inv()` and maintain the invariant
- [x] No tautological ensures — identity ensures are operative
- [x] No subsumed ensures — `result.inv()` is mildly subsumed but deliberate/documented (caller convenience); not unsound
- [x] Error paths have meaningful ensures — all in-scope functions infallible; N/A
- [x] No assume_specification for workspace-internal code — none in module
- [x] vstd searched before any assume_specification — N/A (no assume_specification in module)
- [x] Specs written for the caller — round-trip identities directly usable in caller proofs
- [x] Trait obligations satisfied — `Address` semantic contract (inverse-of-construction) matched
- [x] Spec completeness (advisory) — complete for infallible identity ops
- [x] Loop invariants — no loops in scope; N/A
- [x] No cheating on module's own functions — admit=0, assume=0, external_body=0, trusted=0
- [x] No specs weakened — `spec_drift.py` vs base branch: 0 contract drift (only strengthening)
- [x] Bug awareness — bugs.md = "None", confirmed accurate
- [x] Cross-module regression — `make verify-sys` CLEAN (crate-level run; out-of-scope fns untouched, AST-MATCH)
- [x] Verification — `make verify-sys` exit 0; `sys` crate builds (exit 0)

### Proving
- [x] No specs weakened — confirmed (base-branch comparison clean; `--before HEAD` artifact explained)
- [x] Zero remaining admit() — 0
- [x] Zero external_body unless listed — 0 external_body in module
- [x] Zero assume/assume_specification in module — 0 (line 266 is comment text only)
- [x] No cfg-gated exec code — only `cfg(verus_keep_ghost)` ghost includes + `cfg(target_pointer_width)` platform conditionals
- [x] Cheating audit — admit=0, external_body=0, assume=0, cfg-gated exec=0
- [x] Claimed Verus limitation has isolated reproducer — `into_raw_value` whole-impl limitation documented in verus-unsupported.md
- [x] Exec rewrites minimal/equivalent — 0 `// VERUS REWRITE` (no exec rewrites)
- [x] Cross-module regression — verify-sys CLEAN
- [x] Verification — `make verify-sys` 0 errors; build 0 errors

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code (only includes/derives/platform conditionals)
- [x] Zero external_body unless listed — none in module
- [x] AST consistency: zero mismatches (18 fns + 1 struct MATCH)
- [x] All exec rewrites have VERUS REWRITE comment — N/A (zero rewrites)
- [x] For each surviving external_body: listed in tcb-allowed.md — N/A (none in module)
- [x] No specs weakened — confirmed
- [x] Cross-module regression — verify-sys CLEAN
- [x] Verification — 0 errors, 0 warnings

### Bug Recording
- [x] bugs.md exists — present, states "None"
- [x] Each bug is a real code defect — N/A (no bugs)
- [x] Each bug entry has required fields — N/A
- [x] No external_body used to mask a code defect — confirmed (no external_body in module)
- [x] Bug entries include provenance — N/A

## Spec Quality
Public API specs are **correct, complete, and understandable** for a thin infallible
newtype identity abstraction:
- `View` (`virt.rs:333-340`): `type V = int`, `closed spec fn view = self.0 as int` —
  `closed` correctly hides the newtype; `int` is the documented tower-wide choice.
- `inv` (`virt.spec.rs:13-15`): `open`, `0 <= self@ <= usize::MAX as int` — weakest
  universally-true, caller-useful invariant; no spurious validity/alignment/page-index
  invariant invented.
- `new` (`virt.rs:53-60`) and inherent `from_raw_value` (`virt.rs:71-78`): identity
  `result@ == arg as int` + `result.inv()` — the operative, non-tautological round-trip
  half callers depend on.
- `into_raw_value` identity (`result as int == self@`) is the inverse half; not
  body-verifiable in-module (Verus front-end limitation) and held at the sanctioned
  consumer-side `assume_specification` (`kernel/.../phys.spec.rs:107-113`, allow-listed).

Only **two minor, deliberate** notes: `result.inv()` on the two constructors is
logically subsumed by `result@ == arg as int` + `arg: usize` — retained as a documented
caller convenience (view_design.md), benign. No tautological/one-sided/missing-error-path
defects.

## Caller Coverage
- **Covered/Justified: 4 / 4 caller expectations** (consolidated verdict)
- Adjudication of the codex dissent (which scored 4/6):
  1. **Round-trip `new(a).into_raw_value()==a`** — ✅ covered (new ensures ∘ consumer-side into_raw_value ensures).
  2. **`from_raw_value(a).into_raw_value()==a`** — ✅ covered (same composition).
  3. **`into_raw_value` purity / non-consuming** — ✅ **justified, not missing.** Purity is a
     type-system guarantee (`VirtualAddress: Copy`, by-value `self`); spec-design says to skip
     type-system guarantees. Its *value* identity is specced at the allow-listed consumer
     boundary (`tcb-allowed.md:266`). The Verus limitation precludes an in-module spec by design.
  4. **`Ord`/`Eq` agreement with the raw integer** — ✅ **justified, not missing.** The comparison
     operators are **derived** (`virt.rs:36`) and **out of scope**; the hard rule forbids touching
     unlisted functions. `view_design.md` (§Round-trip corollaries) explicitly classifies this as
     a derivable consequence, deliberately made *expressible* by `View = self.0 as int`, not an
     in-scope contract to add.
- **Missing (genuine in-scope function without a spec): none.**

## Proof Completeness
- Remaining admit(): **0** (none) — no BLOCKER
- Remaining external_body not in tcb-allowed.md: **0** (no external_body in module) — no BLOCKER

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES** — the module introduces **zero** in-module
  external_body / assume_specification. The single residual trust surface
  (`<VirtualAddress as Address>::into_raw_value` identity) lives on the consumer side
  (`kernel/.../phys.spec.rs`) and is pre-approved at `tcb-allowed.md:266`
  (`result as int == addr@`). No NEW trust boundary introduced inside this module.

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **0**, assume_specification: **0**, cfg-gated exec: **0**
- Confirmed independently by `verify.sh`: `assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
- Legitimate (non-cheating) conditionals: `cfg(verus_keep_ghost)` ghost includes (`virt.rs:9,11`);
  `cfg(target_pointer_width="32")` platform conditionals (`virt.rs:39,308`, out of scope, AST-MATCH).

## AST Consistency
- AST check: **PASS** — `ast_consistency.py` (auto-detect base on `verus-ai/*`): 18 functions + 1
  struct MATCH, 0 mismatch. No `// VERUS REWRITE` comments. Exec code byte-faithful to the original
  after stripping ghost annotations.
  (Note: `--base-ref` mode reported name-collision false positives on the two same-named
  `from_raw_value` symbols; the recommended auto-detect mode is fully consistent.)

## Verification
- verus: **PASS** — `make verify-sys` exit 0, status CLEAN, 0 errors. In-scope body-verified
  targets `VirtualAddress::new` + inherent `from_raw_value` confirmed (absent from
  coverage-unverified.txt). `make build` no-op target; `sys` crate compiles (cargo exit 0).
- spec drift: **PASS** — vs base branch `verus-ai/hal-platform-microvm`: 0 contract drift, 1 function
  added (strengthening only). The `spec_drift.py --before HEAD` "ensures removed" is a tool artifact
  (two same-named `from_raw_value` symbols; working tree byte-identical to HEAD).

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` = "None" — confirmed accurate)
- True Bugs: **0**
- The sole unverifiable item, `<VirtualAddress as Address>::into_raw_value`, is correctly classified
  as a **Verus front-end limitation** (whole-impl trait verification pulls the unsupported
  `usize as *const u8` casts of sibling `as_ptr`/`as_mut_ptr` into scope) — **not a code bug**.
  Documented in `verus-unsupported.md`; identity contract preserved at the allow-listed consumer boundary.

## Issues (highest priority first)
1. *(Minor, accepted — residual trust surface)* `into_raw_value` identity is held by a consumer-side
   `assume_specification`, not body-verified in `sys`. Documented, pre-approved Verus limitation; to be
   discharged when the `Address` trait becomes verifiable. Not a blocker.
2. *(Cosmetic)* `result.inv()` on `new` / inherent `from_raw_value` is logically subsumed by the
   identity ensures + `usize` input; retained deliberately as a caller convenience. Harmless.
3. *(Tooling note)* `spec_drift.py --before HEAD` and `ast_consistency.py --base-ref` produce
   false positives from the two same-named `from_raw_value` symbols; authoritative base-branch /
   auto-detect comparisons are clean. No code action.
4. *(Reconciled dissent)* gpt-5.3-codex scored caller coverage 4/6 (→ FAIL) by requiring in-module
   specs for `into_raw_value` purity and `Ord`/`Eq` agreement. Adjudicated as **covered/justified**:
   purity is type-system-guaranteed (spec-design: skip), value identity is the allow-listed consumer
   boundary, and the comparison operators are derived + explicitly out of scope (hard rule forbids
   specifying them). No in-scope function lacks a spec.

## Result: PASS
