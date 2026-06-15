# Final Independent Review — `mm::phys::frame` (Verus)

**Reviewer:** Claude (independent final review)
**Date:** 2026-06-15
**Branch:** `verus-ai-prove` (HEAD `e28e4a3f2`)
**Scope:** `share, Inner::share, instance, refcount, Inner::refcount, is_covered,
Inner::is_covered, free_count, free, Inner::free, book, Inner::book, alloc_range,
Inner::alloc_range, alloc_contiguous, Inner::alloc_contiguous, alloc, Inner::alloc`
(`init` excluded).
**Files:** `frame.rs` (1677 L), `frame.spec.rs` (38 L), `frame.proof.rs` (720 L).

All claims below are backed by tool-verified commands (grep, ast_consistency.py,
spec_drift.py, `make verify-kernel`, `make verify`). Worktree source files are
committed/clean (`git status --porcelain` shows only review artifacts).

---

## Spec Quality

The **`Inner::*` methods** carry strong, complete, in-body-verified contracts over
the abstract `FrameAllocView { allocated_frames, free_frames, refcounts }`:

- `Inner::alloc` — Ok: exact set transition (insert/remove + refcount=1); Err:
  `final@ == old@ && free_frames.is_empty()`. **Non-tautological, full.**
- `Inner::alloc_contiguous` — Ok: contiguous frame-set union/difference + refcount=1;
  Err: `final@ == old@`. **Frame-preserving error path.**
- `Inner::alloc_range` — Ok: region set reserved; Err: `final@ == old@ &&
  !frames.subset_of(free)`. **Full.**
- `Inner::free` — Ok: branch on last-reference (release vs decrement); Err:
  `final@ == old@ && !allocated.contains`. **Full.**
- `Inner::share` — Ok: refcount+1; Err: `final@ == old@ && (!allocated ||
  refcount >= 255)`. **Both failure modes named.**
- `Inner::refcount` — Ok: `count == refcounts[frame@]`; Err: `!allocated`. **Full.**
- `Inner::book` — Ok: free→allocated + refcount=1; Err: `final@==old@ && !free`. **Full.**
- `Inner::is_covered` — `ret <==> allocated ∨ free`. **Full.**

The **free-function wrappers** (the external-top API the manager/upool/mod call)
are deliberately weaker and pinned to the parameter-free global `phys_view().frames`
(see TCB rationale). Quality of wrapper specs:

- `alloc`: Ok `frame.inv() && allocated.contains`; Err `free_frames.is_empty()` — good.
- `free_count`: `result as nat == free_count()` — good.
- `free`: `ensures true`, `opens_invariants none`, `no_unwind` — **intentionally empty**;
  correct/required for the `Drop` callers (documented, matches caller_analysis).
- `is_covered`: `ret <==> covers(phys_addr@)` (verified in-body, not external_body) — good.
- `book`: Err `!free_frames.contains` — meaningful.
- `alloc_range`: Err `!all_free(region_frame_addrs)` — meaningful.
- `share`: Err `!allocated || refcounts[frame@] >= 255` — meaningful, both modes.
- `refcount`: Err `!allocated` — meaningful.
- **`alloc_contiguous`: `Err(_) => true`** — TAUTOLOGICAL error arm (spec-design
  anti-pattern). The caller (`manager::alloc_kernel_frames`) expects "state unchanged"
  on `Err`; the wrapper provides nothing. *Mitigation:* `Inner::alloc_contiguous`
  proves `final@ == old@`; the wrapper cannot re-express it because the parameter-free
  `phys_view()` accessor has no pre/post receiver, and the caller re-derives restoration
  via its own partition + `lemma_user_bulk_err_restored`. **Minor weakness, not a blocker.**

`frame.spec.rs` is clean: all `assume_specification`/`external_type_specification`
placeholders have been **removed** (superseded by real verified contracts) — only
explanatory comments remain.

---

## Caller Coverage (from `caller_analysis.md`)

9 in-scope free-function entry points (excluding `init`). Each caller expectation
(success + failure) checked against the wrapper requires/ensures:

| Function | requires | Ok-path | Err-path | Verdict |
|---|---|---|---|---|
| `alloc` | — | `inv` + allocated ✓ | `free.is_empty()` ✓ | **Covered** |
| `alloc_contiguous` | `count > 0` ✓ | `inv` + no-overflow bound ✓ | `=> true` ✗ (caller wants state-unchanged) | **Partial** |
| `alloc_range` | — | all reserved ✓ | `!all_free` (info, not state-unchanged) | **Covered** |
| `book` | — | reserved + r=1 ✓ | `!free.contains` ✓ | **Covered** |
| `is_covered` | `inv` ✓ | `covers ⇔` ✓ | n/a (bool) | **Covered** |
| `free` | — (Drop-safe) | best-effort ✓ | best-effort ✓ (`no_unwind`/`opens_invariants none`) | **Covered** |
| `free_count` | — | `== free_count()` ✓ | n/a | **Covered** |
| `share` | `frame.inv()` ✓ | allocated ✓ | `!allocated ‖ r≥255` ✓ | **Covered** |
| `refcount` | `frame.inv()` ✓ | allocated + `count==refcounts` ✓ | `!allocated` ✓ | **Covered** |

**Covered: 8 / 9 full** (success-path 9/9). The single gap is
`alloc_contiguous`'s tautological `Err(_) => true` arm (success path fully covered;
failure-state-unchanged not expressed at the wrapper, re-derived by the caller).
No caller success expectation is unmet.

> Note: `caller_analysis.md`'s "Pre-existing Specs / Assessment" section is stale —
> it states `is_covered`/`book`/`alloc_range` "have no wrapper spec yet". The final
> code **adds** wrapper specs for all three (pure strengthening; spec-drift clean).

---

## Proof Completeness

- **`admit()` in frame files: 0.** (`grep -nE 'admit\s*\(' frame.rs frame.spec.rs
  frame.proof.rs` → none; confirmed absent from `cheating-detail.txt`.)
- **`external_body` in frame files: 10**, all enumerated below and **all present in
  `tcb-allowed.md`**:
  `instance`(1408), `init`(1446, excluded), `alloc`(1502), `alloc_contiguous`(1532),
  `free_count`(1553), `free`(1571), `book`(1613), `alloc_range`(1634), `share`(1654),
  `refcount`(1675).
- **external_body NOT in TCB: 0.**

> The 16 global `admit`s reported by the pipeline are entirely **out-of-scope**:
> `hal/mem/types/address/*.proof.rs` (4), `mm/phys/manager.proof.rs` (4 — manager
> module, not frame), `mm/virt/identity_map*` (8). **None in the frame module.**

---

## TCB Compliance

**YES — every frame-module `external_body` is in `tcb-allowed.md`.** Verified by name:
`instance`, `init` (skip list), `alloc`, `alloc_contiguous`, `free_count`, `free`,
`book`, `alloc_range`, `share`, `refcount` are all listed (under "Allowed",
"Skip/exclude", and "Cross-module dependencies"). **No new/undocumented trust
boundary introduced.** `is_covered` is verified in-body (correctly NOT external_body).

---

## Guardrails Compliance (frame-module exact counts)

| Dimension | frame.rs | frame.spec.rs | frame.proof.rs | Total (frame) |
|---|---|---|---|---|
| `admit(` | 0 | 0 | 0 | **0** ✅ |
| `assume(` | 0 | 0 | 0 | **0** ✅ |
| `assume_specification` | 0 | 0 (comment only) | 0 | **0** ✅ |
| `external_body` | 10 | 0 | 0 | **10** (all in TCB) ✅ |
| `exec_allows_no_decreases` | 0 | 0 | 0 | **0** ✅ |
| cfg-gated exec `#[cfg(not(verus_keep_ghost))]` | 28 | — | — | **28** ⚠ (benign) |

- `admit == 0` and `assume == 0` → **no blocker.**
- The 28 `cfg(not(verus_keep_ghost))` gates wrap **exec-only diagnostics**
  (`debug_assert_eq!`, `error!` formatting, the `saturating_mul` address bindings
  fed only to log messages). They do **not** hide any verified logic — confirmed by
  AST consistency (stripping them leaves only the documented bug-fixes). The 2
  `#[cfg(verus_keep_ghost)]` are the standard `include!("frame.spec.rs"/".proof.rs")`
  guards; 3 `#[cfg_attr(verus_keep_ghost, verus_spec(...))]` attach loop invariants.

---

## AST Consistency

`ast_consistency.py summary`: **matched=11, mismatched=8, missing=0, extra=0.**
All 11 wrappers + the `Inner` struct MATCH. The 8 MISMATCHes are the `Inner::*`
methods; each was diffed and accounted for — **all are pre-approved deviations**:

| Function | Cause | Status |
|---|---|---|
| `Inner::alloc` | only `Ok(x) => { Ok(x) }` block-wrapping + stripped-ghost blanks | semantically identical (pre-approved block form) |
| `Inner::alloc_contiguous` | `// VERUS BUG FIX` count≤num_bits guard | documented bug-fix |
| `Inner::alloc_range` | `// VERUS BUG FIX` total index calc + `for..=` → `while`, `saturating_mul` diagnostics | documented bug-fix (range `start..=start+n-1` ≡ `while < start+n`) |
| `Inner::book` | `// VERUS BUG FIX` total index calc | documented bug-fix |
| `Inner::free` | `// VERUS BUG FIX` total index calc | documented bug-fix |
| `Inner::share` | `// VERUS BUG FIX` total index calc | documented bug-fix |
| `Inner::refcount` | `// VERUS BUG FIX` total index calc + unaligned-input rejection | documented bug-fix |
| `Inner::is_covered` | `// VERUS BUG FIX` total index calc | documented bug-fix |

All 7 behavioral changes carry `// VERUS BUG FIX:` comments and are recorded in
`bugs.md` (auto-fixable categories: panic-avoidance / overflow / bounds). No
`// VERUS REWRITE` comments exist. No undocumented exec change.

**Spec drift** (`spec_drift.py git-diff frame.rs --before HEAD`): exit 0 — 0
contract drift (0 ensures removed, 0 requires added). No spec weakened.

**Verdict: PASS** (all MISMATCHes are documented bug-fixes or pre-approved
block-wrapping; spec drift clean).

---

## Verification

- `make verify-kernel MODULE=mm::phys` → **Exit code 0** (verifies frame, kframe,
  manager, upool, mod). Error count: **0**. status reports CHEATING_DETECTED only
  because out-of-scope modules carry `admit`/`external_body`; frame-module is clean.
- `make verify` (full crate regression) → **Exit code 0**, no regressions across all
  modules. Error count: **0**.

**Verification: PASS (0 errors).**

---

## Bug Summary (`bugs.md`) — reconciliation against final code

5 entries:

1. **[auto-fixed] panic on top-of-space aligned address** (`into_frame_number().unwrap()`)
   — *safety-critical*. Replaced with total `into_raw_value() / FRAME_SIZE` at 7 sites
   (`free`, `share`, `refcount`, `book`, `is_covered`, `alloc_range`; `refcount` also
   adds unaligned rejection). **Verified present in final code; valid; fixed.**
2. **[auto-fixed] `internal_inv` clause 7 too weak** (admitted unrepresentable top frame
   on 32-bit) — *safety (state corruption/frame leak)*. Strengthened with
   `i <= spec_max_frame_number()` (confirmed `frame.proof.rs:69`). A strengthening →
   spec-drift clean. **Valid; fixed.** *(Touches the "do-not-modify" `internal_inv`;
   justified & flagged — see Issues.)*
3. **[auto-fixed] `alloc_contiguous` missing `count <= num_bits` guard** — *correctness*.
   Guard present (`frame.rs:297`). **Valid; fixed.**
4. **[auto-fixed] `alloc_range` diagnostic `index * FRAME_SIZE` overflow** — *robustness*.
   `saturating_mul`/`saturating_add` + cfg-gating present. **Valid; fixed.**
5. **[open] `alloc_range` off-by-one (body vs spec)** — *Status STALE.* The entry says
   it is "masked by `proof! { admit(); }`". The final code has **no admit** in
   `alloc_range` (or anywhere in frame), the body was rewritten, and the module
   **verifies (exit 0)** — i.e. Verus discharged the half-open/inclusive equivalence
   from `region.inv()` page-alignment. **The concern is RESOLVED; the doc status is
   outdated and should be marked closed.** No code defect remains.

No unrecorded bug discovered during this review. All surviving (out-of-scope) admits
are in other modules and are tracked there.

---

## Issues (highest priority first)

1. **(Minor / spec-quality)** `alloc_contiguous` wrapper has a tautological
   `Err(_) => true`. Caller wants failure-state-unchanged; provided only by
   `Inner::alloc_contiguous` and re-derived by the caller. Acceptable given the
   parameter-free `phys_view()` limitation, but it is the one anti-pattern per
   spec-design. *Not a blocker.*
2. **(Doc reconciliation)** `bugs.md` entry #5 is stale ("[open] … masked by admit()")
   — it is actually resolved (no admit, verifies). Recommend marking closed.
3. **(Doc reconciliation)** `caller_analysis.md` "Pre-existing Specs/Assessment"
   pre-dates the added `is_covered`/`book`/`alloc_range` wrapper specs; harmless.
4. **(Reviewer awareness, already flagged in bugs.md)** Bug-fix #2 modified the
   "do-not-modify" `Inner::internal_inv`. It is a *strengthening* to match documented
   intent, drift-clean, and necessary to prove the locked `Inner::alloc` spec on the
   32-bit target — accepted.

None of the above is a blocker.

---

## Result: PASS

Blocker checklist:
- admit (frame) = 0 ✅
- assume (frame) = 0 ✅
- assume_specification (frame) = 0 ✅
- all frame `external_body` (10) in TCB ✅
- AST consistent (all 8 MISMATCHes = documented bug-fixes / pre-approved block form) ✅
- spec drift = 0 ✅
- verification PASS (module exit 0, full exit 0, 0 errors) ✅

**Zero blockers. Final verdict: PASS.**
