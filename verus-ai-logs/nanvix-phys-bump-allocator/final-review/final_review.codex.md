# Final Verification Review — `bump-allocator`

## Scope
Reviewed only:
- `src/libs/bump_allocator/src/lib.rs`
- `src/libs/bump_allocator/src/lib.spec.rs`
- `src/libs/bump_allocator/src/lib.proof.rs`
- `verus-ai-logs/nanvix-phys-bump-allocator/{caller_analysis.md,view_design.md,bugs.md}`
- `verus-ai-logs/tcb-allowed.md`

---

## 1) SPEC QUALITY

### Findings

### ✅ Good
- `align_up` has a clear total contract tied to `align_up_spec` (`lib.rs:126-133`), and implementation guards zero divisor (`lib.rs:134-138`).
- `align_up_spec` encodes least-multiple/overflow semantics (`lib.spec.rs:57-68`).
- `alloc_as` exposes typed guard semantics for `SizeMismatch`/`AlignmentMismatch` (`lib.rs:362-364`).
- `BumpView::inv()` is non-trivial and uses math types (`nat`/`int`) (`lib.spec.rs:116-133`), matching view-design goals (`view_design.md:107-139`).

### ❌ Blockers
1. **Tautological/underspecified error arms on external-top API**
   - `alloc`: `Err(_) => true` (`lib.rs:283`) gives no failure-path meaning.
   - `alloc_as`: fallback `Err(_) => true` (`lib.rs:364`) leaves propagated `alloc` errors unconstrained.
   - This violates caller-facing error-path rigor expected in spec-design (one-sided error spec).

2. **Uniqueness/non-aliasing not delivered in API contracts**
   - `alloc`/`alloc_as` only guarantee single-slot alignment+bounds over `slot_ref_addr(slot)` (`lib.rs:276-282`, `lib.rs:353-361`).
   - No postcondition relates returned slot to prior successful allocations.
   - Caller expectation requires distinct/non-aliasing slots (`caller_analysis.md:73-76`, `117-119`).

3. **Monotone-capacity / no-spurious-consumption not in method contracts**
   - `lemma_alloc_transition` and `lemma_exhausted_boundary` are abstract over `BumpView` (`lib.proof.rs:107-133`) but `alloc`/`alloc_as` contracts do not express/bridge `bump_view(self)` pre/post transitions.
   - Caller expectation requires exhaustion boundary and no slot consumption on failure (`caller_analysis.md:77-80`, `123-129`).

4. **Potential vacuity/soundness gap from uninterpreted bridge**
   - `bump_view`, `slot_ref_addr`, `base_of` are uninterpreted (`lib.spec.rs:41`, `50`, `177`).
   - Because `alloc`/`alloc_as` are `external_body` (`lib.rs:271`, `348`), ensures over uninterpreted `slot_ref_addr` can be assumed without tying to concrete pointer identity/aliasing behavior.
   - Net effect: alignment/in-bounds obligations are abstractly asserted, but aliasing/identity guarantees are not concretely connected to returned references.

### `align_up_spec` vs `div_ceil` assumption consistency
- `assume_specification [<usize>::div_ceil]` states ceil-division formula (`lib.spec.rs:28-33`), and `align_up_spec` uses the same arithmetic shape with overflow cut-off (`lib.spec.rs:61-66`).
- Implementation uses `div_ceil(...).checked_mul(...)` (`lib.rs:137`), consistent with `None` on multiply overflow.
- **Consistency verdict: PASS.**

---

## 2) CALLER COVERAGE

Source of expectations: `caller_analysis.md:53-129`.

### Coverage matrix (strict)
- `align_up`: **4/4 covered**
- `alloc`: **3/8 covered**
- `alloc_as`: **4/7 covered**
- `BssStorage::as_mut_ptr`: **1/3 covered**

**Total covered: 12/22**

### Missing properties
- `alloc` uniqueness/non-aliasing across calls (`caller_analysis.md:73-76`, `117-119`) — missing from `alloc` ensures (`lib.rs:275-284`).
- `alloc` monotone-capacity/exhausted boundary (`caller_analysis.md:79`, `123-125`) — not specified in `alloc` ensures.
- `alloc` no spurious consumption on failure (`caller_analysis.md:77-80`, `128-129`) — `Err(_) => true`.
- `alloc` specific error meaning for `Exhausted`/`Overflow`/`OutOfBounds`/`Misaligned` (`caller_analysis.md:79-80`) — unspecified.
- `alloc_as` propagated `alloc`-error semantics and no-consumption guarantee (`caller_analysis.md:91-94`, `128-129`) — fallback `Err(_) => true`.
- `alloc_as` cross-call uniqueness/non-aliasing (`caller_analysis.md:85-87`, `117-119`) — not in ensures.
- `as_mut_ptr` storage-size/writability/alignment semantic duties (`caller_analysis.md:37-43`, `100-102`) — only base equality is specified (`lib.rs:200-204`).

### Adequacy judgment for the requested invariant set
- **In-bounds:** partially covered per returned slot (`lib.rs:280-282`, `359-361`).
- **Alignment:** partially covered per returned slot (`lib.rs:279`, `358`).
- **Stable-size:** covered for `alloc_as` (`lib.rs:356-357`, `362-363`).
- **Uniqueness / non-aliasing:** **not caller-delivered** (only abstract geometry lemma, no tie to returned refs).
- **Monotone-capacity:** **not caller-delivered** (lemma-only, no method transition spec).

---

## 3) PROOF COMPLETENESS

### Counts in in-scope files
- `admit()`: **0**
- `external_body`: **2**
  - `src/libs/bump_allocator/src/lib.rs:271` (`alloc`)
  - `src/libs/bump_allocator/src/lib.rs:348` (`alloc_as`)

**Blocker rule check:**
- `admit() > 0`? **No**.
- `external_body` outside TCB? See section 4.

---

## 4) TCB COMPLIANCE

`external_body` in scope:
- `FixedSizeBumpAllocator::alloc` (`lib.rs:271`)
- `FixedSizeBumpAllocator::alloc_as` (`lib.rs:348`)

Both are pre-listed in TCB allow-list:
- `tcb-allowed.md:16-23`

**TCB verdict: PASS (no unlisted `external_body`).**

---

## 5) AST CONSISTENCY

Command run:
```bash
cd /home/ruize/nanvix-phy-specs-bottom-up && \
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py \
  --base-ref verus-ai-prove-bottom-up src/libs/bump_allocator/src/lib.rs count
```

Result (trimmed):
- `✅ Consistent: 12 functions, 7 structs match.`

`// VERUS REWRITE` / `// VERUS DEVIATION` scan:
- none found in the three in-scope files.

**AST verdict: PASS.**

---

## 6) VERIFICATION

### `make verify-bump-allocator`
Command run from repo root.

Result (trimmed):
- `Exit code : 0`
- `status: CHEATING_DETECTED`
- `cheating: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0`
- Verus wrapper reported cached run (`cached (no recompilation)`, `—`).

**PASS/FAIL:**
- **Execution PASS** (exit code 0)
- **Error count:** wrapper reported `—` (cached, no explicit numeric error line).

### `make build`
Result:
- `make: Nothing to be done for 'build'.`
- Exit code 0.

**Build verdict: PASS.**

---

## 7) GUARDRAILS COMPLIANCE

Across `lib.rs`, `lib.spec.rs`, `lib.proof.rs`:

- `admit`: **0**
- `assume(...)`: **0**
- `external_body`: **2**
  - `lib.rs:271`, `lib.rs:348`
- `assume_specification`: **1**
  - `lib.spec.rs:28` (`<usize>::div_ceil`)
- cfg-gated real exec code: **0**
  - Present cfgs are allowed-only:
    - `#![cfg_attr(not(any(test, feature = "std")), no_std)]` (`lib.rs:83`)
    - `#[cfg(verus_keep_ghost)]` includes (`lib.rs:101`, `105`)
    - `#[cfg(test)]` test module (`lib.rs:392`)

### `div_ceil` vstd search
Searches run over multiple local vstd trees (`/home/ruize/toolchain/verus/vstd`, `/home/ruize/verus/vstd`, `/home/ruize/verus-bin/vstd`) found **no `div_ceil` spec hit**.

**Judgment:** current `assume_specification [<usize>::div_ceil]` is acceptable as external-bottom std boundary (pending future vstd support).

---

## 8) SPEC DRIFT

Command run:
```bash
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff \
  /home/ruize/nanvix-phy-specs-bottom-up/src/libs/bump_allocator/src/lib.rs \
  --before HEAD
```

Result (trimmed):
- `Contract drift: 0`
- `✅ No contract drift detected.`
- Exit code 0.

**Spec drift verdict: PASS.**

---

## 9) BUG RECONCILIATION

`bugs.md` says no code bugs found (`bugs.md:5-18`).

Consistency check against code:
- `align_up` zero-divisor and overflow guards present (`lib.rs:134-138`).
- `alloc` uses checked arithmetic and bounds/alignment guards (`lib.rs:293-317`).
- `alloc_as` size/alignment guards before cast (`lib.rs:368-373`).

No contradictory concrete code bug found in reviewed scope.

However, this review found **specification quality/coverage defects** (not code defects), which are currently undocumented in `bugs.md` because they are not runtime-code bugs.

**Bug reconciliation verdict: code-bug record is consistent; spec-quality blockers remain.**

---

## Issues ordered by severity

### BLOCKER-1 — API contract misses uniqueness/non-aliasing and monotone-capacity guarantees
- Evidence: `alloc`/`alloc_as` contracts (`lib.rs:275-285`, `351-365`) + caller requirements (`caller_analysis.md:73-80`, `117-129`) + lemmas only abstract (`lib.proof.rs:107-133`).

### BLOCKER-2 — Error-path under-specification (`Err(_) => true`)
- Evidence: `lib.rs:283`, `lib.rs:364`.
- Missing caller-meaningful guarantees for propagated failure cases and no-consumption semantics.

### BLOCKER-3 — Uninterpreted address/view bridge leaves caller-facing guarantees weak
- Evidence: `slot_ref_addr`/`bump_view`/`base_of` uninterpreted (`lib.spec.rs:41`, `50`, `177`), while external-body APIs assert properties only over those symbols (`lib.rs:276-282`, `353-361`).

---

RESULT: FAIL
