## Turn 1: Specification Review — Full Checklist Walk-through

Source: `src/kernel/src/mm/phys/frame.rs`
Spec:   `src/kernel/src/mm/phys/frame.spec.rs`
Proof:  `src/kernel/src/mm/phys/frame.proof.rs`
Verus:  `make verify-kernel`

### Progress
- Done (PASS): #1 fn-coverage, #3 view-consistency, #5 subsumed (advisory note),
  #8 vstd-first, #10 trait-obligations, #14 no-specs-weakened, #16 cross-module,
  #17 verification+build.
- Reported (informational): #13 cheating counts.
- Current FAIL (lead): #4 / #6 — tautological / non-meaningful error-path ensures
  on caller-facing free functions (`alloc`, `book`, `alloc_range`, `alloc_contiguous`).
- Remaining FAIL to address in later turns: #2 caller-coverage (tied to #4/#6),
  #7 assume_specification on workspace-internal `crate::hal::*`, #9 specs-for-caller
  (tied to #4/#6), #12 loop invariants (`invariant false` stubs), #15 bug-awareness
  (alloc_range off-by-one — recorded in bugs.md, to confirm).

---

### Verification (commands run)

- `make verify-kernel` → exit 0. `47 verified, 0 errors`. Status line:
  `CHEATING_DETECTED` — `assume=0 external_body=14 admit=24 trusted=0 cfg_gate=17`.
- `fn_coverage.py frame.rs frame.rs` → 11/11 top-level exec fns matched, 0 missing,
  0 extra.
- `spec_drift.py git-diff frame.rs --before 97e31408 (view-design boundary) --after HEAD`
  → 5 functions changed; all changes are **strengthening** (ensures added on
  `alloc`/`book`/`alloc_range`/`is_covered`) or **requires added that match documented
  caller expectations** (`region.inv()`, `phys_addr.inv()`). No pre-existing guarantee
  weakened. `Inner` method specs unchanged.
- `./z build -- all` → `[OK] Build complete.`
- `cheating-detail.txt` reviewed (per-function admit/external_body list).

---

### Per-item determination

**#1 fn-coverage — PASS.** All 11 top-level exec fns carry contracts (`fn_coverage`
0 missing). All 8 `Inner` methods are individually specced. `init` is excluded
(tcb-allowed). `instance` is `external_body` with `ensures` (tcb-allowed).

**#2 caller-coverage — FAIL.** `caller_analysis.md` states callers of `alloc` assume
on `Err`: "allocator state is unchanged and the free pool was empty
(`free_count()==0`/`free_frames.is_empty()`)"; for `book`: frame not free; for
`alloc_range`: region not fully free. The free-function `Err` arms expose only
`true` (see #4). `Inner::alloc`/`book`/`alloc_range` *do* carry these post-conditions,
so the wrapper specs are strictly weaker than both the callee and the documented
caller need. Consequence: `Upool::alloc` (upool.rs:279) states
`Err(_) => old(self)@.free_count()==0` but is forced to remain `#[verus_verify(external_body)]`
because `frame::alloc`'s `Err` arm gives it nothing to derive that from.

**#3 view-consistency — PASS.** All free-fn specs are phrased over
`phys_view().frames` (`FrameAllocView`); all `Inner` specs over `self@: FrameAllocView`
and preserve `inv()` (`final(self).inv()`). View fields (`allocated_frames`,
`free_frames`, `refcounts`) are referenced; no machine types leak. Matches
`view_design.md`.

**#4 no-tautological-ensures — FAIL.** `Err(_) => true` appears in caller-facing
free functions:
- `alloc` (frame.rs:754)
- `alloc_contiguous` (frame.rs:784)
- `book` (frame.rs:865)
- `alloc_range` (frame.rs:885)
(`free`'s `ensures true` is intentional per the `Drop` contract and accepted — see #10.)

**#5 no-subsumed-ensures — PASS (advisory).** Minor: `refcount`'s `Ok` arm asserts
both `allocated_frames.contains(frame@)` and `refcounts.contains_key(frame@)`; by
`FrameAllocView::wf` the latter is derivable from the former. Same for `Inner::refcount`.
Harmless; note for tidy-up, not a blocker.

**#6 error-paths-meaningful — FAIL.** Same evidence as #4. `alloc`/`book`/`alloc_range`
have an expressible, meaningful post-state `Err` fact (their `Inner` callee already
proves it) that is being thrown away.

**#7 no-assume_specification-for-workspace-internal — FAIL.** `frame.spec.rs` declares
`assume_specification` for `crate::hal::mem::*`, which is **intra-crate** (kernel crate),
not an external dependency:
- `FrameAddress::from_frame_number` / `into_frame_number` (lines 37–43)
- `PhysicalAddress::into_frame_number` (line 45)
- `TruncatedMemoryRegion::start` / `size` (lines 49–55)
- `PageAligned::into_raw_value` / `deref` (lines 57–63)
None of these are listed in `verus-ai-logs/tcb-allowed.md`. (`::arch::mem::*` at lines
26–35 is a genuinely separate crate and is acceptable as a temporary placeholder.)
At minimum these intra-crate `assume_specification`s must be recorded in the TCB list
with the same "superseded when the address layer is verified" rationale already used
for `FrameAddress::from_raw_value`/`into_raw_value`.

**#8 vstd-searched-first — PASS.** All `assume_specification` targets are
arch/HAL-specific (frame-number conversions, region accessors, `FRAME_SIZE`); none has
a vstd equivalent.

**#9 specs-for-caller — FAIL (tied to #2/#4/#6).** The free-fn layer is the caller
boundary; its weak `Err` arms make it not directly usable in `Upool::alloc`'s proof.

**#10 trait-obligations — PASS.** `free` carries `opens_invariants none` + `no_unwind`
(frame.rs:823-824), satisfying the `Drop` obligations for `UserFrame::drop`
(upool.rs:201) and `KernelFrame::drop` (kframe.rs:197) documented in `caller_analysis.md`.

**#11 spec-completeness (advisory).** Drive: the missing `Err` post-state facts (#4/#6)
are the concrete completeness gap. Otherwise the `Ok`-arm transitions are full
`FrameAllocView { ... }` rewrites.

**#12 loop-invariants — FAIL.** Both loops in `Inner::alloc_contiguous` (frame.rs:225)
and the two loops in `Inner::alloc_range` (frame.rs:598, 626) use
`#[cfg_attr(verus_keep_ghost, verus_spec(invariant false))]`. `invariant false` is a
stub that only passes because the function bodies are preceded by `admit()`. Real
invariants (defined as `spec fn` in `frame.spec.rs`/`frame.proof.rs` if complex) are
required. This is gated behind the `admit()`s and belongs to the proving phase, but is
recorded here so it is not lost.

**#13 no-cheating-own-functions — REPORTED (proof-phase debt).** Counts from
`cheating-detail.txt`: `assume=0 external_body=14 admit=24 trusted=0`. The module's
*own* exec bodies are all admitted — frame.rs `Inner` methods: `alloc`(136),
`alloc_contiguous`(213), `free`(298), `share`(379), `refcount`(442), `book`(497),
`is_covered`(535), `alloc_range`(583); free fns: `alloc`(757), `alloc_contiguous`(787),
`free_count`(808), `free`(826), `book`(868), `alloc_range`(888). These are placeholders
for the proving phase, not a specification defect, but every one must be discharged
later. Recorded so the proving phase inherits the exact list.

**#14 no-specs-weakened — PASS.** `spec_drift` shows only strengthening; pre-existing
`Inner` and View contracts unchanged.

**#15 bug-awareness — TO CONFIRM.** Potential off-by-one between `Inner::alloc_range`
body and spec: body iterates `start..=end` with
`end = start + size/FRAME_SIZE - 1` (inclusive), while the `ensures` derives
`end_frame_number = (start+size)/spec_page_size()` (exclusive) for the frame set.
These should denote the same set only if `size` is an exact multiple of `FRAME_SIZE`.
Recorded in `verus-ai-logs/nanvix-phys-phys-frame/bugs.md` for the proving phase to
confirm/refute against `region.inv()`.

**#16 cross-module-regression — PASS.** `make verify-kernel` → `47 verified, 0 errors`
across `mm::phys`, `frame`, `kframe`, `manager`, `upool`.

**#17 verification+build — PASS (with cheating flag).** Verus exit 0; `./z build -- all`
OK. The `CHEATING_DETECTED` status reflects the #13 admits (expected at this phase).

---

### Fix Request (lead item: #4 / #6, also resolves part of #2 / #9)

Strengthen the **error-path ensures** of the caller-facing free functions so they
match the documented caller expectations in `caller_analysis.md` and the
post-conditions their `Inner` callees already prove. Do not weaken any `Ok` arm.

1. `frame.rs::alloc` (around line 747-755). Replace `Err(_) => true` with the
   post-state free-pool fact the callers rely on:
   ```rust
   Err(_) => crate::mm::phys::phys_view().frames.free_frames.is_empty(),
   ```
   (Equivalently `free_count() == 0`.) This mirrors `Inner::alloc`'s
   `old(self)@.free_frames.is_empty()` and unblocks `Upool::alloc`'s
   `Err(_) => old(self)@.free_count() == 0`.

2. `frame.rs::book` (around line 859-867). Replace `Err(_) => true` with:
   ```rust
   Err(_) => !crate::mm::phys::phys_view().frames.free_frames.contains(phys_addr@),
   ```
   mirroring `Inner::book`'s `!old(self)@.free_frames.contains(phys_addr@)`.

3. `frame.rs::alloc_range` (around line 878-886). Replace `Err(_) => true` with the
   "region was not fully free" fact mirroring `Inner::alloc_range`
   (`!frames.subset_of(old(self)@.free_frames)`), expressed over the post-state
   region frame set, e.g.:
   ```rust
   Err(_) => !crate::mm::phys::phys_view().frames.all_free(
       crate::mm::phys::region_frame_addrs(region@.start, region@.size)),
   ```
   (Use whatever `FrameAllocView` predicate names the existing `all_reserved`
   counterpart; add an `all_free` spec fn in `mod.spec.rs` only if one does not
   already exist — confirm before adding.)

4. `frame.rs::alloc_contiguous` (around line 775-785). If no expressible post-state
   `Err` fact exists (`Inner::alloc_contiguous` only guarantees state-unchanged, which
   is not expressible at the free-fn boundary without `old()`), then `Err(_) => true`
   may stand — but you MUST justify it with tool output, not prose: show that no
   `FrameAllocView` predicate over the post-state captures a non-trivial `Err`
   condition and that no caller consumes one. Otherwise strengthen it.

These free fns already carry `proof! { admit(); }`, so adding ensures will not block
verification at this phase.

Verify after editing:
- `make verify-kernel` → must stay `0 errors` (and counts unchanged).
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff \
    src/kernel/src/mm/phys/frame.rs --before 97e31408ea7ada8403c40f87f2c785a0161ca80a \
    --after HEAD` → the three (or four) functions must now show **ensures added**
  (strengthening), no "ensures removed" without a strictly-stronger replacement.
- Re-run `fn_coverage.py` (unchanged 11/11).

Do **not** answer with a justification in place of a code change for items 1–3
(those facts are expressible and caller-required). Item 4 is the only one where a
documented "cannot be strengthened" answer is acceptable — and only with tool
evidence.

### Note
No `STOP` file created — multiple checklist items are FAIL (#2, #4, #6, #7, #9, #12)
plus #15 to confirm. Address #4/#6 first (this request), then #7, then #12, then #2/#9
fallout, then confirm #15.
