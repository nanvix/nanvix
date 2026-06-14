## Turn 2: Zero assume/assume_specification — FIXED → RESOLVED

### Progress
- Done (PASS/FIXED):
  - No specs weakened — PASS
  - Zero remaining `admit()` — PASS
  - Zero `external_body` (target module) — PASS
  - **Zero assume/assume_specification — FIXED this turn**
  - No cfg-gated exec code — PASS
  - Cheating audit (counts + locations) — PASS
  - Verus-limitation isolated reproducer — PASS (N/A)
  - Exec rewrites minimal / `// VERUS REWRITE` — PASS (none)
  - Cross-module regression — PASS
  - Verification + build (0 errors, 0 warnings) — PASS
- Current: Zero assume/assume_specification (the only Turn-1 FAIL)
- Remaining: none — all checklist items verified.

### Verification

**Fixer's claim:** took the Fallback (documenting the intra-crate placeholder in
`tcb-allowed.md`), justified because `phys.rs` is out of editable scope ("Do not touch
unlisted functions"). Turn-1 review explicitly permitted this fallback.

I verified every claim independently:

1. **The `assume_specification` is now allow-listed.**
   `grep -n "frame.spec.rs::<PhysicalAddress" verus-ai-logs/tcb-allowed.md` →
   hit at line 156. Read `tcb-allowed.md:154-168`: a new bullet group
   *"Intra-crate placeholder in `hal::mem::types::address::frame.spec.rs`"* records
   `src/kernel/src/hal/mem/types/address/frame.spec.rs::<PhysicalAddress as
   ::sys::mm::Address>::from_raw_value`, states the exact contract
   (`Ok(pa) => pa@ == value as int`, `Err(_) => true`), confirms `phys.rs:185` carries
   **no** `#[verus_spec]`, explains why it is genuinely required for bottom-up
   verification (the verified `FrameAddress::from_raw_value` postcondition depends on it),
   and gives the "removed/superseded when `phys` is verified" rationale matching the
   existing intra-crate placeholders. **Accurate and committed** (`git status` clean;
   committed in `d5a720d3d`).

2. **The contract is sound and not weakened.** `frame.spec.rs:20-27` matches the
   documented contract exactly; `phys.rs:185` indeed has no `#[verus_spec]` (re-read
   `phys.rs:183-187`). This is the standard external-bottom/bottom-up placeholder
   convention, now properly recorded as the HARD-RULE allow-list requires.

3. **`make verify-kernel MODULE=hal::mem::types::address::frame`** → **exit 0**,
   `6 verified, 0 errors`. Module-scoped cheating: only `cfg-gated exec code: 1` (the
   ghost-only `verus!` block — PASS as established Turn 1).

4. **`make verify-kernel`** (all modules) → **exit 0**;
   `cheating: assume=0 external_body=11 admit=29 cfg_gate=15` — every entry remains in
   `mm/phys`, `mm/virt`, sibling `phys.proof.rs` (all `tcb-allowed.md`-listed or
   out-of-scope). No regression. Target module: admit=0, external_body=0, assume()=0.

5. **`./z build -- all-kernel`** → **Build complete, 0 errors, 0 warnings.**

#### Final cheating audit (target module: frame.rs / frame.spec.rs / frame.proof.rs)
- `admit` = **0**
- `external_body` = **0**
- `assume(...)` = **0**
- `assume_specification` = **1** (`frame.spec.rs:20`) — now **documented** in
  `tcb-allowed.md:156` (allow-listed). HARD RULE satisfied.
- cfg-gated exec code = **0** (the 1 `cfg_gate` at `frame.rs:36` is a ghost-only
  `verus!` block: `spec_page_size`, `View::view`, `inv` — all spec fns; no exec
  branch/expr/match arm gated).

### Fix Request
None — item FIXED. All checklist items are PASS or FIXED with tool-verified evidence.

### Result: RESOLVED
