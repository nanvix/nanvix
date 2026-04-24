# Turn 1: Full Checklist Review

## Progress
- Done: (none yet — first pass)
- Current: All items
- Remaining: (see below)

---

## 1. Zero admit() remaining

**PASS** ✅

Searched all three files (`kpool.rs`, `kpool.spec.rs`, `kpool.proof.rs`) with
`grep -n "admit"` — zero matches.

---

## 2. Zero assume() remaining

**PASS** ✅

Searched all three files with `grep -n "assume\b"` — zero matches.
(The `assume_init_mut` in `instance()` is a standard-library call, not a Verus `assume`.)

---

## 3. Zero trusted functions

**PASS** ✅

Searched all three files with `grep -n "trusted"` — zero matches.

---

## 4. Zero exec_allows_no_decreases_clause

**PASS** ✅

Searched all three files with `grep -n "exec_allows_no_decreases_clause"` — zero matches.

---

## 5. Zero cfg-gated exec code

**PASS** ✅

The cheating scanner reports `cfg-gated exec code: 1` for the kpool module.
The flagged instance is line 807:

```rust
#[cfg(verus_keep_ghost)]
verus! {
    impl View for KernelFrame { ... }
}
```

This is a **spec/proof block** (a `View` implementation), not exec code. The
scanner's heuristic doesn't recognize `verus! {}` as ghost-only code and flags
it. All other `#[cfg(not(verus_keep_ghost))]` usages gate only `error!`,
`info!`, `trace!` macros (allowed logging).

---

## 6. Zero external_body on user functions

**PASS** ✅

The global cheating scan reports `external_body=8`, but `cheating-detail.txt`
shows all 8 are in **dependency modules** (`frame.rs`, `hal/mem`), not in
`kpool.rs`. Searching kpool.rs/spec.rs/proof.rs for `external_body` yields
zero matches. The four `Inner` methods (`new`, `alloc`, `alloc_range`, `free`)
that previously had `external_body` are now fully body-verified.

---

## 7. Challenge surviving trust items in trust.md

**PASS** ✅

`trust.md` declares:

- **Dependency trust**: `pa_into_raw` (STDLIB_WRAPPER) — a single-line wrapper
  `pa.into_raw_value()` marked `external_body` because Verus cannot resolve the
  generic trait method chain. Classification is correct.
- **kpool-specific trust**: "None" — verified by finding zero `external_body`,
  `trusted`, `admit`, or `assume` in kpool files.

No workarounds or limitations disguised as trust.

---

## 8. AST consistency: zero mismatches

**FAIL** ❌

```
python3 ast_consistency.py kpool.rs summary
→ Consistent: ❌ NO (matched=13 mismatched=4 missing=0 extra=0)
```

Four functions have exec-level mismatches: `Inner::new`, `Inner::alloc`,
`Inner::alloc_range`, `Inner::free`.

### Analysis of exec changes per function

#### Inner::new — 2 exec changes

1. **`base.into_raw_value()` → `pa_into_raw(base)`** — Semantically equivalent
   (wrapper calls same underlying method). Needed because Verus cannot resolve
   the generic trait method. **Missing VERUS REWRITE comment.**

2. **`Ok(Inner { base, bitmap })` → `let inner = Inner { ..}; Ok(inner)`** —
   Pre-approved deviation (intermediate binding for ensures/proof). **Missing
   comment documenting the pre-approved deviation.**

#### Inner::alloc — 2 exec changes

1. **`self.base.into_raw_value()` → `pa_into_raw(self.base)`** — Same as above.
   **Missing VERUS REWRITE comment.**

2. **`FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(addr)?)?)` → `FrameAddress::from_raw_value(addr)?`** — Semantically identical: `from_raw_value` body is
   `Self(PageAligned::from_address(PhysicalAddress::from_raw_value(raw_addr)?)?)` which equals `FrameAddress::new(...)`. This is a convenience-API call substitution. **Missing VERUS REWRITE comment.**

#### Inner::alloc_range — 4 exec changes

1. **`self.base.into_raw_value()` → `pa_into_raw(self.base)`** — Same pattern.
   **Missing VERUS REWRITE comment.**

2. **Same FrameAddress construction change** as `alloc`. **Missing VERUS REWRITE
   comment.**

3. **`if count == 0 { return Err(...); }`** — New guard. Has `// VERUS REWRITE`
   comment. ✓

4. **`let num_pages = ...; if count > num_pages { return Err(...); }`** — New
   guard. Has `// VERUS REWRITE` comment. ✓

#### Inner::free — 2 exec changes

1. **`self.base.into_raw_value()` → `pa_into_raw(self.base)`** — Same pattern.
   **Missing VERUS REWRITE comment.**

2. **`if addr.into_raw_value() < pa_into_raw(self.base) { return Err(...); }`**
   — New underflow guard. Has `// VERUS REWRITE` comment. ✓

---

## 9. All exec rewrites have VERUS REWRITE comment and minimal reproducer

**FAIL** ❌

The three VERUS REWRITE guards (count==0, count>num_pages, addr<base) are
properly documented. However, the following exec rewrites are **missing
comments**:

| Change | Affected Functions | Comment? |
|--------|-------------------|----------|
| `.into_raw_value()` → `pa_into_raw()` | new, alloc, alloc_range, free | ❌ |
| Multi-step FrameAddress construction → `from_raw_value()` | alloc, alloc_range | ❌ |
| `Ok(Inner{..})` → `let inner = ..; Ok(inner)` | new | ❌ |

**Fix request**: Add a `// VERUS REWRITE` (or `// VERUS DEVIATION` for
pre-approved) comment before each undocumented exec change, with a brief
reason. Specifically:

- Before each `pa_into_raw(...)` call, add:
  `// VERUS REWRITE: wrapper needed because Verus cannot resolve generic trait into_raw_value()`
  (Once per function is enough, not per-call.)

- Before each `FrameAddress::from_raw_value(addr)?`, add:
  `// VERUS REWRITE: convenience API equivalent to FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(addr)?)?)` — or equivalently, document that this is the same call chain wrapped in a function.

- Before `let inner = Inner { base, bitmap };` in `new`, add:
  `// VERUS REWRITE: intermediate binding needed for ensures clause (pre-approved deviation)`

Then re-run: `python3 ast_consistency.py kpool.rs count` to confirm the tool
still reports the same set (tool will still flag mismatches — that's expected
for documented deviations, but comments must be present).

---

## 10. For each surviving external_body: confirm not masking a defect

**PASS** ✅ (N/A for kpool)

There are zero `external_body` items in kpool.rs. The dependency `pa_into_raw`
is a trivial 1-line wrapper and cannot mask a defect.

---

## 11. trust.md contains only declared external-bottom trust boundaries

**PASS** ✅

trust.md lists exactly one dependency trust boundary (`pa_into_raw`) and zero
kpool-specific trust. No workarounds or limitations disguised as trust entries.

---

## 12. Verification: run verify and build — 0 errors, 0 warnings

**PASS** ✅

- **Build**: `./z build` — exits 0, `[OK] Build complete.`
- **Verification**: `make verify-kernel MODULE=mm::phys::kpool` — Verus exits 0.
  Output: `Finished dev profile ... in 0.24s` (cached, no errors).
- The `CHEATING_DETECTED` status in the script output is driven by **global**
  counts (`external_body=8, cfg_gate=7`) from dependency modules, not from
  kpool itself.

---

## Summary

| # | Item | Status |
|---|------|--------|
| 1 | Zero admit() | ✅ PASS |
| 2 | Zero assume() | ✅ PASS |
| 3 | Zero trusted | ✅ PASS |
| 4 | Zero no_decreases | ✅ PASS |
| 5 | Zero cfg-gated exec | ✅ PASS |
| 6 | Zero external_body (user) | ✅ PASS |
| 7 | Trust items challenged | ✅ PASS |
| 8 | AST consistency | ❌ FAIL — 4 mismatches, need VERUS REWRITE comments |
| 9 | Exec rewrites documented | ❌ FAIL — 7 undocumented exec rewrites |
| 10 | external_body not masking defect | ✅ PASS (N/A) |
| 11 | trust.md clean | ✅ PASS |
| 12 | Verify + build green | ✅ PASS |

## Fix Request

Items 8 and 9 are the same root issue: missing `VERUS REWRITE` comments on
exec changes. Add comments as described in item 9 above, then re-run AST
consistency to confirm no other unexpected changes exist.

Specifically, add the following comments in `kpool.rs`:

1. **`Inner::new` (line ~108)**: Before `if !is_valid_physical_region(pa_into_raw(base), kpool_size)`:
   ```rust
   // VERUS REWRITE: pa_into_raw wrapper needed because Verus cannot resolve generic trait .into_raw_value()
   ```

2. **`Inner::new` (line ~127)**: Before `let inner = Inner { base, bitmap };`:
   ```rust
   // VERUS REWRITE: intermediate binding for proof block (pre-approved deviation)
   ```

3. **`Inner::alloc` (line ~209)**: Before `let addr: usize = pa_into_raw(self.base) + ...`:
   ```rust
   // VERUS REWRITE: pa_into_raw wrapper needed; FrameAddress::from_raw_value equivalent to FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(addr)?)?)
   ```

4. **`Inner::alloc_range` (line ~383)**: Before `let base_addr: usize = pa_into_raw(self.base) + ...`:
   ```rust
   // VERUS REWRITE: pa_into_raw wrapper needed (see Inner::alloc)
   ```

5. **`Inner::alloc_range` (line ~464)**: Before `let frame: FrameAddress = FrameAddress::from_raw_value(addr)?;`:
   ```rust
   // VERUS REWRITE: from_raw_value is equivalent convenience API (see Inner::alloc)
   ```

6. **`Inner::free` (line ~624)**: Before `let index: usize = (addr.into_raw_value() - pa_into_raw(self.base)) / ...`:
   ```rust
   // VERUS REWRITE: pa_into_raw wrapper needed (see Inner::alloc)
   ```

After adding these comments, re-run:
```bash
make verify-kernel MODULE=mm::phys::kpool
./z build
```
Both must still exit 0.
