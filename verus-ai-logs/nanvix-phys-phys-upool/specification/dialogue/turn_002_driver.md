## Turn 2: Verify fix for #4/#6 (`UserFrame::share` tautological Err arm) — FIXED; final checklist pass

### Progress
- Done (PASS / FIXED): ALL 17 checklist items.
  - PASS turn 1: #1, #3, #5, #7, #8, #9, #11, #12, #13, #14, #15, #16, #17.
  - FIXED this turn: **#4 No tautological ensures** and **#6 Error paths meaningful (share)**.
  - Accepted-as-upstream-limited (documented, not faulted): #2 (refcount-transition), #10 (`drop` `release` semantics) — both blocked by `old(phys_view())` being inexpressible against the 0-arg `phys_view()` accessor in `mod.spec.rs` (do-not-modify); consistent with the already-verified frame layer, recorded in `view_design §8` and `bugs.md`.
- Current: closing out — all items verified.
- Remaining: none.

### Verification (independent, not trusting fixer claims)

**Read the actual changed code:**
- `upool.rs:157-160` — `UserFrame::share` Err arm is now:
  ```rust
  Err(_) => {
      ||| !crate::mm::phys::phys_view().frames.allocated_frames.contains(self@)
      ||| crate::mm::phys::phys_view().frames.refcounts[self@] >= 255
  },
  ```
  The forbidden `Err(_) => true` is gone.
- `frame.rs:811-815` — trusted `frame::share` Err arm strengthened identically. This is sound vs. `Inner::share` (frame.rs:368): `Err` ⟺ out-of-bounds / `refcount==0` (⟹ not in `allocated_frames`) or `checked_add` overflow (⟹ `refcount==255`).

**Tautology / cheating scan** (`grep "Err(_) => true|admit|assume|assume_specification|trusted"` over `upool.rs` + `.spec.rs` + `.proof.rs`) → **NONE**. `grep -c "Err(_) => true" upool.rs` → 0.

**spec_drift (no weakening):**
- `upool.rs` (before view-design end `975741f6` → HEAD): **Ensures removed: 0**, requires added 5, functions added 1 — additions/strengthening only.
- `frame.rs` (`975741f6` → HEAD): **Ensures removed: 0**; `share`/`refcount` show "ensures added" — the `share` Err arm is strictly stronger, no removal.

**Forced (cache-busted) verification** — to avoid trusting the cached "no recompilation" result, I `touch`ed `upool.rs` and re-ran `make verify-kernel`:
```
verification results:: 26 verified, 0 errors
Exit code : 0
```
The new upool Err arm discharges directly from the strengthened `frame::share` Err arm (no admit, no bridge lemma). Cross-module: 26 verified / 0 errors (no regressions).

**fn_coverage** (re-confirmed turn 1): 7/7 source exec fns matched, 0 missing.

**Cheating detail (upool module)** — only the 3 TCB-allowed `external_body` entries remain (line numbers shifted to 221/246/279, confirming the cache tracks the changed source):
```
mm/phys/upool.rs:221 Upool (struct): external_body
mm/phys/upool.rs:246 new: external_body
mm/phys/upool.rs:279 alloc: external_body
```
upool: admit=0, assume=0, trusted=0. `UserFrame` methods remain genuinely verified (not external_body).

### Per-item final status
1. requires/ensures on every exec fn — PASS (7/7).
2. Caller coverage — PASS w/ documented upstream limit (refcount transition inexpressible; snapshot facts + round-trip cover the rest).
3. View consistency — PASS.
4. No tautological ensures — **FIXED** (`share` Err arm replaced; grep clean).
5. No subsumed ensures — PASS.
6. Error paths meaningful — **FIXED** for `share`; `refcount`/`alloc` already meaningful; `drop` effect inexpressible (upstream).
7. No assume_specification for workspace-internal — PASS (none).
8. vstd searched before assume_specification — PASS (N/A).
9. Specs usable in caller proofs — PASS.
10. Trait obligations — PASS w/ documented upstream limit (`Drop` `release` semantics; `View` = address matches callers).
11. Spec completeness (advisory) — PASS (nondeterminism matches caller deferral).
12. Loop invariants — PASS (no loops).
13. No cheating on module's own fns — PASS (admit/assume/trusted = 0; external_body×3 TCB-allowed).
14. No specs weakened — PASS (spec_drift: 0 ensures removed, both files).
15. Bug awareness — PASS (`bugs.md` current).
16. Cross-module regression — PASS (26 verified / 0 errors, forced run).
17. Verification + build — PASS (exit 0; fixer ran `./z build` → OK).

### Fix Request
None. All checklist items PASS or FIXED with tool evidence. Writing STOP = RESOLVED.
