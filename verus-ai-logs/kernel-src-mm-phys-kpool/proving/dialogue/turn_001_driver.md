## Turn 1: Full Checklist Review

### Progress
- Done: (none yet — first pass)
- Current: All checklist items
- Remaining: (see individual results below)

---

### 1. No specs weakened from specification phase

**FAIL**

Compared `git show 295f4fe0b` (strip-external-body, start of proving) against HEAD.

#### `alloc_range` — 3 preconditions added

Original requires:
```rust
requires
    old(self).inv(),
```

Current requires:
```rust
requires
    old(self).inv(),
    count > 0,                          // ADDED
    count as int <= old(self)@.num_pages, // ADDED
    old(addrs)@.len() == 0,             // ADDED
```

These narrow the domain. The original spec handled `count == 0`, `count > num_pages`, and non-empty `addrs` via the `Err` path postcondition. Moving them to preconditions means callers that pass these values get **no guarantees at all** (the spec is vacuously satisfied), rather than the guaranteed `Err` path they had before.

#### `free` — 1 precondition added

Original requires:
```rust
requires
    old(self).inv(),
    addr.inv(),
```

Current requires:
```rust
requires
    old(self).inv(),
    addr.inv(),
    addr@ >= old(self)@.start,  // ADDED
```

Same issue: the original spec covered the case `addr@ < start` via the `Err` path; now callers get no guarantees for that case.

#### Fix Request

Restore the original preconditions. Handle the out-of-range / degenerate cases inside the function proof so the `Err` path postconditions still cover them. Specifically:

- **`alloc_range`**: Remove the three added `requires` clauses. In the proof, handle `count == 0` and `count > num_pages` as cases that produce `Err` (bitmap.alloc_range will return Err for those). For `addrs` non-empty, the function already checks `!addrs.is_empty()` and returns Err — the postcondition already covers `addrs@ == old(addrs)@` in the Err arm.
- **`free`**: Remove `addr@ >= old(self)@.start`. In the proof, show that when `addr@ < start`, the index computation wraps and `bitmap.clear` returns `Err`, which satisfies `!input_valid`.

If any of these cases is genuinely unprovable without the precondition (e.g., the subtraction in `free` truly underflows at the Rust level), document the reason with a `// VERUS REWRITE` comment explaining why the precondition was necessary, and file a finding.

---

### 2. Zero remaining admit()

**PASS**

```
grep -n 'admit' kpool.rs kpool.spec.rs kpool.proof.rs  →  0 matches
```

---

### 3. Zero external_body on this module's own functions — HARD RULE

**FAIL**

Two functions in `kpool.rs` carry `#[verus_verify(external_body)]`:

| Line | Function | Purpose |
|------|----------|---------|
| 74 | `pa_into_raw` | Wraps `pa.into_raw_value()` (method on `PageAligned<PhysicalAddress>`) |
| 83 | `frame_from_raw` | Wraps `FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(…)?))` |

Both functions are defined **in this module** (`kpool.rs`), so they fall under the hard rule regardless of the fact that they delegate to HAL types.

#### Fix Request

These wrappers exist because the underlying HAL methods (`into_raw_value`, `from_raw_value`, `PageAligned::from_address`, `FrameAddress::new`) do not have Verus specs. The correct fix is:

1. **Move the specs to the HAL types themselves** (preferred): Add `#[verus_spec]` to `PageAligned::into_raw_value()`, `PhysicalAddress::from_raw_value()`, `PageAligned::from_address()`, `FrameAddress::new()` in their own modules, with `#[verus_verify(external_body)]` there (where they are external-bottom trust boundaries for the HAL crate). Then remove `pa_into_raw` and `frame_from_raw` from kpool.rs entirely, calling the HAL methods directly.

2. **If HAL specs cannot be added** (e.g., out of scope): Move `pa_into_raw` and `frame_from_raw` to a shared HAL shim file (e.g., `hal/mem/verus_shims.rs`) so they are external-bottom trust boundaries of the HAL module, not of kpool. Then import and call them from kpool.

Either way, kpool.rs must end up with **zero** `external_body` annotations on its own functions.

---

### 4. Zero assume/assume_specification

**PASS**

```
grep -n 'assume\b\|assume_specification' kpool.rs kpool.spec.rs kpool.proof.rs  →  0 matches
```

---

### 5. No cfg-gated exec code (branches, expressions, match arms)

**PASS** (with note)

7 instances of `#[cfg(not(verus_keep_ghost))]` in kpool.rs — **all** on logging macros:

| Line | Statement |
|------|-----------|
| 132 | `error!("{reason}");` |
| 139 | `error!("{reason}");` |
| 143 | `info!("kernel pool: …");` |
| 202 | `error!("{error:?}");` |
| 334 | `error!("{reason}");` |
| 370 | `error!("{error:?} (count={count})");` |
| 649 | `error!("{error:?} (addr={addr:?})");` |

These are pure side-effect logging with no impact on control flow, state, or return values. This is the standard Verus pattern for handling format-string macros that Verus cannot parse. **No semantic divergence between verified and production code.**

---

### 6. Cheating audit — exact counts and locations

| Pattern | Count (this module) | Locations |
|---------|---------------------|-----------|
| `admit` | 0 | — |
| `external_body` | 2 | `pa_into_raw` (L74), `frame_from_raw` (L83) |
| `assume` | 0 | — |
| `assume_specification` | 0 | — |
| `cfg-gated exec code` | 7 | L132, L139, L143, L202, L334, L370, L649 (all logging macros) |

Global (whole kernel crate): assume=0 external_body=9 admit=0 trusted=0 cfg_gate=7.

---

### 7. Claimed Verus limitations with isolated reproducers

**PASS**

No Verus limitations are claimed anywhere in kpool.rs, kpool.spec.rs, or kpool.proof.rs.

---

### 8. Exec rewrites minimal and semantically equivalent

**PASS**

All exec-level changes from the pre-proving baseline (295f4fe0b):

| Original | Replacement | Equivalent? |
|----------|-------------|-------------|
| `base.into_raw_value()` | `pa_into_raw(base)` | ✓ (wrapper) |
| `self.base.into_raw_value()` | `pa_into_raw(self.base)` | ✓ (wrapper, 3 sites) |
| `FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(addr)?))` | `frame_from_raw(addr)?` | ✓ (wrapper, 2 sites) |
| `Ok(Inner { base, bitmap })` | `let inner = Inner { base, bitmap }; Ok(inner)` | ✓ (let binding for proof) |
| `Ok(()) => Ok(())` | `Ok(()) => { proof!{…} Ok(()) }` | ✓ (added proof block only) |

No `// VERUS REWRITE` comments present (none needed — changes are trivial wrapper substitutions).

---

### 9. Verification: 0 errors, 0 warnings

**FAIL** (minor)

```
verification results:: 7 verified, 0 errors ✓
warning: `kernel` (bin "kernel") generated 2 warnings ✗
```

Warnings:
1. `kpool.rs:87` — `result.is_Ok()` in `frame_from_raw` spec uses deprecated `is_Ok` method.
2. `phys.rs:264` — same deprecation in a different module (not our responsibility).

#### Fix Request

In `kpool.rs:87`, replace `result.is_Ok()` with the non-deprecated equivalent:

```rust
// Before (line 87):
        result.is_Ok(),

// After:
        result matches Ok(_),
```

Then re-run `make verify-kernel MODULE=mm::phys::kpool` and confirm 0 warnings from this module.

---

### Summary

| # | Checklist Item | Result |
|---|----------------|--------|
| 1 | No specs weakened | **FAIL** — 4 preconditions added across 2 functions |
| 2 | Zero admit | **PASS** |
| 3 | Zero external_body (own functions) | **FAIL** — 2 functions (`pa_into_raw`, `frame_from_raw`) |
| 4 | Zero assume/assume_specification | **PASS** |
| 5 | No cfg-gated exec code | **PASS** (logging-only, standard pattern) |
| 6 | Cheating audit | Reported above |
| 7 | Verus limitation reproducers | **PASS** (none claimed) |
| 8 | Exec rewrites minimal & equivalent | **PASS** |
| 9 | Verification 0 errors 0 warnings | **FAIL** — 1 warning from this module (deprecated `is_Ok`) |

**3 items require fixes before this review can pass.**
