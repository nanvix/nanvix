## Turn 2: Item 3/6 external_body audit — verify fixer's tcb-allowed.md correction

### Progress
- Done (PASS/FIXED): all 10 items.
  - (1) spec-drift, (2) admit=0, (4) assume=0, (5) no cfg-gated exec,
    (7) Verus-limitation reproducer (N/A), (8) exec rewrites (none),
    (9) cross-module regression, (10) verification+build — all PASS in Turn 1.
  - (3)/(6) external_body audit — **FIXED this turn** (doc rationale corrected).
- Current: item 3/6 (the sole Turn-1 FAIL).
- Remaining: none.

### Verification

The Turn-1 FAIL was not the *presence* of the 3 upool `external_body`
(design-forced + listed in `tcb-allowed.md`), but that their listed rationale was
false/self-contradictory ("no specs yet" for `new`; "Verified when upool is" for
`alloc`). I verified the fixer's correction independently — NOT by trusting the
fixer's prose:

1. **Stale phrases gone for upool.**
   `grep -n "no specs yet\|Verified when .upool. is" verus-ai-logs/tcb-allowed.md`
   → only `manager.rs:init` (out of scope) matches; **zero** upool matches.

2. **Entries reclassified with accurate rationale** (read
   `tcb-allowed.md:91–118` directly):
   - New dedicated section L91: *"`external_body` thin-facade trust boundaries in
     `mm::phys::upool` (permanent until the frame free-function layer is
     verified)"*.
   - `Upool` (struct) L101 — opaque `external_body`; `View` is
     `uninterp spec fn view() -> FrameAllocView`; unconstructible in verified
     code. Accurate.
   - `Upool::new` L106 — states the **real** contract `ensures result@.wf()` and
     correctly explains `FrameAllocView::wf()` is unprovable from an
     uninterpreted view (assumed §8 ghost-attachment axiom). Matches the actual
     `#[verus_spec]` at `upool.rs:242`.
   - `Upool::alloc` L112 — correctly states it stays `external_body` *until the
     frame free-function layer is verified*, sibling of `frame::alloc`/`book`/
     `share`, NOT "when upool is verified". Matches `upool.rs:263` contract
     (`alloc_one` transition + empty-pool `Err free_count()==0`).
   The old stale lines are removed from the "Cross-module dependencies trusted
   until their module is verified" section (L120+ now contains only
   frame::init / manager::init / kframe::map_frame / frame::* — no upool).

3. **Doc-only change — source untouched.**
   `git diff HEAD -- upool.rs upool.spec.rs upool.proof.rs` → empty. The
   `external_body` attributes were NOT removed (the forbidden path), exactly as
   required.

4. **Independent re-run of `make verify-kernel`** (not the fixer's quote):
   ```
   verification: cached (no recompilation), — (exit 0)
   cheating: assume=0 external_body=15 admit=7 trusted=0 no_decreases=0 cfg_gate=12
   ```
   Exit 0; counts unchanged (external_body=15 / admit=7 / cfg_gate=12), as
   expected for a pure documentation change. `kernel::all` PASS; no regression.
   0 errors, 0 warnings on the verus build.

5. **Pre-existing `./z build` lint** (`manager.rs:245` unused `i` under
   `-D warnings`) is out of scope: `git diff HEAD -- manager.rs` is empty, it is
   in a different (already-committed) module, and the checklist's build gate for
   this target is `make verify-kernel` (exit 0, confirmed). Not introduced by the
   upool proving phase. Noted, not charged against upool.

Re-confirmed the remaining items still hold at HEAD: upool has admit=0,
assume=0, no cfg-gated exec (only the ghost `verus!` block + sanctioned `error!`
log in `Drop`), no `// VERUS REWRITE`, and the spec surface is unchanged from
proving START. UserFrame's `view`/`inv`/`new`/`address`/`leak`/`share`/
`refcount`/`drop` are all machine-verified (none appears in cheating-detail).

### Fix Request

None. Item 3/6 is FIXED with tool-verified evidence. All 10 checklist items are
PASS/FIXED. Creating STOP = RESOLVED.
