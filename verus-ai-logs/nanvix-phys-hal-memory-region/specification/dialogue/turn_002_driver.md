## Turn 2: Verify fix — No cheating on module's own functions (admit removed)

### Progress
- Done (PASS/FIXED):
  - **No cheating on module's own functions** → **FIXED** (was the sole FAIL; verified below).
  - Every in-scope exec function has requires/ensures — 4/4 (`grep -c verus_spec` = 4).
  - Caller coverage — ensures supply `spec_addr(&result)==self@.start` / `result as int==self@.size`.
  - View consistency — specs reference `self@.start`/`self@.size`; View+inv match view_design.md.
  - No tautological / no subsumed ensures; error paths N/A (infallible reads).
  - No assume_specification for workspace-internal code (region.{rs,spec,proof}.rs: 0).
  - vstd searched before assume_specification — N/A (none present).
  - Specs written for the caller / trait obligations satisfied (Ord-by-start preserved).
  - Loop invariants — no loops.
  - No specs weakened — region.spec.rs was an empty stub pre-phase; all changes additive.
    The `Address` supertrait change (`+ Copy`) **strengthens** a bound, not weakens.
    All four ensures are byte-for-byte intact (grep confirmed lines 218/227/382/391).
  - Bug awareness — no fundamentally incorrect code found; no bugs to record.
  - Cross-module regression — verify-kernel, verify-sys, verify-bitmap, verify-nanvix-slab,
    verify-bump-allocator all exit 0.
  - Verification + build — `make verify-kernel` exit 0; `./z build` → `[OK] Build complete.`
- Current: final sign-off.
- Remaining: none.

### Verification
Read the fixer's changes (committed in `0d6650056`, not just claimed):

1. `src/kernel/src/hal/mem/types/region.rs:220-222` — `MemoryRegion::start` body changed
   from `self.start.clone()` to `self.start`; `proof! { admit(); }` **removed**.
   Ensures unchanged: `spec_addr(&result) == self@.start` (line 218).
2. `src/libs/sys/src/sys/mm/address/mod.rs:33` — `Address` supertrait gained `+ Copy`
   (diff: `Clone + PartialEq...` → `Clone + Copy + PartialEq...`). All four Address impls
   already derive `Copy`, so a `Copy` field read is identity in Verus and discharges the
   postcondition by congruence — no clone law, no admit.
3. `region.spec.rs` — unchanged View/inv/helpers; no broken clone-spec residue.

I did not accept the fixer's narrative — I re-ran every tool:

```
make verify-kernel  → Exit 0
  cheating: assume=0 external_body=25 admit=0 trusted=0 no_decreases=0 cfg_gate=7
grep -nE "region\.(rs|spec\.rs|proof\.rs)" cheating-detail.txt → NONE (clean)
grep admit|assume|external_body|trusted in region.{rs,spec,proof}.rs → NONE
```
`admit` for this module dropped **1 → 0**. The remaining 25 `external_body` are in other
modules (page/frame/manager/mod), pre-existing TCB-allowed, out of scope.

Cross-module regression (the `+ Copy` change touches the shared `sys` crate):
```
make verify-sys          → Exit 0, status CLEAN, admit=0
make verify-bitmap       → Exit 0, admit=0 (cfg_gate=5 pre-existing baseline)
make verify-nanvix-slab  → Exit 0, admit=0 (cfg_gate=1 pre-existing)
make verify-bump-allocator → Exit 0, admit=0 (external_body=2 pre-existing)
make verify-kernel       → Exit 0
```
Build (validates the supertrait change compiles for every Address impl in a normal build):
```
./z build → [OK] Build complete.  (EXIT=0)
```

The reviewer's literal suggestion (assume_specification on generic `<T as Clone>::clone`)
is structurally rejected by Verus; the fixer documented 4 failed mechanisms and instead
eliminated the obligation at its root via `Copy`. That is a code change, not a
justification, and the tool output proves it: `admit=0`, ensures unchanged, all crates
build and verify. Accepted.

### Fix Request
None — all checklist items PASS or FIXED with tool evidence. Phase resolved.
