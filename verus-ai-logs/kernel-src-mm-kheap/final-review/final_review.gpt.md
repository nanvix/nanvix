# Final Review: kheap (GPT-5.3-Codex)

## 1. Spec Quality
Overall: **mixed** (good local method specs, but incomplete module/API coverage).

- **Strong points**
  - `layout_to_allocator`, `allocate`, `deallocate` specs are declarative and caller-usable (`src/kernel/src/mm/kheap.rs:266-401`).
  - Error paths for `allocate`/`deallocate` include **state preservation** (`self@ == old(self)@`) (`kheap.rs:287,336`).
  - Uses `View` abstraction (`KheapView`) cleanly (`kheap.spec.rs:191-287`), with explicit state-transition specs (`spec_allocate`/`spec_deallocate`).

- **Gaps / issues**
  1. **One-sided / incomplete error contract in constructor**: `from_raw_parts` only guarantees `Err => e.code == InvalidArgument`; it does not encode full bidirectional failure condition FN-2g in contract (`kheap.rs:147-150`).
  2. **Property mismatch vs declared analysis**: TYPE-4/5/6 are claimed in analysis but not explicitly represented in `KheapView::inv()` (only TYPE-1/2/3 present) (`kheap.spec.rs:197-210`).
  3. **External-top API coverage missing for wrappers**: `GlobalAlloc::alloc`, `GlobalAlloc::dealloc`, and `init` are unverified/no contracts in Verus summary (only 4/7 exec functions contracted).
  4. **Caller-facing completeness gap**: FN-5/FN-6/FN-7, GLOBAL-*, BUG-* properties are mostly documented in `property_analysis.md` but not captured in spec/proof files.

- **Anti-pattern check**
  - No tautological postconditions found in core verified methods.
  - No `admit()`/escape in these files.
  - Main anti-pattern observed: **spec-documentation drift** (properties declared but not encoded in spec/proof files).

## 2. Property Coverage
- Covered: **11 / 59**
- Missing: **48 IDs**

Coverage check was done against **`kheap.spec.rs` + `kheap.proof.rs` only**.

- **Covered IDs**: `TYPE-1`, `TYPE-2`, `TYPE-3`, `FN-1c`, `FN-3d`, `FN-4c`, `MOD-1`, `MOD-2`, `MOD-3`, `MOD-5`, `LIVE-5`

- **Missing IDs**:
  - TYPE: `TYPE-4`, `TYPE-5`, `TYPE-6`
  - FN: `FN-1a`, `FN-1b`, `FN-1d`, `FN-2b`, `FN-2c`, `FN-2d`, `FN-2e`, `FN-2f`, `FN-2g`, `FN-3b`, `FN-3c`, `FN-3e`, `FN-3f`, `FN-3g`, `FN-4b`, `FN-4d`, `FN-4e`, `FN-4f`, `FN-5a`, `FN-5b`, `FN-5c`, `FN-6a`, `FN-6b`, `FN-6c`, `FN-7b`, `FN-7c`, `FN-7d`
  - MOD: `MOD-4`, `MOD-6`, `MOD-7`
  - LIVE: `LIVE-1`, `LIVE-2`, `LIVE-3`, `LIVE-4`, `LIVE-6`
  - GLOBAL: `GLOBAL-1`, `GLOBAL-2`, `GLOBAL-3`, `GLOBAL-4`, `GLOBAL-5`
  - BUG: `BUG-1`, `BUG-2`, `BUG-3`, `BUG-4`, `BUG-5`

Property-by-property status:
- TYPE-1 ✅ TYPE-2 ✅ TYPE-3 ✅ TYPE-4 ❌ TYPE-5 ❌ TYPE-6 ❌
- FN-1a ❌ FN-1b ❌ FN-1c ✅ FN-1d ❌
- FN-2b ❌ FN-2c ❌ FN-2d ❌ FN-2e ❌ FN-2f ❌ FN-2g ❌
- FN-3b ❌ FN-3c ❌ FN-3d ✅ FN-3e ❌ FN-3f ❌ FN-3g ❌
- FN-4b ❌ FN-4c ✅ FN-4d ❌ FN-4e ❌ FN-4f ❌
- FN-5a ❌ FN-5b ❌ FN-5c ❌
- FN-6a ❌ FN-6b ❌ FN-6c ❌
- FN-7b ❌ FN-7c ❌ FN-7d ❌
- MOD-1 ✅ MOD-2 ✅ MOD-3 ✅ MOD-4 ❌ MOD-5 ✅ MOD-6 ❌ MOD-7 ❌
- LIVE-1 ❌ LIVE-2 ❌ LIVE-3 ❌ LIVE-4 ❌ LIVE-5 ✅ LIVE-6 ❌
- GLOBAL-1 ❌ GLOBAL-2 ❌ GLOBAL-3 ❌ GLOBAL-4 ❌ GLOBAL-5 ❌
- BUG-1 ❌ BUG-2 ❌ BUG-3 ❌ BUG-4 ❌ BUG-5 ❌

## 3. Proof Completeness
- admit() count: **0**
- Locations: **none** (searched in `kheap.rs`, `kheap.spec.rs`, `kheap.proof.rs`)
- Assessment: No unresolved `admit()` holes in the reviewed files.

## 4. Trust Boundary Audit
- assume_specification count: **2**
- axiom count: **0**

Assumptions found:
1. `assume_specification[Layout::size]` (`kheap.spec.rs:83-85`)
   - Assumes: `Layout::size()` returns unconstrained uninterpreted value `spec_layout_size(layout)`.
   - Human-approved: **Yes** (`Needed Assumptions` has `Layout::size` marked `[x]`).
   - Minimal/correctness: Minimal accessor spec; good as a pure bridge.

2. `assume_specification[Error::new]` (`kheap.spec.rs:88-91`)
   - Assumes: constructed error preserves `code` field (`result.code == code`).
   - Human-approved: **Yes** (`Needed Assumptions` has `Error::new` marked `[x]`).
   - Minimal/correctness: Minimal and appropriate for current proofs; does not overconstrain `reason`.

Axioms:
- No explicit `axiom` declarations in `kheap.spec.rs` or `kheap.proof.rs`.

**BLOCKER status**: No unapproved `assume_specification`/`axiom` items found.

## 5. Exec Fidelity
- AST check result: **PASS (with documented deviations)**

Command run:
- `python3 /home/ruize/verus-ai-improve3/scripts/ast_consistency.py src/kernel/src/mm/kheap.rs summary`

Result summary:
- Functions: 3 MATCH, 4 MISMATCH (`layout_to_allocator`, `from_raw_parts`, `allocate`, `deallocate`)
- Structs: all MATCH

Assessment:
- All mismatches are documented in `exec-fidelity/fix_report.md` as approved/justified deviations (named returns, cfg-gated Verus-only compatibility changes, documented Verus closure-parameter limitation). No unexplained mismatch found.

## 6. Verification
- Result: **FAIL** (for requested command)

Command run:
- `make verify-kernel MODULE=kheap`

Output summary:
- Error: `could not find module kheap specified by --verify-module or --verify-only-module`
- Available module shown by tool: `mm::kheap`
- Exit code: 101 (make failed)

Additional check (not requested but performed):
- `make verify-kernel MODULE=mm::kheap` → **19 verified, 0 errors**, but cheating report flags `external_body` usage.

## Overall Assessment
- Grade: **C**
- Key issues:
  - Major property-coverage gap in spec/proof files (**11/59** only by ID evidence).
  - Constructor and wrapper-level API contract completeness is weaker than property_analysis claims.
  - Requested verification command currently fails due module selector mismatch (`kheap` vs `mm::kheap`).
