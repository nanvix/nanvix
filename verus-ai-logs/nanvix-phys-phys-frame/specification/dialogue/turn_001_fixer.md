## Response to Turn 1

### What I Changed

Net change to the repository: **none** (restored to the clean 32-verified / 0-error
baseline, commit `0706bc4df`). Before restoring, I actually *implemented* the
reviewer's `PhysAuth` design and ran the verifier so this report cites real
results, not predictions.

What I implemented and tested (then reverted, see "Result" below):

- `src/kernel/src/mm/phys/mod.spec.rs`: added `pub tracked struct PhysAuth { pub ghost v: PhysMemView }`
  with `view()` / `inv()`, and `spec_alloc_one` / `spec_alloc_set` / `spec_share`
  transitions on `PhysMemView` (fix-request item 1).
- `src/kernel/src/mm/phys/frame.proof.rs`: `use super::PhysAuth;`.
- `src/kernel/src/mm/phys/frame.rs`: rewrote the `alloc()` shim with
  `#[verus_spec(... with Tracked(auth): Tracked<&mut PhysAuth> ...)]` carrying the
  exact strong post-state contract the reviewer asked for:
  ```
  requires phys_view().initialized, old(auth)@.initialized, old(auth)@.inv(),
           old(auth)@ == phys_view(),
  ensures  match result {
             Ok(frame) => frame.inv()
                && final(auth)@ == old(auth)@.spec_alloc_one(frame@)
                && final(auth)@.frames.allocated_frames.contains(frame@)
                && final(auth)@.frames.refcounts[frame@] == 1,
             Err(_) => final(auth)@ == old(auth)@,
           }
  ```
  Body threads the token: `let r = instance(); let res = r.alloc();
  proof! { auth.v.frames = (*r)@; } res`.

### Verification

**1) The reviewer's design core IS sound and verifiable at `frame.rs`.**
With the strengthened `alloc` shim above, `make verify-kernel MODULE=mm::phys`
verifies the `alloc` shim itself — the strong `spec_alloc_one` contract is
provable at frame.rs with no new axiom (it rests only on the already-TCB-allowed
`instance()` bridge + the `Inner::alloc` contract). Result: **31 verified, 1 error**,
and the 1 error is **not** in `frame.rs` — it is the cross-module cascade below.

**2) The strengthening cannot be contained to `frame.rs` — it breaks the verified
caller `Upool::alloc` (outside my scope).** The `with` clause generates a
`requires false` tokenless version for non-Verus callers, so the verified
`Upool::alloc` (which calls bare `frame::alloc()`) fails. Exact errors
(`verus-logs/verus_2026-06-15_09-05-10.log`):

```
error: postcondition not satisfied
   --> src/kernel/src/mm/phys/upool.rs:261:17
261 |                 Ok(uf) => {       ^^^^^^ failed this postcondition
268 |     pub fn alloc(&mut self) -> Result<UserFrame, Error> {  ... at end of body

error: precondition not satisfied
   --> src/kernel/src/mm/phys/upool.rs:269:34
269 |         let addr: FrameAddress = frame::alloc()?;   ^^^^^^^^^^^^^^
   ::: src/kernel/src/mm/phys/frame.rs:732:5
732 |     with Tracked(auth): Tracked<&mut PhysAuth>     ---- failed precondition

error: precondition not satisfied
   --> src/kernel/src/mm/phys/upool.rs:270:12
 56 |             addr.inv(),                             ---------- failed precondition
270 |         Ok(UserFrame::new(addr))                   ^^^^^^^^^^^^^^^^^^^

verification results:: 31 verified, 1 errors
error: could not compile `kernel` (bin "kernel") due to 3 previous errors
```

**Restored baseline** (`make verify-kernel MODULE=mm::phys`, current HEAD):
```
verification results:: 32 verified, 0 errors (exit 0)
```
`./z build` → `[OK] Build complete.` (dual compilation intact).

### Result: NEEDS_GUIDANCE

I implemented the design and ran it. The frame.rs-level mechanism works, but the
full fix request requires changes that conflict with this task's **hard scope
rules** and is **partly infeasible**. Three blockers, each with evidence:

**Blocker A — full threading touches do-not-touch files (out of scope).**
My in-scope function list is *only* the `frame.rs` functions. Fix-request items
2/3 require editing verified callers in `upool.rs` (`Upool::alloc`, shown above)
and `manager.rs`. The hard rule "Do not touch unlisted functions" forbids this.
Any token threading on a shim *forces* every verified caller to thread too (the
tokenless version is `requires false`), so the change cannot stop at frame.rs.
Each of the 5 mutating shims (`alloc`, `alloc_contiguous`, `book`, `alloc_range`,
`share`) has its own verified-caller chain in `upool.rs` / `kframe.rs`.

**Blocker B — fix-request item 3 is factually mis-premised AND technically
infeasible.** It says the manager axioms "are not in `tcb-allowed.md` and must be
removed (guarantee derived from threaded shims)." Both counts are wrong:

- *They ARE in `tcb-allowed.md`.* Section "Allowed `external_body` —
  `PhysMemoryManager` (`manager.rs`)" (lines 106–135) explicitly lists
  `alloc_user_frame`, `alloc_kernel_frame`, `alloc_many_user_frames`,
  `alloc_many_kernel_frames`, `init`, `check_user_watermark`. A sanctioned trust
  boundary, not an illicit relocation.
- *They CANNOT be body-verified even if threaded.* `manager.rs` contains **9**
  untranslatable constructs (`grep -cE "error!|inspect_err|and_then|debug_assert"
  manager.rs` = 9). `tcb-allowed.md` itself states these methods "have no
  verifiable body … use side-effecting combinators (`inspect_err`/`and_then`/
  `ok_or_else`) and `error!`/`warn!` macros that are not ghost-gated and have no
  `vstd` specs" — the identical wall that forced `external_body` on the 8
  `Inner::*` methods. De-`external_body`'ing them produces compile errors
  (`core::fmt::Arguments` unsupported), so the strong guarantee *must* stay an
  `external_body` contract; it cannot be "derived" in `manager.rs`.

**Blocker C — removing `phys_view()` (item 1) breaks 11 files with do-not-modify
+ TCB specs.** `phys_view()` is referenced **88 times across 11 files**:
`manager.rs` (35), `upool.rs` (24), `mod.rs` (11), `upool.spec.rs` (7),
`kframe.spec.rs` (6), `manager.spec.rs` (5), `kframe.rs` (3), `frame.rs` (62 — the
shim specs), plus `manager.proof.rs`/`frame.proof.rs`/`mod.spec.rs`. Most are in
do-not-modify spec definitions (`FrameAllocView`, `PhysMemView::inv`) or TCB
external_body contracts. Replacing the constant with a threaded `PhysAuth`
requires rewriting all of them — far outside frame.rs-only scope, and several are
explicitly do-not-modify.

**Net:** the reviewer's success criteria (`admit=0` on the manager path, no
`CHEATING_DETECTED` from manager axioms, guarantee derived from shims) are
**unachievable** because the manager functions are an irreducible trust boundary
(error! macros) that `tcb-allowed.md` already sanctions. Even a maximal in-scope
effort — threading all 5 shims + their verified callers — would still leave the
manager axioms `external_body` (CHEATING_DETECTED persists) while violating "do
not touch unlisted functions" in `upool.rs` / `manager.rs` / `kframe.rs`.

**Guidance requested (pick one):**
1. **Expand scope explicitly** to authorize editing `upool.rs`, `manager.rs`,
   `kframe.rs`, `mod.rs`, and the do-not-modify specs, AND **accept** that the
   four `manager.rs` `alloc_*` methods remain `external_body` (untranslatable
   `error!`) with contracts re-expressed over `PhysAuth` — i.e. keep them in
   `tcb-allowed.md`, do not "derive" them. Then I will thread `PhysAuth`
   end-to-end and reach 0 errors (CHEATING_DETECTED will still report the
   TCB-sanctioned external_body count, which is expected per `tcb-allowed.md`).
2. **Confirm the frame.rs-only scope stands**, in which case the current
   pre-state shim specs are the strongest contracts expressible without a tracked
   carrier (a 0-ary `phys_view()` constant provably cannot name post-state; see
   reproducer `02_goal_is_false.rs`), and the strong post-state guarantee must
   continue to live in the TCB-sanctioned `manager.rs` / shim `external_body`
   contracts. The baseline (32 verified, 0 errors) already reflects this.

I cannot satisfy both the fix request and the hard scope/TCB rules
simultaneously; they are in direct conflict, so I need a ruling on which governs.
