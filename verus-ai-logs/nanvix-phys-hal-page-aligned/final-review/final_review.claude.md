# Final Comprehensive Review — `hal-page-aligned`

**Reviewer**: Independent strict final review (Claude)
**Date**: 2026-06-15
**Branch**: `verus-ai-prove-bottom-up`
**Verus**: `0.2026.05.31.5dd6d83` (`build/verus-version`)

In-scope targets: `PageAligned::from_address`, `PageAligned::into_raw_value`, type `PageAligned`.
In-scope files:
- `src/kernel/src/hal/mem/types/address/aligned/page.rs`
- `src/kernel/src/hal/mem/types/address/aligned/page.spec.rs`
- `src/kernel/src/hal/mem/types/address/aligned/page.proof.rs`

---

## Checklist

### Caller Analysis
- [x] `caller_analysis.md` present, identifies callers for both in-scope functions, corrects the LSP false-negative for `into_raw_value` (frame.rs:120, elf.rs:288, internal `into_physical_address`).
- [x] Every documented caller expectation maps to a `requires`/`ensures` (see Caller Coverage).

### View Design
- [x] `view_design.md` present; View = scalar `int` (the address), `inv()` = page-alignment. Substitution test, caller-only, minimal, no code-as-spec all satisfied. Mirrors verified `FrameAddress` model.

### Specification
- [x] `from_address` carries a full `#[verus_spec]` (page.rs:42-48).
- [x] `into_raw_value` contract inherited from the `Address` trait declaration (`src/libs/sys/src/sys/mm/address/mod.rs:63-67`, `result as int == self@`).
- [x] Type `PageAligned` has `View` (`view()==self.0@`, page.rs:224-227) and `inv()` (page.spec.rs:14-17).

### Proving
- [x] Module verifies: **2 verified, 0 errors** (fresh, non-cached run).
- [x] 0 `admit()` in in-scope files.

### Cheating Elimination
- [x] In-scope files: `admit=0, assume=0, external_body=0, assume_specification=0, cfg-gated-exec=0`.
- [x] Module-level cheating check: "✅ No cheating detected in module hal::mem::types::address::aligned::page."

### Bug Recording
- [x] `bugs.md` present; VERUS-TOOL-1 recorded and re-confirmed; per task guidance it is a documented Verus tool limitation, not an admit/external_body. `verus-unsupported.md` carries the full reproducer/isolation.

**All checklist items checked.**

---

## Spec Quality

**`from_address(addr: T) -> Result<Self, Error>`** (page.rs:42-48)
```
ensures match ret {
    Ok(r)  => spec_aligned(addr@) && r@ == addr@ && r.inv(),
    Err(_) => !spec_aligned(addr@),
}
```
- Mathematical types: uses `@` (int domain). `spec_aligned(v) := v % spec_page_size() == 0`, `spec_page_size()` is **concrete** (`::arch::mem::PAGE_SIZE as int`, frame.rs:42-44) — not `uninterp`.
- Validate-not-normalize correctly captured: `Ok(r) => r@ == addr@` (no silent re-align) and the invariant `r.inv()`.
- **Meaningful error path** (not tautological): `Err(_) => !spec_aligned(addr@)` is the bidirectional negation of success, not `Err(_) => true`. This rejects a buggy impl that returns `Err` for an aligned input.
- Sufficient to reject bugs: an impl returning `Ok` for an unaligned address violates `spec_aligned(addr@)`; an impl that re-aligns violates `r@ == addr@`.
- Minor (non-blocking) observation: within the `Ok` arm, `r.inv()` is logically *subsumed* by `spec_aligned(addr@) && r@ == addr@` (since `r@==addr@` ⇒ `r.inv() ⟺ spec_aligned(addr@)`). It is retained deliberately as the **caller-facing fact** (`FrameAddress::from_raw_value`, region, vmem consume `.inv()` directly), consistent with the spec-design "written for the caller" principle. Not over-specification of behavior, just a convenience restatement — acceptable.

**`into_raw_value(self) -> usize`** — inherited trait contract `result as int == self@`.
- Total, side-effect-free, value-preserving projection in `int`. Matches `FrameAddress::into_raw_value` spec verbatim. Not tautological; pins the result to the abstract address.

**Verdict**: external-top API contracts are correct, complete, declarative, and caller-abstract.

---

## Caller Coverage

**Covered: 3/3 in-scope targets; all listed caller expectations satisfied.**

| Caller expectation (caller_analysis.md) | Spec clause | Status |
|---|---|---|
| `from_address` Ok ⇒ `result@ % page == 0` (`inv`) | `r.inv()` | Covered |
| `from_address` Ok ⇒ value preserved `result@ == addr@` | `r@ == addr@` | Covered |
| `from_address` Err ⇒ input was unaligned | `Err(_) => !spec_aligned(addr@)` | Covered |
| `into_raw_value` ⇒ `result as int == self@` | trait-decl ensures (mod.rs:63-67), inherited | Covered |
| Type ⇒ View `@ == inner@`; invariant `@ % page == 0` | `view()` + `inv()` | Covered |

**Missing: none.**

Confirmed the inheritance claim: the trait method `into_raw_value` in `src/libs/sys/src/sys/mm/address/mod.rs:63-67` carries `#[verus_spec(result => ensures result as int == self@)]`, and `PageAligned`'s impl inherits it; this is what `FrameAddress::into_raw_value` (`tcb-allowed.md` placeholder) ultimately relies on.

---

## Proof Completeness

- **`admit()` count in in-scope files: 0.** Evidence: `grep -nE "admit\(" page.rs page.spec.rs page.proof.rs` → no matches.
- **`external_body` not in tcb-allowed.md (in-scope): 0.** No `external_body` exists in any in-scope file at all.
- Module fresh verification: `verification: 2 verified, 0 errors (exit 0)` / `status: CLEAN` (log `verus-ai-logs/verify-kernel/verus-logs/verus_2026-06-15_07-54-36.log`).
- `into_raw_value`'s **impl body** is trusted-via-trait-spec due to VERUS-TOOL-1 (generic-trait-impl panic). Per the explicit task note this is *not* an `admit`/`external_body` and *not* a blocker; the contract still reaches callers via the trait declaration. `proof.rs` is empty (`verus! { }`), `spec.rs` is concrete.

No admit anywhere ⇒ **no BLOCKER**.

---

## TCB Compliance

- In-scope `external_body` count: **0**. Therefore nothing new needs to appear in `tcb-allowed.md`; no new trust boundary is introduced by this module.
- The whole-kernel `external_body=11 / admit=27` reported by the crate-level run are entirely in **out-of-scope** modules (`mm/phys/*`, `mm/virt/*`) per `cheating-detail.txt`; none in `aligned/page`.
- The `tcb-allowed.md` placeholders touching this module (`PageAligned::<T> as Address::into_raw_value` / `as Deref::deref` listed for `mm::phys::frame.spec.rs`) are dependency-side `assume_specification`s in *other* modules, not in the in-scope files. Compliant.

**TCB: compliant.**

---

## Guardrails Compliance (exact counts, in-scope files only)

| Dimension | Count | Locations |
|---|---|---|
| `admit` | 0 | — |
| `assume` | 0 | — |
| `external_body` | 0 | — |
| `assume_specification` | 0 | — |
| cfg-gated exec | 0 | only `#[cfg(verus_keep_ghost)]` at page.rs:9 and :11 guard `include!("page.spec.rs")`/`include!("page.proof.rs")` — ghost includes, **not** exec gating |

Evidence: `grep -nE "admit\(|assume\(|assume_specification|external_body|verifier::(trusted|external)|rlimit|spinoff|exec_allows_no_decreases|uninterp|VERUS REWRITE|cfg\(not\(verus_keep_ghost"` over the three files → **NONE FOUND**.

`admit==0` and `assume==0` ⇒ **no BLOCKER**.

---

## AST Consistency

**PASS.** `ast_consistency.py --base-ref de24f6057` (pre-verification baseline) over `page.rs`:
- All 17 pre-existing functions + struct `PageAligned`: **MATCH**. Both in-scope exec bodies (`from_address`, `into_raw_value`) MATCH.
- 1 `EXTRA_IN_VERUS`: `PageAligned::clone_address` — a method added crate-wide when the `Address` trait gained `clone_address` (sys-trait extension), **not** a modification of an in-scope function. Tool verdict: `Consistent: ✅ YES (matched=17 mismatched=0 missing=0 extra=1)`.
- **0 MISMATCH.** No `// VERUS REWRITE` comments exist in scope. No exec logic changed.

`git diff --stat` for the `aligned/` directory: **no source changes** in the working tree.

---

## Verification

**PASS — 0 errors.**

| Command | Result |
|---|---|
| `make verify-kernel MODULE=kernel::hal::mem::types::address::aligned::page` (fresh, after touch) | `2 verified, 0 errors`, exit 0, `status: CLEAN`, "✅ No cheating detected in module" |
| `make verify-kernel` (full crate) | exit 0; crate-level `CHEATING_DETECTED` reflects out-of-scope WIP modules only (mm/phys, mm/virt) — in-scope module is clean |
| `./z build -- check` (normal, ghost-erased compile of workspace) | `build-finished: success`, "[OK] Build complete." |
| `spec_drift.py git-diff page.rs --before HEAD` | "✅ No contract drift detected" (0 ensures removed; specs were only added relative to the unspecified baseline) |

Note: there is no `make build` target; the project builds via `./z` (build skill). `./z build -- check` is the workspace compile validation and passed. The kernel exec code additionally compiles successfully as part of the (passing) `verify-kernel` run.

---

## Bug Summary

- **VERUS-TOOL-1** — Verus internal panic (`vir/src/traits.rs:511 inherit_default_bodies`) when `#[verus_verify]`-annotating the generic `impl<T: Address> Address for PageAligned<T>`. Classification per bug-reporting skill: **False Positive / verifier-tool limitation** (not a Nanvix code bug). Status **open**, correctly recorded in `bugs.md` + `verus-unsupported.md` with isolation (non-generic `PhysicalAddress` impl does not panic) and an empirical demonstration that the impl body is unchecked (replacing it with a value-violating body still reports `2 verified, 0 errors`). Mitigation: impl left unannotated; contract delivered via trait declaration; **no** `admit`/`assume`/`external_body` introduced. Consistent with the task's explicit guidance.
- **PAGE_ALIGNMENT trust-boundary removal** (bugs.md "Improvement") — `assume_specification[PAGE_ALIGNMENT]` was eliminated by adding `#[verus_verify]` to the constant; strengthens the spec. Not a bug; verified to still pass.
- **`from_address` admit eliminated** (view_design.md turn-1 update) — confirmed: no `admit` remains in scope.
- **New bugs found during proving not previously recorded:** none. No code logic defect exists in the in-scope functions; both bodies are trivial/correct projections.

---

## Issues (highest priority first)

1. *(Informational, not blocking)* `into_raw_value`'s **impl body** is trusted via the inherited trait-declaration contract rather than machine-verified, due to VERUS-TOOL-1. The task explicitly designates this as acceptable (documented tool limitation, not admit/external_body). Recommendation: re-add `#[verus_verify]` to the generic impl once an upgraded Verus fixes the `inherit_default_bodies` duplicate-registration assertion. Tracked in `verus-unsupported.md`/`bugs.md`.
2. *(Minor, cosmetic, not blocking)* `from_address`'s `Ok` arm restates `r.inv()`, which is logically subsumed by `spec_aligned(addr@) && r@ == addr@`. Retained intentionally as the direct caller-facing fact; acceptable but technically redundant.

No correctness, TCB, cheating, AST, or verification blockers found.

---

## Result: PASS

All checklist items are checked. Module verifies **2 verified, 0 errors** with **0 admit / 0 assume / 0 external_body / 0 assume_specification / 0 cfg-gated-exec** in the in-scope files, AST consistency is clean (0 mismatches on existing/in-scope code), caller coverage is complete (3/3 targets, 0 missing), no TCB additions, and all bugs are correctly classified and recorded. The single tool limitation (VERUS-TOOL-1) is, per task guidance, an accepted trusted-via-trait-spec condition rather than a verification escape.
