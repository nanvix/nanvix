## Turn 1: Full checklist walkthrough + first fix request (in-scope exec functions have no meaningful specs)

### Progress
- Done (verified PASS): #5 (vacuous), #7, #8, #10, #14, #15, #16
- Current (FAIL, fix requested this turn): #1, #2, #3, #4, #6, #9, #13 — the in-scope
  exec functions carry no usable contract, and the module's own proof lemmas all
  `admit()`.
- Remaining (FAIL, will be re-checked after the cluster above is fixed): #11 (advisory),
  #12, #17 (cheating gate).

---

### Evidence gathered

Commands run:
- `make verify-kernel` → exit 0, but `status: CHEATING_DETECTED`
  (`assume=0 external_body=17 admit=7 trusted=0`, coverage `9/1022`).
- `fn_coverage.py src/kernel/src/mm/phys/mod.rs …` → 4 source exec fns, 4 matched.
- `spec_drift.py check nanvix-phys-phys-mod` → 0 contract drift.
- `grep -n "verus_spec|ensures|requires" mod.rs` → exactly **one** `#[verus_spec]`
  block, on `init`, body `ensures true`.
- `cheating-detail.txt` → all 7 admits and both in-scope `external_body` are in this
  module's own files.

In-scope exec functions (caller_analysis): `init`, `book_physical_memory_regions`,
`book_mmio_regions`.

Current exec specs:
- `init` (mod.rs:134-147): `#[verus_spec(ret => ensures true)]`.
- `book_physical_memory_regions` (mod.rs:67-80): `#[verus_verify(external_body)]`, **no**
  `#[verus_spec]`, no requires/ensures.
- `book_mmio_regions` (mod.rs:87-117): `#[verus_verify(external_body)]`, **no**
  `#[verus_spec]`, no requires/ensures.

Module's own proof lemmas (mod.proof.rs) — **all 7 bodies are `admit()`**:
- :19 `lemma_spec_initialize_establishes_inv`
- :36 `lemma_spec_book_frame_preserves_inv`
- :57 `lemma_spec_book_frames_preserves_inv`
- :79 `lemma_book_region_reserves_region_frames`
- :105 `lemma_book_mmio_skip_untracked`
- :120 `lemma_book_mmio_books_tracked`
- :140 `lemma_init_establishes_and_reserves`

---

### Per-item verdicts

| # | Item | Verdict | Evidence |
|---|------|---------|----------|
| 1 | Every in-scope exec fn has requires/ensures | **FAIL** | `book_physical_memory_regions` and `book_mmio_regions` have no `#[verus_spec]`; `init` has only `ensures true`. |
| 2 | Caller coverage | **FAIL** | caller_analysis Ok-expectations (allocator established ⇒ `inv()`/`wf()` holds; reserved RAM frames booked; tracked MMIO booked; untracked MMIO skipped; one-shot) are encoded in **zero** exec ensures. |
| 3 | View consistency | **FAIL** | No exec ensures references any `PhysMemView` field or `inv()`. The View + transitions exist only in mod.spec.rs / mod.proof.rs, fully disconnected from the exec functions. |
| 4 | No tautological ensures | **FAIL** | `init` ensures `true` — the canonical tautology. |
| 5 | No subsumed ensures | PASS (vacuous) | No real exec ensures exist to be subsumed. |
| 6 | Error paths have meaningful ensures | **FAIL** | No `Ok =>`/`Err =>` split anywhere; `init` is `true`, `book_*` have nothing. |
| 7 | No assume_specification for workspace-internal code | PASS | `assume=0`. Only `external_type_specification` for foreign std `LinkedList` (external dep), allowed. |
| 8 | vstd searched before assume_specification | PASS | No assume_specification; LinkedList gap documented in verus-unsupported.md. |
| 9 | Specs written for the caller | **FAIL** | `ensures true` is unusable in `kernel_vas::init`'s proof; the proof lemmas are free-standing and never connected to `init`'s postcondition. |
| 10 | Trait obligations satisfied | PASS | No traits (caller_analysis confirms free functions, no Drop/Iterator/fn-ptr). |
| 11 | Spec completeness (advisory) | **FAIL** | Contract is empty relative to the documented abstract effect ("establish `inv`, pre-reserve boot frames"). |
| 12 | Loop invariants | **FAIL** | Both `book_*` contain loops (`for region in …iter()`, and a `while start < end` in `book_mmio_regions`), none verified — escaped via `external_body`. |
| 13 | No cheating on module's own functions | **FAIL** | 7× `admit` in mod.proof.rs (this phase's own lemmas) + 2× `external_body` on this module's own exec fns. |
| 14 | No specs weakened | PASS | spec_drift: 0 contract drift. |
| 15 | Bug awareness | PASS | bugs.md present; no code bug; tooling limitation recorded. |
| 16 | Cross-module regression | PASS | verify-kernel verified mm::phys, frame, manager, upool — exit 0. |
| 17 | Verification | **FAIL (gate)** | verus exit 0, but `status: CHEATING_DETECTED`. The exit-0 only holds because all 7 lemmas `admit()`. The cheating gate is the blocker, tied to #13. |

---

### Root problem

The specification phase produced a View (`PhysMemView`), spec transitions
(`spec_initialize`, `spec_book_frame`, `spec_book_frames`), and 7 proof-lemma
*signatures* — but **none of this is wired into the exec functions**. `init` promises
`true`, and the lemmas that would carry the real content all `admit()`. The exec layer
therefore offers the caller nothing, and the proof layer cheats on every one of the
module's own lemmas.

The mod.rs / view_design.md justification — "binding that effect to the global singleton
state is deferred to a later phase" because the functions take no `self`/ghost handle —
is a justification, not a fix. The frame layer already proves that the global state is
nameable: `frame.rs` `Inner` methods carry full `self@ : FrameAllocView` ensures
(book :116, alloc_range :182, is_covered :506). The missing piece is purely a spec-phase
task: expose the global (`instance()@` / `INSTANCE_INIT`) as a `PhysMemView` and give the
`frame::*` free-function wrappers thin pass-through specs, then state `init`'s real
postcondition over it. This does **not** require changing the View struct fields, so it
is fixable in this phase (no rollback yet).

---

### Fix Request (this turn)

Address the following. Do **not** respond with justifications — change the code and show
`make verify-kernel` output.

**A. Give `init` a real, caller-usable postcondition (fixes #1,#2,#3,#4,#6,#9).**
Replace `ensures true` (mod.rs:134-147) with an Ok/Err contract stated over the module
View. Concretely:
1. Add a `spec fn` that names the global post-state as a `PhysMemView` (e.g.
   `spec fn phys_view() -> PhysMemView` reading `frame::instance()@` and `INSTANCE_INIT`),
   placed in mod.spec.rs.
2. Give the `frame::*` free-function wrappers (`frame::init`, `frame::book`,
   `frame::is_covered`, `frame::alloc_range` — currently spec-less `external_body` at
   frame.rs:657/744/750/756) thin pass-through `#[verus_spec]` that forward the already-proven
   `Inner` post-states to the global view. (The View design "Notes for Later Phases" names
   this exact step; do it here.)
3. State `init`'s ensures as:
   - `Ok(())  ==> phys_view().initialized && phys_view().inv()` and the reserved RAM +
     tracked-MMIO frames are in `phys_view().frames.allocated_frames` and disjoint from
     `free_frames` (use `spec_initialize` / `spec_book_frames` and `region_frames`,
     matching `lemma_init_establishes_and_reserves`).
   - `Err(_) ==> ` a non-trivial statement (e.g. lifecycle/`!initialized` per view_design,
     not `true`).

**B. Give the two `book_*` helpers real ensures (fixes #1,#2,#6 for them).**
Even while they remain `external_body` for the LinkedList orphan-rule reason, add
`#[verus_spec]` requires/ensures (external_body bodies are unverified, but the *contract*
must exist and be honored by callers):
- `book_physical_memory_regions`: `Ok ==> new == old.spec_book_frames(R)` where `R` is the
  union of `region_frames` (per view_design); `Err ==> new == old`.
- `book_mmio_regions`: `Ok ==> new == old.spec_book_frames(M.intersect(old.covered()))`
  (encodes skip-if-not-covered); `Err ==> new == old`.

**C. Discharge the 7 `admit()` lemmas in mod.proof.rs (fixes #13, unblocks #17).**
Every one must be individually replaced with a real proof body (or removed if subsumed
once the exec specs above exist). They are not deferrable — they are this module's own
proof functions and the spec-design contract that the exec ensures depend on:
- mod.proof.rs:19 `lemma_spec_initialize_establishes_inv`
- mod.proof.rs:36 `lemma_spec_book_frame_preserves_inv`
- mod.proof.rs:57 `lemma_spec_book_frames_preserves_inv`
- mod.proof.rs:79 `lemma_book_region_reserves_region_frames`
- mod.proof.rs:105 `lemma_book_mmio_skip_untracked`
- mod.proof.rs:120 `lemma_book_mmio_books_tracked`
- mod.proof.rs:140 `lemma_init_establishes_and_reserves`

**Verify after fixing:**
- `make verify-kernel` → must reach exit 0 **and** `cheating: … admit=0` (the in-scope
  `external_body` on `book_*` may remain only if the LinkedList limitation is still
  documented and the functions now carry full `#[verus_spec]` contracts).
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py check nanvix-phys-phys-mod`
  → must stay at 0 contract drift (strengthen only; never weaken).
- `grep -n "verus_spec" src/kernel/src/mm/phys/mod.rs` → must show contracts on all three
  in-scope functions, with `init` no longer `ensures true`.

### STOP status
Not creating STOP — items #1,#2,#3,#4,#6,#9,#11,#12,#13,#17 are unresolved.
