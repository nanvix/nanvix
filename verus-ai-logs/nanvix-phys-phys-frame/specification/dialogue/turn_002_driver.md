## Turn 2: Adjudicating the fixer's NEEDS_GUIDANCE — scope refuted, two turn-1 premises corrected, core FAIL stands

### Progress
- Done (PASS): items 1, 7, 8, 10, 12, 15, 16, 17.
- Corrected this turn: item 13 (my turn-1 "delete manager axioms" demand was wrong — WITHDRAWN).
- Current FAIL (unchanged root cause): items 2, 3, 9, 14 — verified shims **and verified callers** still carry weakened pre-state specs; the approved `view_design.md` (PhysAuth) is not implemented.
- Remaining/secondary: item 4/6 (Err arms — were fixed in the prior round, still present), item 5, item 13's lone `admit` in `free`.

### Verification (commands run, files read — I verified BOTH parties' claims)

The fixer reverted to baseline and returned **NEEDS_GUIDANCE** with three blockers.
I checked each against the actual repository.

**Fixer claim 1 (Blocker B, part 1): "the manager `alloc_*` ARE in `tcb-allowed.md`."**
→ **TRUE.** `tcb-allowed.md` lines 106–135, section "Allowed `external_body` —
`PhysMemoryManager`", explicitly lists `init`, `alloc_user_frame`,
`check_user_watermark`, `alloc_many_user_frames`, `alloc_kernel_frame`,
`alloc_many_kernel_frames`. **My turn-1 item-13 claim that they "are not in
tcb-allowed.md and must be removed" was factually wrong. I withdraw it.**

**Fixer claim 2 (Blocker B, part 2): "manager `alloc_*` cannot be body-verified."**
→ **TRUE.** `grep -nE "error!|warn!|inspect_err|and_then|ok_or_else"
src/kernel/src/mm/phys/manager.rs` → lines 205,210,309,311,316,354,355,357,417,
422,430,440. These are the same untranslatable `core::fmt::Arguments` /
side-effecting-combinator constructs that forced `external_body` on `Inner::*`.
**So the manager `alloc_*` legitimately stay `external_body`; the strong guarantee
cannot be "derived" inside them.** My turn-1 demand to derive them was infeasible.

**Fixer claim 3 (Blocker A): "my in-scope function list is ONLY the frame.rs
functions; editing `upool.rs`/`manager.rs` is out of scope."**
→ **FALSE.** `verus-ai-logs/verification-plan.json` is the scope authority for this
target (`"target": "src/kernel/src/mm/phys/frame.rs"`). Its `entries` list the
**in-scope functions across the whole `mm::phys` subsystem**, explicitly including:
```
src/kernel/src/mm/phys/upool.rs:Upool::alloc
src/kernel/src/mm/phys/upool.rs:UserFrame::new / share / refcount / leak / drop
src/kernel/src/mm/phys/manager.rs:PhysMemoryManager::alloc_user_frame
src/kernel/src/mm/phys/manager.rs:PhysMemoryManager::alloc_kernel_frame
src/kernel/src/mm/phys/manager.rs:...alloc_many_user_frames / alloc_many_kernel_frames / init
src/kernel/src/mm/phys/kframe.rs:KernelFrame::new / base / drop
src/kernel/src/mm/phys/mod.rs:book_physical_memory_regions / book_mmio_regions / init
```
`upool.rs`, `manager.rs`, `kframe.rs`, `mod.rs` are **in scope**. Blocker A rests on
a false premise.

**The threading win is real and in-scope.** I read `upool.rs:247-271`:
`Upool::alloc` is a **verified `#[verus_spec]`** function (NOT `external_body`),
and it carries the SAME weakened spec the rollback condemned:
```
Ok(uf) => uf.inv() && phys_view().frames.free_frames.contains(uf@)   // pre-state only
Err(_) => true                                                       // tautology
```
Strengthening `frame::alloc` + threading the carrier lets this verified caller
PROVE the strong post-state (`allocated_frames.contains(uf@) && refcounts[uf@]==1`)
instead of relaying a pre-state fact. That is exactly the rollback's requirement,
and it lands entirely inside `verification-plan.json` scope.

**`make verify-kernel`** (current HEAD, fixer's restored baseline) → exit 0,
"32 verified, 0 errors", `status: CHEATING_DETECTED` (assume=0 external_body=22
admit=1). `grep PhysAuth src/` → none; `mod.spec.rs:171` still the 0-ary constant.
**The approved `view_design.md` design is NOT implemented.**

### Ruling

The fixer is **right on the two manager facts** (claims 1 & 2) and **wrong on
scope** (claim 3). The conflict the fixer reported does not actually exist:

- "Do not touch unlisted functions" does NOT block this fix, because
  `upool.rs`/`manager.rs`/`kframe.rs`/`mod.rs` functions **are listed** in
  `verification-plan.json`.
- The manager `alloc_*` staying `external_body` is **acceptable** — they are in
  `tcb-allowed.md`. Checklist item 13 permits TCB-sanctioned `external_body`. They
  do NOT need to be "derived" or body-verified. (My turn-1 sub-demand is withdrawn.)

So the fixer's own "option 1" path is the correct, authorized one — and it does
NOT require the impossible part (deriving manager bodies). **NEEDS_GUIDANCE is
resolved: proceed with end-to-end threading; manager `alloc_*` remain
`external_body` with contracts re-expressed over the carrier.**

### Per-item status (delta from turn 1)
- [PASS] 1, 7, 8, 10, 12, 15, 16, 17 — unchanged.
- [FAIL] 2 Caller coverage — verified callers (`Upool::alloc`) still relay pre-state, not the caller-required post-state.
- [FAIL] 3 View consistency — specs do NOT implement the approved `view_design.md` (PhysAuth absent; 0-ary `phys_view()` retained). **Primary failure.**
- [PASS→retained] 4/6 Err arms — `alloc`/`book`/`alloc_range`/`share` shim Err arms are non-tautological (verified in prior round); `alloc_contiguous`(786)/`init`(686) `Err=>true` are contract-justified. After strengthening, give `alloc`/`alloc_contiguous` a meaningful `Err => auth@ == old(auth)@`.
- [FAIL] 9 Specs usable by caller — `Upool::alloc` cannot prove the strong post-state from the weak shim spec.
- [WITHDRAWN] 13 manager axioms — they are TCB-allowed `external_body` (untranslatable `error!`); NOT a violation. **Remaining item-13 concern: the single `admit()` in `frame::free` (frame.rs:846/853) — must be eliminated or shown unnecessary (see fix #4).**
- [FAIL] 14 Specs weakened — root cause; the approved remedy (carrier) is unimplemented.

### Fix Request (CORRECTED — authorized scope, feasible path)

Implement the approved `view_design.md` carrier END-TO-END across the
`verification-plan.json` in-scope functions. The manager `external_body` boundary
stays; only the **verified** layer is strengthened.

1. **`mod.spec.rs`**: add the diff-able carrier `pub tracked struct PhysAuth`
   with `spec fn view(self) -> PhysMemView` and `inv(self) == self.view().inv()`,
   plus `spec_alloc_one` / `spec_alloc_set` / `spec_share` on `PhysMemView`
   (`spec_book_frame`/`spec_book_frames` retained as aliases). You may KEEP the
   0-ary `phys_view()` for the query/`init`/`Drop` paths that cannot thread a token
   (minimizes churn) — the carrier is required only on the **mutating** verified
   path. `FrameAllocView` stays verbatim (do-not-modify).

2. **`frame.rs`**: redesign `instance()` to take `Tracked(&mut PhysAuth)` bridging
   `(*r)@ == auth@.frames`, `auth@ == old(auth)@` (per `view_design.md`). Thread
   `Tracked(&mut PhysAuth)` through the mutating shims and restore STRONG
   post-state, meaningful Err arms:
   - `alloc`: `Ok ⇒ frame.inv() && auth@ == old(auth)@.spec_alloc_one(frame@) &&
     allocated_frames.contains(frame@) && refcounts[frame@]==1`; `Err ⇒ auth@==old(auth)@`.
   - `book`: `Ok ⇒ auth@ == old(auth)@.spec_alloc_one(phys_addr@) && allocated_frames.contains(phys_addr@)`.
   - `alloc_range`: `Ok ⇒ auth@ == old(auth)@.spec_alloc_set(region_frames) && region_frames.subset_of(allocated_frames)`.
   - `alloc_contiguous`: `Ok ⇒` the `count` page-strided frames now allocated, each refcount 1.
   - `share`: `Ok ⇒ auth@ == old(auth)@.spec_share(frame@)`.
   Query shims (`refcount`, `is_covered`, `free_count`) take `Tracked(&PhysAuth)`.

3. **Thread the verified callers (in scope — this is the step you skipped):**
   `upool.rs:Upool::alloc`, `UserFrame::share`/`refcount`, `kframe.rs:KernelFrame::new`
   must accept/forward the `Tracked` carrier and tighten their `ensures` to the
   strong post-state (`Upool::alloc` Ok ⇒ `allocated_frames.contains(uf@) &&
   refcounts[uf@]==1`). The `manager.rs:alloc_user_frame`/`alloc_kernel_frame`/
   `alloc_many_*` **stay `external_body`** (tcb-allowed); re-express their
   `#[verus_spec]` contracts over the carrier and **hold** `Tracked<PhysAuth>` in
   `PhysMemoryManager`, threading `&mut self.auth` into the shims. Do NOT attempt
   to body-verify them — that is infeasible (`error!`) and unnecessary.

4. **`frame::free`**: keep the weak `opens_invariants none` / `no_unwind` Drop
   contract (the caller-justified exception), but **remove `proof! { admit(); }`**.
   If the postcondition is genuinely always-true it needs no `admit`; if it cannot
   be discharged without `admit`, state a Drop-compatible always-true `ensures`
   that verifies cleanly. `admit` on this module's own function is an item-13 flag.

5. **`bugs.md`**: mark the "phys_view() is a constant → post-state inexpressible"
   limitation **resolved by the carrier** for the verified mutating path; note the
   manager `external_body` boundary is retained per `tcb-allowed.md`.

**Verify:** `make verify-kernel` must report **0 errors**, `admit=0` for
`frame.rs`, and the verified `Upool::alloc` must carry the strong post-state
(read its `ensures` and confirm `allocated_frames.contains(uf@)`); then
`./z build` (or `make all-kernel`) must succeed. The residual
`external_body=22` from the TCB-sanctioned manager/`Inner::*` boundary is
expected and acceptable.

Do not revert and re-request guidance citing scope — `verification-plan.json`
settles it: `upool.rs`/`manager.rs`/`kframe.rs`/`mod.rs` are in scope. Thread the
carrier through them.

**Verdict: FAIL.** STOP not created — items 2, 3, 9, 14 unresolved.
