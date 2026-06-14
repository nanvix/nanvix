## Turn 2: Item 7 (assume_specification for workspace-internal code) — verified FIXED; full checklist RESOLVED

### Progress
- **Done (all 18 items PASS/FIXED):**
  1. In-scope exec fns have requires/ensures — PASS
  2. Caller coverage — PASS
  3. View consistency — PASS
  4. No tautological ensures — PASS
  5. No subsumed ensures — PASS
  6. Error paths have meaningful ensures — PASS
  7. **No assume_specification for workspace-internal code — FIXED this turn**
  8. vstd searched before assume_specification — PASS (no remaining assumes in arch)
  9. Specs written for the caller — PASS
  10. Trait obligations satisfied — PASS (none)
  11. Spec completeness (advisory) — PASS
  12. Loop invariants — PASS (no loops)
  13. No cheating on module's own functions — PASS (admit/external_body/trusted/assume = 0)
  14. No specs weakened — PASS (spec-drift: 0 contract drift on all changed files)
  15. Bug awareness — PASS (no bugs)
  16. Cross-module regression — PASS (`make verify` exit 0, all crates)
  17. Verification — PASS (`make verify-arch` CLEAN; `./z build -- all` OK)
- **Current:** Item 7 re-verification (below)
- **Remaining:** none

### Verification

The fixer implemented Option 2 (interpret `spec_max()`). I verified every claim independently.

**1. `assume_specification[ FrameNumber::MAX ]` removed; `spec_max()` now interpreted.**
`number.spec.rs` no longer contains any `assume_specification`. `spec_max()` is now:
```rust
pub open spec fn spec_max() -> nat {
    (mem::MAX_ADDRESS / mem::FRAME_SIZE - 1) as nat
}
```
`grep -rn assume_specification src/libs/arch/src/` → **no matches**. The arch crate is assume-free.

**2. Exec `MAX` moved into the `#[verus_verify] impl` (number.rs:34-37)** so its concrete body
`MAX_ADDRESS / FRAME_SIZE - 1` is verified against `spec_max()`, not assumed.

**3. Supporting constants made Verus-visible** (`constants.rs`): `PAGE_SIZE` (=4096),
`MAX_ADDRESS` (=usize::MAX), `FRAME_SIZE` (=PAGE_SIZE) now `#[verus_verify]`. Confirmed
`PAGE_SIZE` annotated at line 48.

**4. Two kernel placeholder assumes superseded (not weakened):**
- `frame.rs`: `spec_page_size()` changed `uninterp` → `open { ::arch::mem::PAGE_SIZE as int }`;
  `assume_specification[PAGE_SIZE]` removed. Concretizing an uninterpreted fn to its real value is
  strengthening; callers relying on `PAGE_SIZE == spec_page_size()` still get it definitionally.
- `mm/phys/frame.spec.rs`: `assume_specification[FRAME_SIZE]` removed (now reduces to verified 4096).

**Tools run (this turn, independently):**
- `make verify-arch` → Exit 0, **status CLEAN**, `assume=0 external_body=0 admit=0 trusted=0`.
- `make verify` (full workspace) → Exit 0 for every crate. arch CLEAN. Kernel
  `external_body=11 admit=28 cfg_gate=15` — **identical to the pre-phase baseline** (turn 1),
  so no regression; net workspace `assume` count dropped by 3.
- `spec_drift git-diff` on `number.spec.rs`, `phys.spec.rs`, `frame.rs`, `mm/phys/frame.spec.rs`
  → **0 contract drift** on all four; ensures removed 0, requires added 0.
- `./z build -- all` → `[OK] Build complete.`
- `grep admit|external_body|trusted|assume(` on changed arch files → NONE.

The in-scope `from_raw_value` / `into_raw_value` ensures are unchanged and still reference
`self@` / `spec_max()`; their guarantees were not weakened by the `spec_max()` reinterpretation
(the bound is now concrete `usize::MAX/4096 - 1`, strictly stronger than the former `uninterp nat`).

### Fix Request

None. Item 7 is **FIXED** and all 18 checklist items are PASS/FIXED with tool-backed evidence.
Writing `STOP = RESOLVED`.
