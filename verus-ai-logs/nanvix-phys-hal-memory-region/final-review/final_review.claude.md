# Final Verification Review — `hal-memory-region`

- **Reviewer:** Independent strict final verification (Claude)
- **Date:** 2026-06-15
- **Branch under review:** `verus-ai/hal-memory-region`
- **In-scope target functions (the ONLY functions in scope):**
  - `TruncatedMemoryRegion::start`
  - `MemoryRegion::start`
  - `TruncatedMemoryRegion::size`
  - `MemoryRegion::size`
- **Files reviewed:**
  - `src/kernel/src/hal/mem/types/region.rs`
  - `src/kernel/src/hal/mem/types/region.spec.rs`
  - `src/kernel/src/hal/mem/types/region.proof.rs`
  - `caller_analysis.md`, `view_design.md`, `tcb-allowed.md`
  - `bugs.md` — **absent** (confirmed below)

---

## 1. Spec Quality

The four target accessors carry these `#[verus_spec]` ensures:

| Function | Ensures | Verdict |
|----------|---------|---------|
| `MemoryRegion::start`          | `spec_addr(&result) == self@.start` | Faithful ✅ |
| `MemoryRegion::size`           | `result as int == self@.size`       | Faithful ✅ |
| `TruncatedMemoryRegion::start` | `spec_addr(&result) == self@.start` | Faithful ✅ |
| `TruncatedMemoryRegion::size`  | `result as int == self@.size`       | Faithful ✅ |

**Correctness vs. the View (region.spec.rs):**
- `MemoryRegion<T>::view()` defines `start = spec_addr(&self.start)` and
  `size = self.size as int` (closed view). The `start` body returns `self.start`
  (a `Copy` field read), so `spec_addr(&result) == spec_addr(&self.start) ==
  self@.start` discharges by congruence of `spec_addr`. The `size` body returns
  `self.size`, so `result as int == self.size as int == self@.size`. Both are
  exact, not approximations.
- `TruncatedMemoryRegion<T>::view()` forwards (`self@ == self.0@`). Its `start`
  and `size` delegate to the inner region's accessors, so the inner ensures
  propagate verbatim to the outer `self@`. Correct by delegation.

**Tautology / subsumption / weakening check:** None. Each ensures links the
*runtime* return value to the *abstract* View field — they are not trivially true
(e.g. not `result == result`) and not derivable without the body. There is no
redundancy or overlap among the four. The use of `spec_addr(&result)` rather than
`result@` is the correct projection because the exec impl blocks are
`impl<T: Address>` with no `View` bound (a `View<V=int>` bound would be
`cfg(verus_keep_ghost)`-gated and unsatisfiable in a normal build); this mirrors
the established `PageAligned<T>` pattern. `spec_addr` is the
`pub uninterp spec fn spec_addr<T: Address>(addr: &T) -> int` defined in
`page.spec.rs:31` and reachable as `crate::hal::mem::spec_addr`.

**Caller-need fidelity:** `start` callers consume the value via `into_raw_value`,
`into_frame_number`, `into_inner`, and as the `Ord` key — all of which need the
returned address to equal the constructed `start` (the abstract `self@.start`).
`size` callers compute `size - 1`, `size / FRAME_SIZE`, and `start + size`,
needing the returned length to equal `self@.size`. The ensures supply exactly
these equalities. The page-alignment / non-wrap geometry that some callers
additionally rely on is supplied through `inv()` (dimension 2), not duplicated
into the accessor ensures — correct separation per spec-design.

**Result:** Spec quality is correct, minimal, faithful, and well-documented. ✅

---

## 2. Caller Coverage

Every caller expectation in `caller_analysis.md` mapped to a spec (ensures or
`inv()` clause):

| # | Caller expectation | Covered by | Status |
|---|--------------------|-----------|--------|
| 1 | `TruncatedMemoryRegion::start` returns first valid address == constructed start | ensures `spec_addr(&result) == self@.start` | ✅ |
| 2 | Truncated `start` is page-aligned (frame/page base, no re-align) | `TruncatedMemoryRegion::inv()`: `self@.start % spec_page_size() == 0` (+ ensures binding result to `self@.start`) | ✅ |
| 3 | Range math `start + size - 1` does not overflow | `inv()` → `wf_geometry()`: `start >= 0 && start + size <= usize::MAX + 1` | ✅ |
| 4 | `MemoryRegion::start` returns `T` == constructed start (faithful clone) | ensures `spec_addr(&result) == self@.start` | ✅ |
| 5 | `TruncatedMemoryRegion::size` returns byte length == constructed size | ensures `result as int == self@.size` | ✅ |
| 6 | Truncated `size` is a non-zero multiple of page size | `inv()`: `wf_geometry()` (`size >= 1`) + `self@.size % spec_page_size() == 0` | ✅ |
| 7 | `size / FRAME_SIZE` exact (no remainder) | `inv()`: page-multiple `size` | ✅ |
| 8 | `MemoryRegion::size` == constructed size | ensures `result as int == self@.size` | ✅ |
| 9 | `start` is the stable `Ord` key for both region types | View documents `start` as `Ord` key; ensures binds `start()` to `self@.start`; `Ord::cmp` sorts by `self.start` consistently | ✅ |

**Covered: 9 / 9. Missing: none.**

Note: `inv()` is an `open spec fn` and is *not* attached as a `requires`/`ensures`
on the read-only accessors (they are `&self` reads that trivially preserve it).
For the four target accessors the in-scope obligation is purely the
start/size linkage, which is complete and correct. The alignment / no-wrap facts
become available to callers that establish `self.inv()` (a downstream
constructor-phase obligation). This is the intended layering per `view_design.md`
and is acceptable for the accessor phase.

**Result:** Full caller coverage. ✅

---

## 3. Proof Completeness

Counts across the 3 region files (`region.rs`, `region.spec.rs`,
`region.proof.rs`):

- `admit()`: **0** — no blocker.
- `external_body`: **0** — none to reconcile against TCB list.
- `region.proof.rs` is `verus! { } // verus!` (empty) — no proof obligations
  deferred or stubbed.

The four accessors are body-verified with no `admit`, no `assume`, no
`external_body`. `MemoryRegion::start` is discharged by the documented
`Copy`-field-read rewrite (dimension 5), not by an axiom.

**Result:** Proof complete, no `admit()`, no stubs. ✅

---

## 4. TCB Compliance

There are **0** `external_body` items in the three region files, so nothing must
appear in `tcb-allowed.md` for this module. No new `external_body` is introduced
or justified. (The repository-wide `external_body=25` reported in the
`kernel::all` build-harness commit messages are pre-existing TCB-allowed items in
*other* modules — `frame.rs`, `manager.rs`, `kframe.rs`, `mod.rs`, `upool.rs`,
`page.rs`, `phys.rs` — all enumerated in `tcb-allowed.md`; none are in scope
here.)

**Result:** Compliant. ✅

---

## 5. AST Consistency

Command run:
```
ast_consistency.py --base-ref verus-ai/hal-phys-address region.rs summary
```
Result: `matched=27 mismatched=1 missing=0 extra=0` — `Consistent: ❌ NO` due to
the single expected mismatch.

**The one MISMATCH — `MemoryRegion::start`** (diff confirmed):
```
-        self.start.clone()
+        // VERUS REWRITE: original exec body was `self.start.clone()`. ...
+        self.start
```
Independent semantic-equivalence assessment:
- The `Address` trait requires `Self: ... + Copy + ...`
  (`src/libs/sys/src/sys/mm/address/mod.rs:33`, independently verified:
  `Self: core::fmt::Debug + Clone + Copy + PartialEq + Eq + PartialOrd + Ord`).
- For a `Copy` type, `Clone::clone(&x)` is by contract a bitwise copy returning a
  value equal to `*x`; therefore `self.start.clone()` ≡ `self.start`. The two
  produce identical runtime values.
- The rewrite is driven by a **genuine Verus limitation**: `Clone::clone` on a
  generic `T: Address` has no spec relating `spec_addr(&result)` to
  `spec_addr(&self.start)`, so the postcondition cannot be discharged through
  `.clone()`. The in-code comment includes a minimal reproducer
  (`40 verified, 1 errors`, postcondition not satisfied at region.rs:218).
- Per `ast-consistency` skill (steps 3–4): a real, reproduced Verus limitation
  with a minimal, semantically-equivalent exec change, fully documented with a
  `VERUS REWRITE` comment and reproducer, is an **acceptable deviation**, not a
  violation. All 27 other functions and all 3 structs MATCH.

**Result:** The single mismatch is the expected, verified, semantically-equivalent
`Copy`-clone rewrite. **Not a blocker.** ✅

Supplementary read-only checks:
- `spec_drift.py git-diff ... --before HEAD`: **0** functions with changes, **0**
  contract drift, 0 ensures removed, 0 requires added. ✅
- `fn_coverage.py`: 17 source exec fns / 17 verus exec fns, **17 matched, 0
  missing, 0 extra**. ✅

---

## 6. Verification

Per instruction, `make verify`, `make verify-kernel`, and `make build` were **not**
run locally (a central build is in progress; running mine would corrupt it). I
relied on the AUTHORITATIVE central results:
- `make verify-kernel MODULE=hal::mem::types::region` ⇒ **PASS, exit 0, 0 errors.**
- `make verify` (cross-module) and `make build` ⇒ assumed **PASS** (no
  override provided).

This review's static evidence (0 `admit`/`assume`/`external_body`, no contract
drift, full fn coverage, faithful ensures) is consistent with the central PASS.

**Result:** Module verifies (central). ✅ *(Relied on central results; did not run
verification locally.)*

---

## 7. Guardrails

Exact `grep` counts across `region.rs` + `region.spec.rs` + `region.proof.rs`:

| Token | Count | Locations |
|-------|------:|-----------|
| `admit` | **0** | — |
| `assume` | **0** | — |
| `external_body` | **0** | — (note: `external_derive` appears in `#[verus_verify(external_derive)]` — a distinct token, not `external_body`) |
| `assume_specification` | **0** | — |
| cfg-gated exec code | **0 divergent** | only `region.rs:9` & `region.rs:11` — `#[cfg(verus_keep_ghost)] include!("region.spec.rs")` / `include!("region.proof.rs")`, the standard spec/proof include guard, not cfg-divergent exec bodies |

`admit > 0`? No. `assume > 0`? No. No blocker.

**Result:** All guardrails clean. ✅

---

## 8. Bug Reconciliation

`bugs.md` is **absent** (`cat` → "No such file or directory", confirmed). No bugs
were recorded.

Assessment of the four targets: they are pure, read-only field accessors
(`start: T` / `PageAligned<T>` and `size: usize`) that never mutate, never fail,
and never allocate. There is **no true code defect** in any of them. The only
deviation from the original source is the `MemoryRegion::start`
`.clone()` → `self.start` rewrite, which is a **Verus front-end limitation**
worked around by a semantically-equivalent change — per the `bug-reporting` skill,
a missing spec or a verifier limitation is explicitly **NOT a bug** and must not be
recorded as one.

**Result:** The absence of `bugs.md` is **correct.** ✅

---

## Issues (highest priority first)

None. No blockers and no non-blocking concerns identified.

- (Informational, not an issue) `inv()` remains an `open spec fn` not asserted on
  the accessors; the alignment/no-wrap facts are available only where a caller
  establishes `self.inv()`. This is the intended phase layering and does not
  affect the correctness or completeness of the four in-scope accessor contracts.

---

## Scorecard

| Dimension | Verdict |
|-----------|---------|
| 1. Spec quality | ✅ Faithful, minimal, non-tautological |
| 2. Caller coverage | ✅ 9/9, none missing |
| 3. Proof completeness | ✅ 0 admit, 0 stubs |
| 4. TCB compliance | ✅ 0 external_body in scope |
| 5. AST consistency | ✅ 1 expected, semantically-equivalent rewrite |
| 6. Verification | ✅ Central PASS (relied upon) |
| 7. Guardrails | ✅ admit/assume/external_body/assume_specification all 0 |
| 8. Bug reconciliation | ✅ bugs.md correctly absent |

No blocker exists: `admit == 0`, `assume == 0`, no unlisted `external_body`, the
single AST mismatch is a verified semantically-equivalent rewrite for a genuine
Verus limitation, all critical caller expectations are covered, and central
verification PASSes.

## Result: PASS
