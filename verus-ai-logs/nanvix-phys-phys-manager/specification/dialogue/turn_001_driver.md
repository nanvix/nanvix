# Turn 1: Full checklist walkthrough — phys-manager specification review

Reviewer: STRICT. Evidence below is from reading the three source files, the
view-design / caller-analysis / tcb-allowed docs, `make verify-kernel` output,
and direct greps of the vstd source tree.

## Progress
- **PASS (verified):** fn coverage; view consistency; no assume_specification for
  workspace-internal code; vstd searched; trait obligations; loop invariants;
  cross-module regression; verification pass; cheating counts reported.
- **FAIL (fix requested this turn):**
  1. No tautological ensures — `init` Err arm is `Err(_) => true`.
  2. Error paths have meaningful ensures — same `init` Err arm.
  3. No specs weakened — **`alloc_many_kernel_frames` dropped contiguity**;
     dropped `frames@.len()==0` on Err; `alloc_many_user_frames` dropped
     distinctness; `alloc_kernel_frame` dropped the `free_count()==0` liveness
     fact on Err; `init` dropped designed Err/Ok facts.
  4. Spec completeness (advisory) — same gaps as (3).
  5. No cheating on module's own functions — 8 admitted lemmas (reported; see
     note — boundary-acceptable for spec phase but two lemma *contracts* are
     themselves too weak and enable the weakened specs in (3)).
- **Remaining (re-verify after fixes):** subsumed-ensures and bug-awareness are
  PASS today but must be rechecked once specs are strengthened.

---

## Verification evidence (commands run)

- `make verify-kernel` → Exit 0; modules `mm::phys`, `frame`, `kframe`,
  `manager`, `upool` all verified (cached). Cheating check (whole kernel crate):
  `assume=0 external_body=24 admit=10 trusted=0`. Coverage 26/1023 (whole crate).
- Manager-scoped greps:
  - `external_body` in manager.rs: **2** — `init` (L96) and `kernel_watermark`
    (L505). Both on `verus-ai-logs/tcb-allowed.md` (cross-module dep /
    external-bottom constant). OK for spec phase.
  - `assume_specification` in manager.spec.rs: **3** — `Result::and_then`,
    `Result::inspect_err`, `Vec::capacity`.
  - `admit()` in manager.proof.rs: **8** (every lemma).
- vstd search (`/mnt/toolchain/verus/vstd/std_specs/`): `result.rs` has
  `is_ok/is_err/as_ref/unwrap/unwrap_err/expect/map/map_err/ok/err` but **no
  `and_then`, no `inspect_err`**; `vec.rs` has `with_capacity` but **no
  `capacity`**; `option.rs` has `Option::and_then` only. → the 3
  assume_specifications are justified (target fns genuinely unspecified in vstd).

---

## Checklist results

### 1. Every in-scope exec function has requires/ensures — **PASS**
All 6 target fns carry `#[verus_spec]`: `init` (L97), `alloc_many_user_frames`
(L173), `alloc_user_frame` (L267), `check_user_watermark` (L307),
`alloc_kernel_frame` (L348), `alloc_many_kernel_frames` (L402); plus
`kernel_watermark` (L506). `get_mut` is uncontracted but explicitly excluded
(`tcb-allowed.md` "Skip / exclude"; caller-analysis "Supporting, not a target").
(Note: `fn_coverage.py` not present in repo; verified via `make verify-kernel`
coverage output + manual inspection of all 7 exec fns.)

### 2. Caller coverage — **FAIL** (rolls up into item 3)
Mapping each caller expectation (`caller_analysis.md`) to the spec:
- `alloc_user_frame`: Ok `user_alloc_ok(1)` + `alloc_one`; Err `!user_alloc_ok(1)`.
  Matches caller's "OutOfMemory iff watermark blocks." **covered.**
- `check_user_watermark`: Ok/Err free_count vs `count+watermark`. **covered.**
- `alloc_kernel_frame`: caller relies on "kernel allocation must succeed whenever
  a physical frame is available" → needs `Err ⇒ free_count()==0`. **MISSING.**
- `alloc_many_kernel_frames`: caller relies on **physical contiguity** and
  "vector left empty on Err, nothing leaks." Both **MISSING.**
- `alloc_many_user_frames`: caller relies on **`count` distinct** frames. The
  `S.len()==count` distinctness is **MISSING.**
- `init`: caller relies on "Err only InvalidArgument / nothing else changed."
  Err arm is `true`. **MISSING.**

### 3. View consistency — **PASS**
`type V = FrameAllocView`; `inv()==self@.wf()`; every spec speaks the partition
vocabulary (`free_frames`, `all_free`, `book_all`, `alloc_one`,
`user_alloc_ok`); all `&mut self` ensures restate `final(self).inv()`. (The
contiguity gap is a *completeness* issue under item 8, not a View-shape issue.)

### 4. No tautological ensures — **FAIL**
`manager.rs:99-102` `init`: `Err(_) => true` — the exact canonical tautology.
This is **expressible and soundly fixable** (see Fix Request A): `init` returns
`Err` only when `PHYS_MEMORY_MANAGER_INIT` is already set, i.e. a prior init
succeeded, so `phys_view().manager_ready` already holds on the Err arm.
(For contrast, every `&mut self` Err arm here is meaningful — `final(self)@ ==
old(self)@` etc. — so this is the only offender.)

### 5. No subsumed ensures — **PASS**
No clause is derivable from `inv()` + the others. `final(self).inv()` is the wf
*preservation* fact (not implied by the transition clauses alone). The watermark
clauses are independent of `wf()`.

### 6. Error paths have meaningful ensures — **FAIL**
`init` Err is meaningless (`true`). All other Err arms are meaningful. Fixed by A.

### 7. No assume_specification for workspace-internal code — **PASS**
All 3 are `core`/`alloc` types (`Result`, `Vec`), not workspace crates.

### 8. vstd searched before any assume_specification — **PASS**
Confirmed by direct grep of `vstd/std_specs/{result,vec,option}.rs` (above).

### 9. Specs written for the caller — **PARTIAL/FAIL**
The covered fns are caller-usable. But the dropped guarantees in item 3 mean
the kernel-bulk and user-bulk specs are **not** directly usable by their callers
(`alloc_kpages`, `alloc_upages`) for the properties those callers actually need
(contiguity, distinctness, no-leak). Fixed alongside item 3.

### 10. Trait obligations satisfied — **PASS**
`caller_analysis.md` §Trait Obligations: none. `PhysMemoryManager` implements no
dispatch-relevant trait. Returned `KernelFrame`/`UserFrame` Drop semantics are
owned by their own modules.

### 11. Spec completeness (advisory) — **FAIL (advisory)**
Same gaps as item 3: the central kernel-bulk property (contiguity) and the
kernel-single liveness fact are absent; user-bulk distinctness absent. These are
not "intentional nondeterminism matching caller expectations" — the callers
*do* depend on them per `caller_analysis.md`.

### 12. Loop invariants — **PASS**
Both `for` loops carry `invariant` clauses: `alloc_many_user_frames` (L230-235);
`alloc_many_kernel_frames` outer (L446-453) and inner cleanup (L465-473).

### 13. No cheating on module's own functions — **REPORTED / CONCERN**
Counts: `external_body=2` (both TCB-allowed), `assume_specification=3` (external,
allowed), `admit=8`, `trusted=0`, `assume=0`.
The 8 admits are all in `manager.proof.rs` lemmas, explicitly "deferred to the
proving phase." Per-lemma:
- `lemma_manager_attached` (L12) — global-token attachment, deferral expected.
- `lemma_free_count_bounded` (L21) — finite-frame bound, deferral expected.
- `lemma_kernel_alloc_one` (L36) — Ok-arm of `alloc_kernel_frame`, contract OK.
- `lemma_kernel_alloc_contiguous` (L49) — **contract too weak**: ensures only
  `book_all(kernel_addr_set(..))`, no contiguity. This is *why* the exec spec
  lost contiguity. Must be strengthened (Fix Request B), not just discharged.
- `lemma_contig_no_overflow` (L70) — arithmetic bound, OK.
- `lemma_user_bulk_ok` (L87) — **contract too weak**: no
  `user_addr_set(frames).len()==count` distinctness. Strengthen (Fix Request C).
- `lemma_user_bulk_err_restored` (L107) / `lemma_kernel_bulk_err_restored`
  (L117) — restoration facts, contracts OK (note the kernel one is currently
  unused since the Err arm dropped the vec-emptied claim — see Fix Request B).
Admitted *bodies* are an acceptable spec→proof hand-off; admitted lemmas with
*weak contracts that launder weakened exec specs* are not. B and C address the
latter.

### 14. No specs weakened — **FAIL (primary item)**
Comparing implemented specs against `view_design.md` §4 (the committed reference)
and `caller_analysis.md`:

| Fn | Designed guarantee | Implemented | Verdict |
|----|--------------------|-------------|---------|
| `alloc_many_kernel_frames` Ok | contiguous: `∃ base. base%ps==0 ∧ ∀i. frames[i]@==base+i*ps` (§4.5) | only `book_all(kernel_addr_set(..))` — a **set**, no order/contiguity | **WEAKENED** |
| `alloc_many_kernel_frames` Err | `v'==v ∧ frames@.len()==0` (§4.5) | only `final@==old@` | **WEAKENED** (no-leak/empty dropped) |
| `alloc_many_user_frames` Ok | `S.len()==count` distinct (§4.4) | distinctness absent → duplicate addresses satisfy the spec | **WEAKENED** |
| `alloc_kernel_frame` Err | `v.free_count()==0` (§4.1 liveness) | only `final@==old@` | **WEAKENED** |
| `init` Ok/Err | `manager_ready` + frames-untouched; Err = InvalidArgument/unchanged (§4.6) | `Ok ⇒ manager_ready`, `Err ⇒ true` | **WEAKENED + tautology** |

Justified *non*-weakenings (acceptable, no action): `alloc_many_user_frames`
dropped `Err ⇒ !user_alloc_ok` (correct: the fn can also fail on empty-vec /
capacity input checks, so Err does not imply watermark breach); `Ok` clause
gated `count>0 ⇒ user_alloc_ok` (correct: a zero-count alloc needs no
watermark). `count>0` precondition added per `bugs.md` OBS-1 (a documented
strengthening). The frames-untouched/`old(phys_view())` parts of init are not
expressible single-state, so only the realistically-expressible facts are
required below.

### 15. Bug awareness — **PASS (augment requested)**
`bugs.md` records OBS-1 (`alloc_many_kernel_frames` count==0). Add **OBS-2**:
the user-bulk spec currently admits *duplicate* physical frames among returned
handles (no distinctness), which would be a double-free hazard — record it as a
spec hole being closed by Fix Request C.

### 16. Cross-module regression — **PASS**
`make verify-kernel` verifies the whole kernel crate; all phys modules exit 0.

### 17. Verification — **PASS**
`make verify-kernel` Exit 0 (cargo build "Finished ... Exit code 0"). Status line
reads `CHEATING_DETECTED` only because of the whole-crate `external_body`/`admit`
counts, which for the manager module are the TCB-allowed + deferred-lemma set
above; no `assume`/`trusted`, verification itself green.

---

## Fix Requests (address all; re-run `make verify-kernel` after each)

### Fix A — `init`: remove the tautological Err arm (items 4, 6, 14)
In `src/kernel/src/mm/phys/manager.rs` (L97-103) change the Err arm from
`Err(_) => true` to a sound, meaningful single-state fact:
```rust
#[verus_spec(result =>
    ensures
        match result {
            Ok(_)  => crate::mm::phys::phys_view().manager_ready,
            Err(_) => crate::mm::phys::phys_view().manager_ready,
        },
)]
```
Rationale (sound because `init` is `external_body`/TCB): `init` returns `Err`
only when `PHYS_MEMORY_MANAGER_INIT` is already set, which only happens after a
prior successful `init`, so the manager is already ready. This removes the
`Err(_) => true` tautology and matches `caller_analysis.md` ("Err only on
double-init"). Verify: `make verify-kernel` stays Exit 0 and the
`Err(_) => true` grep in manager.rs returns nothing.

### Fix B — `alloc_many_kernel_frames`: restore contiguity + no-leak (items 3, 9, 11, 14)
1. In `manager.rs` (L408-416) replace the Ok/Err arms with the §4.5 contract:
```rust
Ok(()) => {
    &&& final(frames)@.len() == count
    &&& exists|base: int| {
            &&& base % spec_page_size() == 0
            &&& forall|i: int| 0 <= i < count
                    ==> #[trigger] final(frames)@[i]@ == base + i * spec_page_size()
            &&& old(self)@.all_free(kernel_addr_set(final(frames)@))
            &&& final(self)@ == old(self)@.book_all(kernel_addr_set(final(frames)@))
        }
},
Err(_) => {
    &&& final(self)@ == old(self)@
    &&& final(frames)@.len() == 0
},
```
   (Contiguity may use `region_frame_addrs(base, count*ps)` as in §4.5 if you
   prefer the existing helper; the `base + i*ps` form is equivalent and is what
   the caller `alloc_kpages` needs.)
2. In `manager.proof.rs` strengthen `lemma_kernel_alloc_contiguous` (L49-64) so
   its `ensures` provides the `∃ base. base%ps==0 ∧ ∀i. frames[i]@==base+i*ps`
   contiguity fact (keep `admit()` body — discharge is the proving phase's job).
3. The Err arm now needs the vec-emptied fact: wire
   `lemma_kernel_bulk_err_restored` (currently unused) into the cleanup path, or
   establish `frames@.len()==0` from the `frames.clear()` at L463.
Verify: `make verify-kernel` Exit 0; grep that `frames@[i]` contiguity appears in
the `alloc_many_kernel_frames` ensures.

### Fix C — `alloc_many_user_frames`: restore distinctness (items 3, 11, 14, 15)
1. In `manager.rs` (L180-185) add to the Ok arm:
```rust
&&& user_addr_set(final(frames)@).len() == count
```
2. In `manager.proof.rs` strengthen `lemma_user_bulk_ok` (L87-102) `ensures`
   with `user_addr_set(frames).len() == count` (keep `admit()` body).
3. Record OBS-2 in `verus-ai-logs/nanvix-phys-phys-manager/bugs.md`.
Verify: `make verify-kernel` Exit 0; `user_addr_set(...).len() == count` present
in the Ok arm.

### Fix D — `alloc_kernel_frame`: restore the liveness Err fact (items 2-rollup, 3, 14)
In `manager.rs` (L358) change the Err arm to:
```rust
Err(_) => {
    &&& final(self)@ == old(self)@
    &&& old(self)@.free_count() == 0
},
```
Support it with a new admitted lemma in `manager.proof.rs`, e.g.
`lemma_kernel_alloc_err_empty(pre: FrameAllocView) requires pre.wf() ensures
pre.free_count() == 0` invoked on the `frame::alloc()?` error path — mirroring how
the Ok arm uses `lemma_kernel_alloc_one`. (Discharging it in the proving phase
will require `frame::alloc`'s Err spec to expose `free_count()==0`; that is the
cross-module obligation, recorded now.) If you determine this genuinely cannot
hold (e.g. `frame::alloc` can fail with frames free), do **not** silently drop
it — instead record the contradiction in `bugs.md` with the `frame::alloc` spec
evidence. Justification text alone is not a fix.
Verify: `make verify-kernel` Exit 0; `free_count() == 0` present in the
`alloc_kernel_frame` Err arm.

---

## Not yet resolvable here — none. Do NOT create STOP.
Items 4, 6, 9, 11, 13(contracts), 14, 15 remain FAIL/CONCERN until Fixes A-D land
and I re-verify the diff and `make verify-kernel` myself next turn.
