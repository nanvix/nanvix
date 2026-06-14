## Turn 1: Full checklist walkthrough — bump-allocator proving

### Progress
- Done (PASS): all 10 checklist items.
- Current: final verdict — RESOLVED.
- Remaining: none.

### Verification

Tools run from repo root:
- `make verify-bump-allocator` → Exit code 0.
  Cheating: `assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
- `python3 .../scripts/spec_drift.py check nanvix-phys-bump-allocator` (proving baseline
  `a82c44f6a741` → HEAD) → Exit 0, **0 contract drift** on `lib.rs`.
- `spec_drift.py git-diff lib.spec.rs --before a82c44f6a741` → Exit 0, 0 changes.
- `spec_drift.py git-diff lib.proof.rs --before a82c44f6a741` → 3 proof BODIES modified
  (all `admit()` → real proofs); **ensures removed: 0, requires added: 0**.
- `git diff a82c44f6a741 -- src/libs/bump_allocator/src/lib.rs` → empty (exec untouched).
- Cross-module verify (`bitmap`, `sys`, `nanvix-slab`, `bump-allocator`, `arch`) → all Exit 0.
- `cargo build` (bump_allocator) → Exit 0, no warnings.

Item-by-item:

1. **No specs weakened** — PASS. Drift tool reports 0 contract drift on `lib.rs` and
   `lib.spec.rs`. The only `lib.proof.rs` changes are the three lemma bodies going from
   `admit()` to real proofs (`lemma_geometry`, `lemma_alloc_transition`,
   `lemma_exhausted_boundary`); their `requires`/`ensures` are byte-identical to baseline.
   Discharging `admit()` is the goal of proving, not weakening.

2. **Zero remaining admit()** — PASS. `admit=0`. Only textual hit is a comment in
   `lib.proof.rs:6` ("Bodies are `admit()` placeholders…"), no executable `admit()`.

3. **Zero external_body unless in tcb-allowed** — PASS. Exactly 2, each individually checked:
   - `lib.rs:286 FixedSizeBumpAllocator::alloc` — registered in `tcb-allowed.md`
     (materializes `&'static mut [u8; N]` from a backend `usize as *mut`). Carries a full
     `#[verus_spec]` (`requires bump_view(self).inv()`; `ensures` alignment + in-bounds over
     `slot_ref_addr`). Not contract-free.
   - `lib.rs:367 FixedSizeBumpAllocator::alloc_as` — registered in `tcb-allowed.md`
     (delegates to `alloc`, re-materializes `&'static mut MaybeUninit<T>`). `ensures` adds
     the sound `size_of::<T>()==N` / `align_of::<T>()<=A` guard arms (guarded in exec).
   Both sound (no false-deriving postcondition; distinct slots → distinct uninterpreted
   `slot_ref_addr`).

4. **Zero assume/assume_specification except std/external bottom** — PASS. `assume=0`.
   One `assume_specification [ <usize>::div_ceil ]` in `lib.spec.rs:28` — a `core` std
   function (external-bottom trust boundary, explicitly allowed by the checklist exception).
   Spec is faithful: `requires y != 0`, `ensures result == (x+y-1)/y`.

5. **No cfg-gated exec code** — PASS. `cfg_gate=0`. The only `cfg` uses are the crate-level
   `#![cfg_attr(not(...), no_std)]`, `#[cfg(verus_keep_ghost)]` on the spec/proof `include!`s
   (ghost-only), and `#[cfg(test)]` on the unit-test module. No cfg-gated exec
   branch/expression/match arm.

6. **Cheating audit (exact counts/locations)** —
   - `admit`: 0.
   - `external_body`: 2 → `lib.rs:286 alloc`, `lib.rs:367 alloc_as` (both in `tcb-allowed.md`).
   - `assume`: 0; `assume_specification`: 1 → `lib.spec.rs:28 <usize>::div_ceil` (std boundary).
   - cfg-gated exec: 0.

7. **Claimed Verus limitation has isolated reproducer** — PASS.
   `verus-unsupported.md` documents each construct with an isolated trigger + exact error:
   (1) complex break (`break <value>;` in the CAS loop), (2) `usize`→`*mut [u8; N]` cast
   (`alloc`) and raw-pointer deref (`alloc_as`) — each shown separately, (3) `AtomicUsize`
   value not spec-readable. These justify the `external_body` trust boundary.

8. **Exec rewrites minimal & semantically equivalent / `// VERUS REWRITE`** — PASS.
   `git diff` shows `lib.rs` is unchanged vs the proving baseline: no exec rewrites were made,
   so no `// VERUS REWRITE` markers are required. Nothing to weaken or audit.

9. **Cross-module regression** — PASS. All five Verus crates verify Exit 0
   (`bitmap`, `sys`, `nanvix-slab`, `bump-allocator`, `arch`). bump-allocator's exec is
   untouched and only its proof bodies changed, so no regression is introduced. Cheating
   patterns reported in other crates (e.g. `arch` admit=1/external_body=3) are those crates'
   own pipeline scope, not regressions from this change.

10. **Verification + build, 0 errors / 0 warnings** — PASS.
    `make verify-bump-allocator` Exit 0 (no error/warning lines); `cargo build` Exit 0, no
    warnings.

### Fix Request

None. All 10 checklist items PASS with tool evidence. No actionable fixes required.

Minor non-blocking note (no code change demanded): `verus-unsupported.md` §2 still says
"`external_body` … is forbidden here (`bump_allocator` is not in `tcb-allowed.md`)", which is
now stale — `alloc`/`alloc_as` were subsequently added to `tcb-allowed.md`. This is a doc
comment only and does not affect any checklist verdict.
