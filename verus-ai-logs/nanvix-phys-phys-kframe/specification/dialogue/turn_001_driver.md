# Turn 1: Full Specification Checklist Walkthrough — `mm::phys::kframe`

In-scope functions: `KernelFrame::new`, `KernelFrame::base`, `KernelFrame::drop`.
Out-of-scope (TCB, allowlisted): `clear`, `deref`, `deref_mut`.

## Progress
- Done (PASS/verified): ALL 17 checklist items (see below).
- Current: final verdict.
- Remaining: none.

Baseline tool state:
- `make verify-kernel` → **exit 0, 23 verified, 0 errors** (commit `47c1f269a`).
- Modules verified: `mm::phys`, `mm::phys::frame`, `mm::phys::kframe`, `mm::phys::manager`, `mm::phys::upool`.
- Cheating (crate-wide aggregate): assume=0, admit=0, trusted=0, external_body=28, cfg_gate=9.
  Module-local external_body: **only `kframe.rs:141 clear`** (cheating-detail.txt:18), allowlisted.

---

## Verification per item

### 1. Every in-scope exec fn has requires/ensures — PASS
`grep verus_spec kframe.rs` = 3 (new, base, drop). All three carry `ensures`.
`fn_coverage.py` not present in repo; used verify.sh "Function Coverage" + manual grep.
`clear`/`deref`/`deref_mut` are out of scope (caller_analysis.md §"in scope"; tcb-allowed.md:23–25).

### 2. Caller coverage (caller_analysis.md) — PASS
- `new`: callers (manager.rs:354 `alloc_kernel_frame`, :430 `alloc_many_kernel_frames`) need
  `frame@ == base@` on Ok (to assert `allocated_frames.contains(frame@)` + contiguity) and
  no-consumption on Err (they `frame::free(base)` themselves). Spec: `Ok => frame@ == base@`,
  `Err => true`; `base` is `Copy` so no ownership is consumed on Err. ✓
- `base`: callers (kpage.rs:58/74) need `result@ == self@`. Spec provides it. ✓
- `drop`: callers (manager.rs:434, virt/manager.rs:753, RAII end-of-scope) need invariant
  preservation + never-unwind. Spec: `phys_view().inv()`, `no_unwind`, `opens_invariants none`. ✓

### 3. View consistency (view_design.md) — PASS
View = scalar `int` (`view = self.base@`), per view_design.md §"View Struct". Specs reference the
`int` view via `frame@`/`self@`/`result@` and the global `phys_view().inv()`. No struct field drift.

### 4. No tautological ensures (`Err(_) => true`) — PASS (with tool evidence)
`new` uses `Err(_) => true`. I treated this as the flagged pattern and **empirically tested** whether
a stronger clause is provable:
- Added `phys_view().inv()` to `new`'s ensures → `make verify-kernel` = **22 verified, 1 error**
  ("postcondition not satisfied" at `new`). Reverted.
Evidence that `true` is the *maximal* provable Err fact here:
- `phys_view()` is `uninterp spec fn phys_view() -> PhysMemView` (mod.spec.rs:171) — a single fixed
  value; equalities over it are tautological, and `new` has no `&mut`, so `old(phys_view())` does not exist.
- `new`'s deps (`identity_map_page` = `ensures true`; `from_raw_value` = trivial) establish nothing
  about `phys_view()`, so `phys_view().inv()` is unprovable here (proven above).
- `base: FrameAddress` is `Copy` → no linear ownership to state on Err.
- Consistent with the subsystem's own pattern: `manager.rs::alloc_kernel_frame` also uses `Err(_) => true`.
Conclusion: not a fabricated tautology — it is the honest maximum. PASS.

### 5. No subsumed ensures — PASS
`base ⇒ result@ == self@`, `new(Ok) ⇒ frame@ == base@`, `drop ⇒ phys_view().inv()` — none derivable
from any `inv()` + the others. (The handle has no `inv()` coupling result to self.)

### 6. Error paths have meaningful ensures — PASS
Match style (`Ok => … , Err => …`) is used; the Err clause carries the maximal provable fact (item 4).

### 7. No assume_specification for workspace-internal code — PASS (temporarily allowed)
One `assume_specification`: `<PageAligned<T> as Address>::from_raw_value`. `Address` is the external
**`sys::mm::Address`** library trait (src/libs/sys/.../address/mod.rs:31). Sanctioned in tcb-allowed.md:139–153
as the `sys`/`arch` library trust boundary (mirrors `arch::mem::PAGE_SIZE`, `Error::new`); `hal::mem` is
outside the current `mm::phys` verification target and the shim is removed when `hal::mem` is verified.
Falls under the cheating-item "assume_specification on external dependencies temporarily allowed".

### 8. vstd searched before assume_specification — PASS
`from_raw_value` is a custom newtype trait constructor; no vstd spec exists for it. Nothing to reuse.

### 9. Specs written for the caller — PASS
`frame@ == base@`, `result@ == self@`, `phys_view().inv()` are all directly consumed by `manager.rs`
caller proofs, which verify (0 errors). No HOW/implementation detail leaks into the contracts.

### 10. Trait obligations satisfied — PASS
- `View`: `type V = int`, `view = base@` — matches caller use (`allocated_frames.contains(frame@)`).
- `Drop`: `no_unwind` + `opens_invariants none` + `phys_view().inv()` — mirrors the `frame::free`
  shim contract (frame.rs:762–779), the semantic contract callers rely on for RAII rollback.
- `Deref`/`DerefMut`: TCB (raw identity-mapped slice), CR3 obligation documented in `# Safety`.

### 11. Spec completeness (advisory) — PASS
The `int` address view captures the sole caller-observable abstract state. The Err nondeterminism
(`true`) matches caller expectations (callers free `base` themselves; nothing else observable).

### 12. Loop invariants — PASS (N/A)
`new`/`base`/`drop` contain no loops. (The loops in `manager.rs` belong to other targets.)

### 13. No cheating on module's own functions — PASS (each addressed individually)
Module-local counts: admit=0, assume=0, trusted=0; external_body: **`clear` only** (kframe.rs:141,
cheating-detail.txt:18).
- `clear`: legitimately irreducible TCB — materializes `*mut u8` from `usize` and writes via the
  identity-map `memset` backend; Verus cannot model raw-memory writes. Allowlisted tcb-allowed.md:25.
- `deref` / `deref_mut`: plain `unsafe` Rust outside any `verus!` block (not annotated, not counted);
  allowlisted tcb-allowed.md:23–24.
- `assume_specification` `from_raw_value`: external dependency, temporarily allowed (item 7).
No cheating on a verified in-scope function (`new`/`base`/`drop` carry real, discharged contracts).

### 14. No specs weakened (spec drift) — PASS
No pre-existing specs existed (caller_analysis.md §"Pre-existing Specs": empty stubs). Implemented
contracts match view_design.md §"Spec Transition Functions" exactly. The doc's aspirational
`(on Err) phys_view() == old(phys_view())` note is unimplementable (no `old`; uninterp fixed value) —
its absence is not a weakening of any real guarantee.

### 15. Bug awareness — PASS (no bugs)
- `new`: returns before constructing `Self` on every error path → no partial/double ownership.
- `drop`: logs and swallows `free` errors → cannot unwind, honoring `no_unwind`.
No fundamentally incorrect code found.

### 16. Cross-module regression — PASS
`make verify-kernel` verifies all five `mm::phys` modules: 23 verified, 0 errors.

### 17. Verification + build — PASS
`make verify-kernel` → exit 0, 23 verified, 0 errors. Build: verus drives `cargo build`
("Finished `dev` profile"). No errors.

---

## Fix Request
None. Every checklist item is PASS with concrete tool evidence (verus runs, source/log reads,
and an empirical strengthening probe on `new`). No code change requested.

## Verdict
RESOLVED — all 17 specification checklist items pass.
