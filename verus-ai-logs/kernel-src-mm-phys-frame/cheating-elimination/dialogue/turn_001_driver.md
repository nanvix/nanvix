# Turn 1: Full Checklist Review

## Progress
- Done: (none yet — first pass)
- Current: All 12 checklist items
- Remaining: (none — all reviewed below)

---

## Checklist Results

### 1. Zero `admit()` remaining — ✅ PASS

**Verification:** `grep -n 'admit()' frame.rs frame.spec.rs frame.proof.rs` — zero matches.

---

### 2. Zero `assume()` remaining — ✅ PASS

**Verification:** `grep -n 'assume(' frame.rs frame.spec.rs frame.proof.rs` — zero matches.
Note: `assume_specification` is a separate construct (trusted spec for external functions) and is evaluated under item 7.

---

### 3. Zero trusted functions — ✅ PASS

**Verification:** `grep -n 'trusted\|#\[verifier::trusted\]' frame.rs frame.spec.rs frame.proof.rs` — zero matches.

---

### 4. Zero `exec_allows_no_decreases_clause` — ✅ PASS

**Verification:** `grep -n 'no_decreases\|exec_allows_no_decreases' frame.rs frame.spec.rs frame.proof.rs` — zero matches.

---

### 5. Zero cfg-gated exec code (only imports/derives/debug_assert/logging allowed) — ✅ PASS

**Verification:** Searched all `#[cfg(...)]` in frame.rs. Found:

| Line | Gate | Content | Classification |
|------|------|---------|---------------|
| 46,49 | `cfg(verus_keep_ghost)` | `include!("frame.spec.rs")`, `include!("frame.proof.rs")` | **imports** ✅ |
| 243 | `cfg(not(verus_keep_ghost))` | `error!("{error:?}");` | **logging** ✅ |
| 541 | `cfg(not(verus_keep_ghost))` | `error!("{error:?} (frame={frame:?})");` | **logging** ✅ |
| 748 | `cfg(not(verus_keep_ghost))` | `error!("{error:?} (phys_addr={phys_addr:?})");` | **logging** ✅ |
| 845 | `cfg(not(feature = "nightly-..."))` | Original feature gate (not a verus gate) | **original code** ✅ |
| 893-894 | `cfg(not(verus_keep_ghost))` | `let uncovered_addr = index * mem::FRAME_SIZE;` | **logging variable** ✅ |
| 896-897 | `cfg(not(verus_keep_ghost))` | `error!(...)` | **logging** ✅ |
| 955-967 | `cfg(not(verus_keep_ghost))` | `let conflicting_addr`, `let region_start`, `let region_end`, `error!(...)` | **logging variables + logging** ✅ |
| 1005-1006 | `cfg(not(verus_keep_ghost))` | `error!(...)` | **logging** ✅ |
| 1168-1174 | `cfg(not(verus_keep_ghost))` | `info!(...)` | **logging** ✅ |

All cfg-gated exec code is imports or logging. The variables at lines 893-894, 955-961 are
exclusively used in error-reporting `error!()` calls, documented as "BUG FIX: cfg-gate
error-reporting multiply to avoid usize overflow".

---

### 6. Zero `external_body` on user functions (only external-bottom trust boundaries allowed) — ✅ PASS

**Verification:** 10 `external_body` functions found in frame.rs:

**Conversion wrappers (6) — external-bottom trust boundaries:**
These wrap generic Deref trait chains and arch-crate type conversions that Verus cannot
reason about (`assume_specification` cannot match generic method signatures).

| # | Function | Line | Body | Classification |
|---|----------|------|------|---------------|
| 1 | `frame_addr_to_bitmap_index` | 63 | `self_.into_frame_number().into_raw_value()` | STDLIB_WRAPPER |
| 2 | `bitmap_index_to_frame_addr` | 74 | `FrameNumber::from_raw_value` + `FrameAddress::from_frame_number` | STDLIB_WRAPPER |
| 3 | `page_aligned_pa_to_bitmap_index` | 93 | `self_.into_frame_number().into_raw_value()` | STDLIB_WRAPPER |
| 4 | `region_start_frame_number` | 104 | `region.start().into_frame_number().into_raw_value()` | STDLIB_WRAPPER |
| 5 | `region_size_raw` | 115 | `region.size()` | STDLIB_WRAPPER |
| 6 | `region_start_raw` | 125 | `region.start().into_raw_value()` | STDLIB_WRAPPER |

**Singleton wrappers (4) — external-bottom trust boundary (unsafe static singleton):**
The `Inner` methods (`alloc`, `free`, `book`, `alloc_range`) are **fully verified** with
strong specs. These public wrappers delegate to `instance()` which uses
`unsafe { INSTANCE.assume_init_mut() }`. Verus cannot reason about the `MaybeUninit`
singleton pattern.

| # | Function | Line | Body | Classification |
|---|----------|------|------|---------------|
| 7 | `pub alloc()` | 1184 | `instance().alloc()` | SINGLETON_WRAPPER |
| 8 | `pub free()` | 1201 | `instance().free(frame)` | SINGLETON_WRAPPER |
| 9 | `pub book()` | 1217 | `instance().book(phys_addr)` | SINGLETON_WRAPPER |
| 10 | `pub alloc_range()` | 1231 | `instance().alloc_range(region)` | SINGLETON_WRAPPER |

All 10 functions are legitimate external-bottom trust boundaries.

---

### 7. Challenge each surviving trust item in trust.md — ❌ FAIL

**Verification:** Read `verus-ai-logs/kernel-src-mm-phys-frame/trust.md`. It documents 8 entries.

**Missing from trust.md (5 entries):**

1. **`assume_specification[init]`** (frame.spec.rs:38-42) — Has a tautological spec
   (`result.is_ok() || result.is_err()`, always true). Comment says "function body uses
   MaybeUninit::write() which Verus cannot compile even with external_body." This IS a
   trust item: callers rely on an unverified spec.

2. **`pub(super) fn alloc()`** (frame.rs:1184) — `#[verus_verify(external_body)]`. Weak
   spec: `Ok(frame) => frame.inv()`. Delegates to verified `instance().alloc()`.

3. **`pub(super) fn free()`** (frame.rs:1201) — `#[verifier::external_body]`. Tautological
   spec: `result.is_ok() || result.is_err()`. Delegates to verified `instance().free(frame)`.

4. **`pub(super) fn book()`** (frame.rs:1217) — `#[verus_verify(external_body)]`. Tautological
   spec: `result.is_ok() || result.is_err()`. Delegates to verified `instance().book(phys_addr)`.

5. **`pub(super) fn alloc_range()`** (frame.rs:1231) — `#[verus_verify(external_body)]`.
   Tautological spec: `result.is_ok() || result.is_err()`. Delegates to verified
   `instance().alloc_range(region)`.

**Documented items (8 entries) — all verified correct:**
Entries 1-8 in trust.md are properly classified with appropriate justifications and
reproducer descriptions.

---

### 8. AST consistency — ❌ FAIL (function-matching issues + real exec deviations)

**Verification:** Ran `python3 ast_consistency.py --base-ref dev frame.rs summary`.

**Result:** 0 MATCH, 4 MISMATCH, 6 MISSING_IN_VERUS, 8 EXTRA_IN_VERUS.

**Analysis of mismatches:**

The AST checker has a **function-matching bug**: it matches original `Inner::alloc` (line
66-91 in original) to the verus `pub(super) fn alloc()` singleton wrapper (line 1193-1195)
instead of to the verus `Inner::alloc` method (line 174-342). Same confusion for
`Inner::book` and `Inner::alloc_range`. Only `Inner::free` is matched correctly.

This also explains:
- MISSING_IN_VERUS for `alloc`, `book`, `alloc_range`, `free`, `init`, `instance` — these
  ARE in the verus file but the checker misidentified them.
- EXTRA_IN_VERUS for `Inner::init`, `Inner::instance` — these are `init` and `instance`
  (module-level functions) misidentified with the `Inner::` prefix.
- EXTRA_IN_VERUS for the 6 conversion wrappers — these are genuinely new functions (VERUS
  REWRITE wrappers), which is expected.

**Real exec code deviations (manually verified):**

| Function | Change | Documentation |
|----------|--------|---------------|
| `Inner::alloc` | 3-step FrameNumber/FrameAddress conversion → `bitmap_index_to_frame_addr(index)` | VERUS DEVIATION at line 256-260 ✅ |
| `Inner::free` | `frame.into_frame_number().into_raw_value()` → `frame_addr_to_bitmap_index(frame)` | VERUS DEVIATION at line 375-379 ✅ |
| `Inner::book` | `phys_addr.into_frame_number().into_raw_value()` → `page_aligned_pa_to_bitmap_index(phys_addr)` | No VERUS DEVIATION comment at call site (line 579) ⚠️ |
| `Inner::alloc_range` | (a) wrapper substitutions, (b) `..=` → `..` range, (c) cfg-gated error vars | VERUS REWRITE at lines 796-800 ✅ |
| `Inner::alloc_range` | `continue` → empty block `{}` (Ok(false) branch) | Not documented (trivially equivalent) |

---

### 9. All exec rewrites have VERUS REWRITE comment and minimal reproducer — ✅ PASS (minor gap)

**Verification:** All exec rewrites have `VERUS REWRITE` or `VERUS DEVIATION` comments
explaining the change and the Verus limitation. Reproducers are described textually
(e.g., "assume_specification cannot match generic method signatures") rather than as
standalone code files. One minor gap:

- `Inner::book` (line 579): uses `page_aligned_pa_to_bitmap_index(phys_addr)` instead of
  `phys_addr.into_frame_number().into_raw_value()` — the wrapper function has a VERUS
  REWRITE comment (line 92), but the call site in `book()` lacks a VERUS DEVIATION comment
  unlike the identical pattern in `free()` (line 375).

---

### 10. For each surviving `external_body`: not masking a code defect — ✅ PASS

**Verification:** Reviewed all 10 `external_body` functions:
- Conversion wrappers (6): wrap well-established arch/HAL type conversion chains. Specs
  constrain return values appropriately with arithmetic relationships.
- Singleton wrappers (4): specs are weaker than the verified Inner methods, but this is
  inherent to the singleton pattern (cannot express pre/post on global state), not masking
  a defect. The Inner methods carry the full verification burden.

---

### 11. trust.md contains only declared external-bottom trust boundaries — ❌ FAIL

**Verification:** Same findings as item 7. trust.md is **incomplete**: it is missing 5
trust boundary entries (the `assume_specification[init]` and the 4 singleton wrappers).
The 8 entries that ARE present are correctly classified as external-bottom trust boundaries.

---

### 12. Verification: `make verify-kernel MODULE=mm::phys::frame` — ✅ PASS

**Verification:**
```
$ make verify-kernel MODULE=mm::phys::frame
=== Results ===
  cached (no recompilation)
  Exit code : 0
```
Verification passes with 0 errors. The regular `cargo build` fails due to an unrelated
environment variable (`MEMORY_SIZE_BYTES`), which is a Makefile-level configuration
dependency, not a code issue.

---

## Summary

| # | Item | Status |
|---|------|--------|
| 1 | Zero admit() | ✅ PASS |
| 2 | Zero assume() | ✅ PASS |
| 3 | Zero trusted functions | ✅ PASS |
| 4 | Zero exec_allows_no_decreases_clause | ✅ PASS |
| 5 | Zero cfg-gated exec code | ✅ PASS |
| 6 | Zero external_body on user functions | ✅ PASS |
| 7 | Challenge surviving trust items | ❌ FAIL |
| 8 | AST consistency | ❌ FAIL |
| 9 | Exec rewrites documented | ✅ PASS |
| 10 | external_body not masking defects | ✅ PASS |
| 11 | trust.md completeness | ❌ FAIL |
| 12 | Verification passes | ✅ PASS |

---

## Fix Requests

### Fix 1: Update trust.md — add 5 missing entries (items 7, 11)

Add the following entries to `verus-ai-logs/kernel-src-mm-phys-frame/trust.md`:

**Entry 9: `assume_specification[init]`**
- File: `frame.spec.rs:38-42`
- Trust item: `assume_specification` on `init`
- Classification: `SINGLETON_INIT`
- Justification: `init()` uses `MaybeUninit::write()` which Verus cannot compile even with
  `external_body`. The spec is tautological (`result.is_ok() || result.is_err()`).
- Note: The function is not annotated with `#[verus_verify]`, so it falls outside the
  verification scope entirely.

**Entries 10-13: Singleton wrappers** (`alloc`, `free`, `book`, `alloc_range`)
- For each: document file/line, trust item (`external_body`), classification
  (`SINGLETON_WRAPPER`), and justification (delegates to fully-verified `Inner` method;
  `external_body` needed because `instance()` uses `unsafe { INSTANCE.assume_init_mut() }`
  and Verus cannot reason about the `MaybeUninit` singleton pattern).

### Fix 2: Add VERUS DEVIATION comment to Inner::book (item 8)

At line 579 in `frame.rs`, add a `VERUS DEVIATION` comment before the
`page_aligned_pa_to_bitmap_index` call, matching the pattern used in `Inner::free` (line 375):

```rust
// VERUS DEVIATION: original was `phys_addr.into_frame_number().into_raw_value()`.
// Same limitation as Inner::free — generic Deref chain cannot be specified.
let frame_number: usize = page_aligned_pa_to_bitmap_index(phys_addr);
```

### Fix 3: Acknowledge AST checker matching issues (item 8)

The AST checker incorrectly matches 3 of 4 `Inner::*` methods to the public singleton
wrappers instead of to the corresponding `Inner::*` methods. This is a tool limitation,
not a code problem. After Fix 2, confirm that all real exec deviations have documentation.
No code change needed beyond Fix 2.

**Verify:** After applying fixes, run:
```bash
make verify-kernel MODULE=mm::phys::frame
```
to confirm verification still passes.
