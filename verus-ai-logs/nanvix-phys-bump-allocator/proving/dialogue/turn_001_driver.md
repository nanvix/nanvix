## Turn 1: Full checklist walkthrough — bump-allocator proving

### Progress
- Done (PASS): all 10 checklist items below.
- Current: final verification + build confirmation.
- Remaining: none.

### Verification

Baseline (proving phase start, from `pipeline_state.json`):
`9b2438e44a56046fd3daf7a563d119f31a67c0f2`.

Files in scope:
- `src/libs/bump_allocator/src/lib.rs` (exec)
- `src/libs/bump_allocator/src/lib.spec.rs` (specs)
- `src/libs/bump_allocator/src/lib.proof.rs` (proofs)

---

#### 1. No specs weakened — PASS
Ran the spec-drift checker:
```
python3 scripts/spec_drift.py git-diff src/libs/bump_allocator/src/lib.rs \
    --before 9b2438e44a56046fd3daf7a563d119f31a67c0f2
```
Exit code 1, but every flagged item is a **proof body** change, not a spec
clause change:
- `lemma_geometry`        — body `admit()` → real proof.
- `lemma_exhausted_boundary` — body `admit()` → real proof (now trivial).
- `lemma_alloc_transition` — body `admit()` → `assert(... =~= ...)`.
- 1 function *added*: `lemma_aligned_sum` (new arithmetic helper).

`git diff --stat 9b2438e4 -- src/libs/bump_allocator/` shows **only**
`lib.proof.rs` changed (74 insertions, 7 deletions). `lib.spec.rs` and `lib.rs`
are **byte-identical** to the baseline (empty diff). I confirmed the
`requires`/`ensures` of all three lemmas are unchanged vs the baseline
(`git show 9b2438e4:.../lib.proof.rs`). Discharging `admit()` placeholders with
real proofs is the intended proving-phase work and *strengthens* (does not
weaken) the artifact. No `ensures` removed, no `requires` added/strengthened.

#### 2. Zero remaining admit() — PASS
`make verify-bump-allocator` cheating line: `admit=0`. Source grep for `admit`
in `src/libs/bump_allocator/src/` returns nothing.

#### 3. Zero external_body unless in tcb-allowed — PASS
`external_body=2`, exact locations (cheating-detail.txt):
- `lib.rs:286 alloc: external_body`
- `lib.rs:367 alloc_as: external_body`

Both are explicitly registered in `verus-ai-logs/tcb-allowed.md`:
- `FixedSizeBumpAllocator::alloc` — materializes `&'static mut [u8; N]` from a
  backend address (`usize as *mut`); raw-memory op needing a `PointsTo` for
  externally-owned `BssStorage`. `ensures` states alignment + in-bounds over
  `bump_view`.
- `FixedSizeBumpAllocator::alloc_as` — delegates to `alloc`, re-materializes the
  slot as `&'static mut MaybeUninit<T>`; `ensures` adds size/align guard arms.

Both carry non-trivial `#[verus_spec]` contracts (not contract-free). No
unregistered `external_body` exists.

#### 4. Zero assume/assume_specification (std/external trust boundaries only) — PASS
Cheating line: `assume=0`. The only `assume_specification` is
`lib.spec.rs:28 [ <usize>::div_ceil ]` — a specification for the **std/core**
`usize::div_ceil` method (panics on zero divisor; result `(x+y-1)/y`). This is a
permitted external-bottom trust boundary for the standard library and is not
counted by the cheating scanner. No crate-local `assume`/`assume_specification`.

#### 5. No cfg-gated exec code — PASS
Cheating line: `cfg_gate=0`. The only `cfg` uses are:
- `#![cfg_attr(not(any(test, feature = "std")), no_std)]` (crate attribute),
- `#[cfg(verus_keep_ghost)] include!("lib.spec.rs"/"lib.proof.rs")` (ghost-only
  spec/proof inclusion),
- `#[cfg(test)] mod tests`.
None gate an exec branch, expression, or match arm.

#### 6. Cheating audit — PASS
`make verify-bump-allocator` summary:
`assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
- admit: 0
- external_body: 2 → `lib.rs:286 alloc`, `lib.rs:367 alloc_as` (both TCB-allowed)
- assume: 0 (one std `assume_specification` for `usize::div_ceil`, allowed)
- cfg-gated exec: 0

#### 7. Claimed Verus limitation has isolated reproducer — PASS
`verus-ai-logs/nanvix-phys-bump-allocator/verus-unsupported.md` documents each
limitation with the exact Verus error and a minimal trigger:
- §1 `break <expr>;` value-yielding loop break — "complex break expressions".
- §2 `usize`→`*mut [u8; N]` cast / raw-pointer deref — "Verus does not support
  this cast" / "dereferencing a raw pointer ... only ... permissioned raw_ptr".
  The required `PointsTo`/`PointsToRaw` permission has no source for the
  externally-owned `BssStorage` region → this is exactly the approved
  external-bottom boundary for `alloc`/`alloc_as`.
- §3 `AtomicUsize` value not spec-readable (`load` is exec; vstd: "NO support for
  reasoning about the values inside the atomics").
These are concrete, reproducible by removing `external_body`, and isolate the
offending construct.

#### 8. Exec rewrites minimal & semantically equivalent — PASS (trivially)
`lib.rs` (the only exec file) is byte-identical to the proving baseline — there
are **no** exec rewrites in this phase, hence no `// VERUS REWRITE` comments are
required and no semantic-equivalence concern arises.

#### 9. Cross-module regression — PASS
`make verify` ran to completion (process exit 0). Per-crate Verus exit codes all
0: `bitmap`, `sys`, `nanvix-slab`, `bump-allocator`, `arch`, `kernel`. The
`CHEATING_DETECTED` status reported for `arch` (admit=1, cfg_gate=4) and
`kernel` (admit=36, external_body=12, cfg_gate=15) is pre-existing
work-in-progress in *other* modules — bump-allocator's change set touches only
`lib.proof.rs`, so it introduces no cross-module regression. No verified module
fails.

#### 10. Verification + build — PASS
- `make verify-bump-allocator`: Exit code 0; `0 errors`; cheating
  `external_body=2` (both TCB-allowed), everything else 0.
- Build (kernel target):
  `cargo build -p bump-allocator --locked -Z build-std=... --target
  build/targets/x86-kernel.json` → `Finished` with **0 errors, 0 warnings**.

### Fix Request
None. All checklist items PASS with tool-backed evidence. The proving phase for
`bump-allocator` is complete: `admit()` placeholders discharged, specs unchanged
(no weakening), only TCB-registered `external_body` boundaries remain, and both
single-module and cross-module verification plus the build are clean.
