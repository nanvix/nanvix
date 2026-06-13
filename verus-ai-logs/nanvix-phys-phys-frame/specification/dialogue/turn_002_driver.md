## Turn 2: Verifying Fixer Turn 1 — Full Re-walk

Source: `src/kernel/src/mm/phys/frame.rs`
Verus:  `make verify-kernel`

### Progress
- FIXED this turn (verified): #2 caller-coverage, #4 no-tautological-ensures,
  #6 error-paths-meaningful, #7 no-assume_spec-workspace-internal, #9 specs-for-caller.
- PASS (re-confirmed): #1, #3, #5, #8, #10, #11, #14, #16, #17.
- Deferred to proving phase (reported & recorded, not spec defects): #12 loop
  invariants, #13 module-own admits, #15 alloc_range off-by-one (in `bugs.md`).
- Remaining: none. All checklist items PASS/FIXED → RESOLVED.

---

### Verification (I ran these myself — did not trust fixer claims)

- `make verify-kernel` → **48 verified, 0 errors, exit 0**.
  `assume=0 external_body=14 admit=24 trusted=0 cfg_gate=17`. (48 vs 47 last turn:
  `is_covered`/`book`/`alloc_range` now carry the added `requires`/`ensures` so an
  extra obligation is checked.)
- `fn_coverage.py frame.rs frame.rs` → **Matched 11/11, Missing 0, Extra 0**.
- `spec_drift.py git-diff frame.rs --before 97e31408 (view-design) --after HEAD` →
  5 funcs changed; the single "ensures removed" is `alloc`'s match block **replaced by
  a strictly stronger one** (`Ok` adds `allocated_frames.contains(frame@)`; `Err`
  `true` → `free_frames.is_empty()`). `alloc_range`/`book`/`is_covered`: requires
  added (match documented caller expectations) + ensures added. **No guarantee
  weakened.**
- `./z build -- all` → **[OK] Build complete.**

---

### Per-item determination (changed items)

**#4 / #6 — FIXED.** Read the actual code (`grep 'Err(_) =>' frame.rs`):
- `alloc` (frame.rs:754): `Err(_) => phys_view().frames.free_frames.is_empty()` —
  mirrors `Inner::alloc` (frame.rs:132). ✔
- `book` (frame.rs:865): `Err(_) => !phys_view().frames.free_frames.contains(phys_addr@)`
  — mirrors `Inner::book` (frame.rs:493). ✔
- `alloc_range` (frame.rs:885-886): `Err(_) => !phys_view().frames.all_free(
  region_frame_addrs(region@.start, region@.size))` — mirrors `Inner::alloc_range`'s
  `!frames.subset_of(old(self)@.free_frames)`. `all_free` exists (mod.spec.rs:141,
  definitionally `set.subset_of(free_frames)`). ✔
- `alloc_contiguous` (frame.rs:784): `Err(_) => true` **retained — accepted with code
  evidence I independently verified, not on the fixer's word:**
  1. `Inner::alloc_contiguous` (frame.rs:208-210) Err arm = `final(self)@ == old(self)@`
     only — a relation between old and final, no other fact proved on Err.
  2. Free-fn specs are over `phys_view()` (a fixed `uninterp spec fn`, no `old()` form),
     so "state unchanged" is **not expressible** as a post-state-only predicate.
  3. Sole caller `alloc_many_kernel_frames` (manager.rs:449) calls
     `frame::alloc_contiguous(count)?` and consumes **no** abstract `Err` fact — its
     own Err arm (`final(self)@ == old(self)@`, manager.rs:417-420) is discharged from
     its loop invariant `self@ == g_old` (manager.rs:455), not from the wrapper's
     postcondition. Verified by reading manager.rs:423-499.
  This is the genuinely-strongest expressible `Err` fact (matches #11 allowance for
  intentional nondeterminism that matches caller expectations), not lazy `true`.

**#2 / #9 — FIXED.** The strengthened free-fn `Err` arms now match the documented
caller expectations in `caller_analysis.md` (`alloc` Err ⇒ free pool empty; `book` Err
⇒ frame not free; `alloc_range` Err ⇒ region not fully free) and give
`Upool::alloc`'s `Err(_) => old(self)@.free_count() == 0` something to derive from.
Whether `Upool::alloc` drops its `external_body` is proving-phase work; the spec is now
caller-usable.

**#7 — FIXED.** `tcb-allowed.md` now has section "`assume_specification` for
not-yet-verified callees" separating genuinely-external `::arch::*` (lines 96-98) from
the intra-crate `crate::hal::mem::*` declarations (lines 103-107:
`FrameAddress::{from,into}_frame_number`, `PhysicalAddress::into_frame_number`,
`TruncatedMemoryRegion::{start,size}`, `PageAligned` `into_raw_value`/`deref`), with the
"removed when the address layer is verified" rationale. This follows the established
project convention already present for `FrameAddress::{from,into}_raw_value` (the
hal::mem address layer is an out-of-scope, verified-later dependency boundary, not a
verification escape). Recorded, acceptable.

---

### Deferred to proving phase (reported, recorded — NOT specification defects)

**#12 loop invariants.** Three loops carry `verus_spec(invariant false)` stubs
(frame.rs:224 in `Inner::alloc_contiguous`; 597, 625 in `Inner::alloc_range`). Each
loop **has** an invariant clause (syntactic requirement met); the `false` placeholders
are sound only because each body is preceded by `proof! { admit(); }`. Writing the real
invariants (as `spec fn` where complex) is part of discharging the `admit()` in the
proving phase. Not a spec-phase contract defect.

**#13 module-own admits.** `cheating-detail.txt`: `admit=24`. The 14 frame.rs admits
were individually enumerated in turn 1 (8 `Inner` methods + 6 free fns). These are
spec-phase body placeholders; the requires/ensures contracts are complete. Every one
must be discharged in the proving phase. `external_body` items (`instance`, `init`,
hal address fns) are all in `tcb-allowed.md`.

**#15 alloc_range off-by-one.** Recorded in
`verus-ai-logs/nanvix-phys-phys-frame/bugs.md` (inclusive body range
`start..=(start+size/FS-1)` vs half-open spec set); to confirm against `region.inv()`
alignment guarantees in the proving phase.

---

### Result: RESOLVED

Every specification-phase-actionable checklist item is PASS or FIXED with
independently-verified tool evidence. Items #12/#13/#15 are correctly deferred to the
proving phase per the pipeline's phase separation and are recorded for it.
`make verify-kernel` 48 verified / 0 errors; `./z build -- all` OK; `fn_coverage` 11/11;
`spec_drift` shows only strengthening.
