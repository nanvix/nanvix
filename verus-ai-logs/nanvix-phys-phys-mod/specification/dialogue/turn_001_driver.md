## Turn 1: Full specification-phase checklist walkthrough (`mm::phys` / `mod.rs`)

### Progress
- Done (PASS / acceptable-with-evidence): ALL 17 checklist items below.
- Current: full sweep completed on turn 1.
- Remaining: none.

In-scope functions (per `verus-ai.toml` + `verification-plan.json`):
`init`, `book_physical_memory_regions`, `book_mmio_regions`. (`test` is out of scope.)

Commands run this turn (all from repo root `/home/ruize/nanvix-phy`):
- `make verify-kernel MODULE=mm::phys` → exit 0
- `make verify-kernel` (all modules, cross-module regression) → exit 0
- `make check-kernel` (build / cargo check of kernel crate) → `build-finished success:true`, exit 0
- `python /home/ruize/verus-ai-exp/verus-ai/scripts/fn_coverage.py src/kernel/src/mm/phys/mod.rs src/kernel/src/mm/phys/mod.rs`
- `python /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py check phys-mod`
- Read: `mod.rs`, `mod.spec.rs`, `mod.proof.rs`, `frame.rs`, `manager.rs`, `caller_analysis.md`, `view_design.md`, `bugs.md`, `tcb-allowed.md`, `cheating-detail.txt`.

---

### 1. Every in-scope exec function has requires/ensures — **PASS**
`fn_coverage.py`: Source exec fns 4 / Verus exec fns 4 / Matched 4 / Missing 0 / Extra 0
(`book_mmio_regions`, `book_physical_memory_regions`, `init`, `test`). The 3 in-scope
functions all carry `#[verus_spec(...)]` with both `requires` and `ensures`
(`mod.rs` L59-72, L87-102, L149-166). `test` is out of scope. **PASS.**

### 2. Caller coverage — **PASS**
`caller_analysis.md`: sole external caller is `kernel_vas::init` (kernel_vas.rs:120),
treating `phys::init` as a one-shot boot barrier. Caller expectations after `Ok(())`:
(a) subsystem live, (b) every physical-region frame reserved, (c) every *covered* MMIO
frame reserved, (d) `wf`/`inv` holds. `init`'s Ok ensures provide exactly:
`phys_view().live()`, `frames.all_reserved(phys_regions_frame_set(&physical_memory_regions))`,
`forall a: mmio_regions_frame_set(..).contains(a) && covers(a) ==> reserved(a)`, and
unconditional `phys_view().inv()`. The two private helpers' contracts feed `init`'s
post-state. Every caller expectation has a corresponding ensures. **PASS.**

### 3. View consistency — **PASS**
Specs reference `PhysModView`/`FrameAllocView` fields only (`phys_view().initialized`,
`.frames`, `.inv()`, `.live()`, `all_reserved`, `covers`, `reserved`) — no storage
mechanism named. `inv()` is preserved on **all** paths (stated unconditionally outside the
`match` in all three functions). Matches `view_design.md` §2-§4. **PASS.**
- Observation (non-blocking): the implemented Ok/Err arms are *simplifications* of
  `view_design.md` §4.2-§4.4 (e.g. helper Ok arms state `all_reserved(R)` rather than the
  full `v' == v.frames.book_all(R)` transition; `init` Ok states the headline facts rather
  than `seed(..).book_all(P).book_covered(M)`). These are **sound and caller-complete** (see
  item 11) and the dropped clauses were either derivable or not caller-observed; the
  composed-transition form is correctly deferred to the proof phase together with the
  stateful `phys_view()` token (`view_design.md` §8). No weakening of a caller-relied-upon
  guarantee. Acceptable.

### 4. No tautological ensures — **PASS (with reasoning)**
All three functions contain a `match result { Ok => <fact>, Err => true }`. Taken alone
`Err(_) => true` is the flagged pattern, **but the error path is not vacuous**: each
function additionally states `phys_view().inv()` (and the two helpers also
`phys_view().initialized`) **unconditionally**, outside the match. So on `Err` the contract
still guarantees `inv()` (+ `initialized` for helpers). The `Err => true` arm is the
idiomatic residual of a success-only postcondition and is the **sound** choice for these
`external_body` (trusted) helpers — strengthening a trusted `Err` arm (e.g. claiming
`!all_free(R)`) would assert an unverified fact about the real body. Not a vacuous spec.
**PASS.**

### 5. No subsumed ensures — **PASS**
- `init`: unconditional `inv()` is subsumed by `live()` *only on the Ok path*, but `inv()`
  is the sole guarantee on the Err path, so it is required globally — not subsumable.
- Helpers: `inv()` adds `frames.wf()` that `initialized` alone does not give; the Ok-arm
  `all_reserved(..)` is not derivable from `inv()`. None subsumed. **PASS.**

### 6. Error paths have meaningful ensures — **PASS**
Match style is used (`Ok => ..., Err => ...`). Error paths carry real guarantees via the
unconditional `inv()` / `initialized` clauses (item 4). `init`'s Err arm is intentionally
`true` beyond `inv()` because the caller aborts on `?` and observes no partial state
(`caller_analysis.md` L91-96, `view_design.md` §4.4 / §7.5 — explicitly justified). **PASS.**

### 7. No assume_specification for workspace-internal code — **PASS**
`make verify-kernel` cheating check: `assume=0`. Only external registration present is
`ExLinkedList` (`#[verifier::external_type_specification]` for `alloc::collections::LinkedList`,
a std type — `mod.spec.rs` L65-69). No `assume_specification` on any kernel-internal item. **PASS.**

### 8. vstd searched before any assume_specification — **PASS (N/A)**
No `assume_specification` used. `bugs.md` documents that `vstd` was searched for a
`LinkedList` model and none exists (orphan rule blocks providing one downstream), which is
why the two iterating helpers are `external_body` instead. **PASS.**

### 9. Specs written for the caller — **PASS**
`init`'s ensures are drop-in for the caller's proof: `live()` (single liveness fact downstream
`virt::init` needs) and `all_reserved(..)` / MMIO-covered⇒reserved (the "booked ⇒ never
allocated" safety property the caller writes directly). Helper specs are phrased in the same
`FrameAllocView` vocabulary `init` consumes. **PASS.**

### 10. Trait obligations — **PASS (none)**
In-scope functions are free functions implementing no trait; no `Drop`/`Iterator`/`GlobalAlloc`
contracts apply (`caller_analysis.md` "Trait Obligations: None"). **PASS.**

### 11. Spec completeness (advisory) — **PASS (advisory)**
No `spec-completeness` skill is installed in this repo; assessed manually. The only intentional
nondeterminism is `init`'s `Err => true` arm, which **matches the caller expectation**
(abort-on-`?`, no partial-state reliance). Ok arms fully pin the caller-observed post-state.
Acceptable per the rule (intentional nondeterminism matching caller expectations). **PASS.**

### 12. Loop invariants — **PASS**
`book_physical_memory_regions` has a `for region in ...iter()` loop; `book_mmio_regions` has a
`for` + an inner `while start < end` loop; `init` has no loops. The two looping functions are
`#[verus_verify(external_body)]`, so Verus does not verify their bodies and does not require
`invariant` clauses (and could not check them). No body-verified function in scope contains a
loop. `make verify-kernel` reports `no_decreases=0`. **PASS.**

### 13. No cheating on module's own functions — **PASS (reported + individually challenged)**
Global counts (`make verify-kernel`): `assume=0 external_body=15 admit=2 trusted=0 cfg_gate=5`.
In-scope (`mm::phys/mod.rs` + `mod.proof.rs`) items, each addressed individually:

- `mod.rs:73 book_physical_memory_regions` — `external_body`. **Allowed**: listed in
  `tcb-allowed.md` ("external_body introduced while speccing mm::phys") and root-caused in
  `bugs.md` (no `vstd` `LinkedList` model; orphan rule forbids supplying one). Carries
  meaningful `requires`/`ensures`. OK.
- `mod.rs:103 book_mmio_regions` — `external_body`. **Allowed**: same `tcb-allowed.md` entry
  and `bugs.md` rationale (LinkedList iteration). Carries meaningful `requires`/`ensures`. OK.
- `mod.proof.rs:9 lemma_frame_initialized` — `admit()`. Asserts `phys_view().initialized &&
  frames.wf()` after `frame::init`. I verified `frame::init` (frame.rs:657) is `external_body`
  with **no `verus_spec`** (no ensures), and it is out of this module's scope (`tcb-allowed.md`
  skip-list + "do not touch unlisted functions"). Therefore this fact cannot be derived in the
  spec phase and cannot be supplied by adding a contract to the callee here. It is the
  documented spec-phase stand-in for the external dependency's postcondition, to be discharged
  by the ghost-token machinery in the proof phase (`view_design.md` §8, and the lemma's own
  doc-comment). **Acceptable-deferred** (external-dependency contract, temporarily allowed).
- `mod.proof.rs:22 lemma_manager_ready` — `admit()`. Same situation: `PhysMemoryManager::init`
  (manager.rs:86) is `external_body` with no ensures and out of scope. **Acceptable-deferred.**

The remaining 13 `external_body` (frame.rs ×9, manager.rs ×1, upool.rs struct+`new`,
`ExLinkedList`) are cross-module / external dependencies in other files, all enumerated in
`tcb-allowed.md`, out of this module's scope. **PASS** (every in-scope cheating site
individually accounted for; none removable within this phase/scope).

### 14. No specs weakened — **PASS**
`spec_drift.py check phys-mod` (baseline `f5a2b87` = specification phase-start SHA):
"Functions with changes: 0, Contract drift: 0, Ensures removed: 0, Requires added: 0 — ✅ No
contract drift detected." No original guarantee weakened within the phase. **PASS.**

### 15. Bug awareness — **PASS**
`bugs.md` exists and is current: no code bugs in the three targets (logically correct, no
overflow/off-by-one/impossible path); documents the LinkedList verifier limitation as a
*tooling* limitation, not a code bug, with a resolution path. **PASS.**

### 16. Cross-module regression — **PASS**
`make verify-kernel` (all modules): exit 0. Re-verified `mm::phys`, `mm::phys::frame`,
`mm::phys::manager`, `mm::phys::upool`; all other kernel modules cached-passing. No module
regressed. **PASS.**

### 17. Verification + build — **PASS**
- `make verify-kernel MODULE=mm::phys` → exit 0, 0 errors.
- `make verify-kernel` (full) → exit 0, 0 errors.
- `make check-kernel` (build) → `{"reason":"build-finished","success":true}`, exit 0.
**PASS.**

---

### Verdict
All 17 checklist items are **PASS** or **acceptable-with-evidence**. The two `admit()`
lemmas and the two helper `external_body` markers are the only in-scope "cheating" sites; each
is individually justified by an external/out-of-scope dependency with no available contract
(documented in `tcb-allowed.md`, `bugs.md`, `view_design.md` §8) and is correctly deferred to
the proof phase. Verification (module + full kernel) and the kernel build both pass with 0
errors; no spec drift; coverage complete; caller expectations fully covered.

### Fix Request
None. No checklist item failed in a way that requires (or could soundly accept) a code change
in this phase. Proceeding to STOP = RESOLVED.
