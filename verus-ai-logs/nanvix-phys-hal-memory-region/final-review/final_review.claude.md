# Final Verification Review — `hal-memory-region`

**Module:** `hal::mem::types::region`
**Reviewer:** independent strict final review (Claude)
**Date:** 2026-06-15
**In-scope target functions (only):** `TruncatedMemoryRegion::start`, `MemoryRegion::start`,
`TruncatedMemoryRegion::size`, `MemoryRegion::size`.

---

## Spec Quality

The View is a single shared `MemoryRegionView { start: int, size: int, typ, perm, cache_policy }`
used by both `MemoryRegion<T>` and the newtype `TruncatedMemoryRegion<T>` (`view() == self.0@`).
The geometry endpoints use mathematical `int`, matching the `Address` contract
(`T: View<V = int>`, `into_raw_value() as int == self@`). Reusable helpers `wf()` (`size > 0`)
and `is_page_aligned()` live on the View, and `inv()` is defined per type:
`MemoryRegion::inv == wf()`, `TruncatedMemoryRegion::inv == wf() && is_page_aligned()`.
`spec_page_size()` is a concrete `open spec fn` (= `arch::mem::PAGE_SIZE as int`), not `uninterp`.

The four getter contracts:

| Function | Ensures | Verdict |
|---|---|---|
| `MemoryRegion::start` | `result@ == self@.start` | correct, declarative, View-consistent |
| `MemoryRegion::size` | `result as int == self@.size` | correct usize→int projection |
| `TruncatedMemoryRegion::start` | `result@ == self@.start` | correct; alignment via `inv()` |
| `TruncatedMemoryRegion::size` | `result as int == self@.size` | correct; multiple via `inv()` |

- **Not tautological:** each pins the return value to a specific View field.
- **Not subsumed / no redundancy:** page-alignment is intentionally *not* restated on the
  truncated getters — it is derivable from `inv().is_page_aligned()` combined with the value
  equality, so adding it would be a subsumed clause (correctly avoided per spec-design).
- **Mathematical types:** `int` used throughout the View; `usize` results bridged with `as int`.
- **View consistency:** truncated View delegates to inner (`self.0@`), so the delegating getters
  (`self.0.start()` / `self.0.size()`) discharge their contracts against the inner getters with no
  glue — clean and faithful.
- **Caller-written test:** the contracts are usable verbatim by callers (raw-value / frame-number
  projections and ordering all read `self@.start` / `self@.size`).

Spec quality: **PASS.**

---

## Caller Coverage (Covered 4/4 + missing list)

From `caller_analysis.md`:

| Function | Caller expectation | Covered by |
|---|---|---|
| `MemoryRegion::start` | returns exact stored start, by value, pure | `ensures result@ == self@.start` ✓ |
| `MemoryRegion::size` | exact byte length, `> 0` | value: `result as int == self@.size`; `>0`: `inv()→wf()` ✓ |
| `TruncatedMemoryRegion::start` | base **page-aligned** (`start % page_size == 0`), equal to stored start | value: `result@ == self@.start`; alignment: `inv()→is_page_aligned()` ✓ |
| `TruncatedMemoryRegion::size` | `> 0` **and** `size % page_size == 0` (frame.rs `size/FRAME_SIZE` exact; MMIO overlap) | value: `result as int == self@.size`; `>0` + multiple: `inv()→wf() && is_page_aligned()` ✓ |

- The two load-bearing expectations called out in the task are both covered:
  - **`size % page_size == 0`** (frame.rs `size / FRAME_SIZE` exact frame count) → established by
    `TruncatedMemoryRegion::inv()`'s `is_page_aligned()` (`self@.size % spec_page_size() == 0`),
    combined with `result as int == self@.size`.
  - **Page-aligned start** → `is_page_aligned()`'s `self@.start % spec_page_size() == 0`, combined
    with `result@ == self@.start`.
- The non-empty (`size > 0`) expectation is carried by `wf()` inside both `inv()`s.
- **In-range** (`start + size - 1 <= T::max_addr()`) is *deliberately deferred*: `Address::max_addr`
  has no spec counterpart today, and none of the four in-scope getters need it. This is documented
  in `view_design.md` (Rejected Alternatives / deferred) and is not a coverage gap for the getters.

Design note (not a defect): the getters expose value-equality; the alignment/non-emptiness facts
are `inv()` properties the caller must have in scope. Establishing `inv()` is the job of the
constructors (`new`, `from_*`), which are out of the 4-function scope. This is the correct,
non-redundant split and matches `view_design.md`.

**Missing/uncovered: none.** Coverage 4/4.

---

## Proof Completeness (admit / external_body counts + locations)

Region files only (`region.rs`, `region.spec.rs`, `region.proof.rs`):

```
grep -rn "admit()|assume(|external_body|assume_specification|trusted|..." region*.rs
→ NO MATCHES
```

- **`admit()` in region files: 0** → no blocker.
- **`external_body` in region files: 0** → no blocker.
- `region.proof.rs` is empty (`verus! { }`); `region.spec.rs` contains only View + helpers + `inv()`.
- The build summary's global counts (`admit=12 external_body=19`) are **whole-kernel** totals from
  other, out-of-scope modules — none reside in the region files (confirmed by the grep above).

Proof completeness: **PASS.**

---

## TCB Compliance

Zero `external_body` exist in the region files, so the module is trivially TCB-compliant — there is
nothing to list. `tcb-allowed.md` additionally records that the earlier placeholder
`assume_specification` for `TruncatedMemoryRegion::<T>::start`/`size` (in `frame.spec.rs`) were
**removed** once this module gained real `#[verus_spec]` contracts (now superseding them). No new
trust-boundary entries are required by this module.

TCB compliance: **PASS.**

---

## Guardrails Compliance (exact counts, region files only)

| Guardrail | Count | Status |
|---|---|---|
| `admit()` | 0 | OK (admit>0 = BLOCKER) |
| `assume(...)` | 0 | OK (assume>0 = BLOCKER) |
| `external_body` | 0 | OK |
| `assume_specification` | 0 | OK |
| `trusted` | 0 | OK |
| `exec_allows_no_decreases` / `spinoff` / `rlimit` / `uninterp` | 0 | OK |
| cfg-gated **exec** code | 0 | OK |

The two `#[cfg(verus_keep_ghost)] include!("region.spec.rs"/"region.proof.rs")` lines are the
standard spec/proof include pattern (ghost-only material), **not** cfg-gated exec branches — correct
and excluded from the cfg-gate concern, as the task notes.

Guardrails: **PASS.**

---

## AST Consistency (PASS/FAIL + verdict on the start MISMATCH)

`ast_consistency.py summary`: **27 MATCH, 1 MISMATCH** (matched=27, mismatched=1, missing=0, extra=0).

The single MISMATCH is `MemoryRegion::start`:
```
-        self.start.clone()
+        self.start.clone_address()
```

Evaluation:
- **VERUS REWRITE comment present** (region.rs:210-220) explaining the substitution.
- **Minimal reproducer present & confirmed** in the comment: a generic
  `fn f<T: Clone + View<V=int>>(x:&T)->(r:T) ensures r@==x@ { x.clone() }` fails
  ("postcondition not satisfied") because `Clone::clone` is unspecified; replacing with
  `x.clone_address()` verifies (1 verified, 0 errors).
- **`clone_address` carries the required contract:** `src/libs/sys/src/sys/mm/address/mod.rs:84-88`
  declares `fn clone_address(&self) -> Self` with `#[verus_spec(result => ensures result@ == self@)]`.
- **Impls are genuine clones:**
  - `PhysicalAddress::clone_address` → `PhysicalAddress(self.0)` (copies inner value).
  - `PageAligned::clone_address` → `PageAligned(self.0.clone_address())`.
  - `PageTableAligned::clone_address` → `PageTableAligned(self.0.clone_address())`.
  All return the same abstract value as `Clone::clone` — semantically equivalent, `Copy`-cost.
- This is a genuine Verus limitation (`Clone::clone` has no Verus spec), documented with full
  evidence per the `ast-consistency` "Handling MISMATCHes" process.

**Verdict: MISMATCH = acceptable-justified, NOT a blocker.** It is a documented, view-preserving,
semantically-equivalent rewrite required to discharge the `result@ == self@.start` postcondition.

AST consistency: **PASS (with one justified deviation).**

---

## Verification (PASS/FAIL)

`make verify-kernel MODULE=hal::mem::types::region`:
- `status: CLEAN`, exit 0, verification cached (no recompilation).
- Coverage: **4/28 exec functions have contracts** — exactly the four in-scope getters
  (`MemoryRegion::start`, `MemoryRegion::size`, `TruncatedMemoryRegion::start`,
  `TruncatedMemoryRegion::size`); the 24 unverified functions are out-of-scope.
- Cheating check for the module: `✅ No cheating detected in module hal::mem::types::region.`
- Spec drift (`spec_drift.py git-diff ... --before HEAD`): **0 contract drift** (no ensures
  removed, no requires added).

Verification: **PASS.** Error count: 0.

---

## Bug Summary

- `bugs.md` **does not exist** for this module — confirmed (`ls` → "No such file or directory").
  No bugs were recorded.
- Independent inspection of the four in-scope functions: all are pure field projections.
  - `MemoryRegion::start` → `self.start.clone_address()` (view-preserving clone — correct).
  - `MemoryRegion::size` → `self.size` (correct).
  - `TruncatedMemoryRegion::start` → `self.0.start()` (delegation — correct).
  - `TruncatedMemoryRegion::size` → `self.0.size()` (delegation — correct).
- **No code defect found** in any of the four functions. The absence of `bugs.md` is consistent
  with there being no real bug to report.

---

## Issues (priority order)

None. (The one AST MISMATCH is a documented, justified, semantically-equivalent rewrite and is
explicitly not a failure condition.)

---

## Result: **PASS**

All strict criteria satisfied:
- admit = 0, assume = 0 in region files.
- No `external_body` in region files (so nothing to reconcile against `tcb-allowed.md`).
- The single AST MISMATCH (`clone()` → `clone_address()`) is justified, documented, and
  semantically equivalent (a Verus-limitation rewrite, not a logic change).
- Caller coverage complete (4/4; `size % page_size == 0` and page-aligned start both covered via
  `inv()` + value-equality ensures).
- Verification CLEAN / exit 0 / 0 errors / no contract drift.
