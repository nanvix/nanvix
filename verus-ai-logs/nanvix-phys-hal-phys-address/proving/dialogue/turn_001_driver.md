## Turn 1: Full proving checklist walk-through — hal-phys-address

Proof target: `src/kernel/src/hal/mem/types/address/phys.rs`
(+ `phys.spec.rs`, `phys.proof.rs`). In-scope verified functions:
`from_mmio_address`, `from_number`, `into_frame_number`, plus the
`View`/`inv` material.

### Progress
- Done (all PASS):
  1. No specs weakened
  2. Zero admit
  3. Zero external_body (outside TCB)
  4. Zero assume/assume_specification (except external-bottom boundaries)
  5. No cfg-gated exec code
  6. Cheating audit (counts + locations)
  7. Verus-limitation isolated reproducer
  8. Exec rewrites minimal / semantically equivalent
  9. Cross-module regression (full `make verify`)
  10. Verification + build: 0 errors / 0 warnings
- Current: none remaining
- Remaining: none

### Verification

Commands run and evidence:

- `make verify-kernel` (cached) only re-verified `mm::phys*`; I forced a clean
  re-verify by `touch`ing `phys.rs`:
  `make verify-kernel` → **37 verified, 0 errors, 0 warnings** (log
  `verus_2026-06-15_12-52-36.log`).
- Full `make verify` (bitmap, sys, nanvix-slab, bump-allocator, kernel):
  **every crate exit 0, 0 verification errors**. `sys` = CLEAN. The
  `CHEATING_DETECTED` status comes solely from pre-existing `external_body`
  in out-of-scope modules (`mm/phys/*`, bump-allocator, nanvix-slab, bitmap),
  all governed by `verus-ai-logs/tcb-allowed.md`.
- `./z build -- all` → **Build complete**, no compiler warnings (the lone
  "Warning: Sysroot directory ... not found" is a build-script symlink note,
  not a compiler diagnostic).
- `git diff cdbf5ab..HEAD -- src/` and `git diff b35c5d5..HEAD -- src/` are
  **empty**: the proving phase introduced no source changes; the proofs were
  completed and committed in the specification phase and verify as-is.

Per-item findings:

1. **No specs weakened** — PASS. Source unchanged since the specification
   boundary (`cdbf5ab`), so proving weakened nothing. Contracts are strong and
   non-trivial:
   - `from_mmio_address`: `requires spec_frame_number(addr@) <= spec_max_frame_number()`;
     `ensures result is Ok && r@ == addr@ && r.inv()`.
   - `from_number`: `ensures result@ == spec_from_number(spec_frame_raw_value(frame))
     && result@ % spec_page_size() == 0 && result.inv()`.
   - `into_frame_number`: `requires self.inv()`;
     `ensures spec_frame_raw_value(result) == spec_frame_number(self@)`.
   Type invariant `inv()` = `spec_frame_number(self@) <= spec_max_frame_number()`
   is preserved by every constructor. No `ensures true` / `requires false` /
   trivialized post-states.

2. **Zero admit** — PASS. `grep -rnE "admit"` over `phys.rs`/`phys.spec.rs`/
   `phys.proof.rs` → none. Global `admit=0`.

3. **Zero external_body (outside TCB)** — PASS. No exec/function in the target
   is `external_body`. The single `#[verifier::external_body]` (phys.spec.rs:40)
   sits on `#[verifier::external_type_specification] pub struct ExFrameNumber(FrameNumber)`
   — an opaque registration of the foreign, non-Verus `arch::FrameNumber` type
   (unavoidable; the orphan rule forbids a local `View` impl). The cheating
   classifier scopes this as `external_type_spec`, exactly like the accepted
   `ExLinkedList` in `mm/phys/mod.spec.rs`. It is a type registration, not a
   function-body bypass, so the per-function TCB HARD RULE does not apply.

4. **Zero assume/assume_specification (external-bottom only)** — PASS.
   `assume()` calls: 0. `assume_specification`: 6, all at genuine
   external / not-yet-verified library edges:
   - `::arch::mem::FRAME_SIZE` (phys.spec.rs:80) — `arch` crate, not Verus-enabled.
   - `::arch::mem::FRAME_SHIFT` (89) — `arch`.
   - `FrameNumber::into_raw_value` (113) — `arch`.
   - `FrameNumber::from_raw_value` (120) — `arch`.
   - `VirtualAddress::new` (97) — `sys` crate. Confirmed `src/libs/sys/.../virt.rs`
     exposes only a `closed` `View` impl; `new` carries **no** `#[verus_spec]`,
     so the spec must be supplied at the boundary. The contract `result@ ==
     value as int` is the faithful (not weakened) semantics.
   - `<VirtualAddress as Address>::into_raw_value` (104) — `sys`; faithful
     identity `result as int == addr@`.
   These mirror the codebase's existing `arch`/`sys` boundaries and were
   approved in the specification phase (RESOLVED, 2 rounds).

5. **No cfg-gated exec code** — PASS. `#[cfg(verus_keep_ghost)]` only gates the
   ghost `include!`s, the `vstd` import, and the `View` impl (ghost). No
   cfg-gated exec branch, match arm, or expression inside any function body.

6. **Cheating audit (target module)** — exact counts + locations:
   - `admit`: 0.
   - `assume(...)`: 0.
   - `external_body` on a function: 0.
   - `external_type_specification` + `external_body`: 1 — `ExFrameNumber`
     @ `phys.spec.rs:39-41`.
   - `assume_specification`: 6 — `phys.spec.rs:80, 89, 97, 104, 113, 120`.
   - cfg-gated exec code: 0.
   - `phys.proof.rs`: empty (`verus! { }`); all proof obligations discharged
     inline in `proof!` blocks in `phys.rs` using div/mod/shift lemmas +
     `nonlinear_arith` — no admit/assume.

7. **Verus-limitation isolated reproducer** — PASS (N/A). All three exec
   functions are fully body-verified; none is `external_body` on a claimed
   Verus limitation, so no reproducer is owed. The foreign-type registration
   (`ExFrameNumber`) and the `arch`/`sys` `assume_specification`s are standard
   trust boundaries, not limitation workarounds.

8. **Exec rewrites minimal / semantically equivalent** — PASS. No
   `// VERUS REWRITE` markers in the target. Exec bodies are the original
   implementations with only ghost `proof!` blocks inserted (no change to
   runtime semantics).

9. **Cross-module regression** — PASS. Full `make verify` across all crates:
   every crate exit 0, 0 verification errors (`sys` CLEAN; remaining crates
   only flag pre-existing, out-of-scope, TCB-governed `external_body`).

10. **Verification + build, 0 errors / 0 warnings** — PASS.
    `make verify-kernel` = 37 verified, 0 errors, 0 warnings;
    `./z build -- all` = Build complete, no compiler warnings.

### Fix Request
None. Every checklist item is PASS with concrete evidence (commands, logs, and
git diffs cited above). No code changes required.
