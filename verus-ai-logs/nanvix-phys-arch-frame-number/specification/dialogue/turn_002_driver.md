## Turn 2: Verifying fixer turn 1 — full checklist re-sweep → RESOLVED

### Progress
- Done (PASS/FIXED): items 1–17 (all).
- Current: final verification of fixer's changes (compile fix + contracts + kernel migration).
- Remaining: none.

### Verification

**Diff reviewed** (`git diff 33c9db43..HEAD -- src/`): 6 source files changed —
`arch/.../number.rs`, `number.spec.rs`, `arch/src/x86/mem/constants.rs`,
`arch/Cargo.toml`, and kernel `frame.spec.rs` / `phys.spec.rs`. Changes are
committed in `755f8a542 [verus] verify PASS: arch::all (6 verified, 0 errors)`.

**Commands run by me (not trusting fixer's claims):**

1. `make verify-arch` →
   ```
   Exit code : 0
   cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
   coverage: 2/525 exec functions have contracts
   status: CLEAN
   ```
2. `make verify` (all crates incl. kernel) → exit 0. `kernel::all` exit 0
   (`external_body=23 cfg_gate=6` — see below, pre-existing). All other crates exit 0.
3. `./z build -- all` → `[OK] Build complete.` exit 0.

### Item-by-item

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Every in-scope exec fn has requires/ensures | **FIXED** | `into_raw_value` (number.rs:55) and `from_raw_value` (number.rs:88) now carry `#[verus_spec]`; `make verify-arch` 6 verified, 0 errors; E0252 compile error gone |
| 2 | Caller coverage | **FIXED** | Some-iff-`value<=max`, index preservation `f@==value`, `result as int==self@`, range, round-trip — all present and match `caller_analysis.md` |
| 3 | View consistency | **FIXED** | Specs reference `self@`; `inv()` is a `#[verifier::type_invariant]` discharged via `use_type_invariant`; matches `view_design.md` |
| 4 | No tautological ensures | **PASS** | `from_raw_value` None branch carried by `(Some <==> value<=max)`; no `=> true` |
| 5 | No subsumed ensures | **PASS** | `into_raw_value`'s `0<=result<=max` is intentionally retained — it was part of the kernel's prior assumed contract; removing it would *weaken* vs tcb-allowed (item 14). Keeping it makes the spec directly usable by callers (item 9) |
| 6 | Error paths meaningful | **FIXED** | `result is None <==> value as int > spec_max_frame_number()` (contrapositive of the `<==>`) |
| 7 | No assume_specification for workspace-internal | **PASS** | arch number module has none; kernel migration *removed* 4 arch-related `assume_specification`s |
| 8 | vstd searched before assume_specification | **PASS (N/A)** | none in this module |
| 9 | Specs written for caller | **FIXED** | Identity + bidirectional condition + `f.inv()` directly usable in `phys.rs`/`pde.rs`/`pte.rs`/`frame.rs` proofs |
| 10 | Trait obligations | **PASS** | only derived Debug/Clone/Copy |
| 11 | Spec completeness (advisory) | **PASS** | both functions fully specify behavior; no unintended nondeterminism |
| 12 | Loop invariants | **PASS (N/A)** | no loops |
| 13 | No cheating on module's own fns | **PASS** | arch: `assume=0 external_body=0 admit=0 trusted=0`. `#[verus_verify(external_derive)]` on the struct is the derive-macro shim, not function cheating |
| 14 | No specs weakened | **PASS** | Compared removed kernel `assume_specification`s vs new arch-native specs: `into_raw_value` equivalent (`spec_frame_raw_value(f)` redefined to `f@`); `from_raw_value` equal-or-stronger (adds `f.inv()`). `uninterp`→`open` redefinitions are strengthenings. Kernel still verifies (exit 0), so the bridge `arch::spec_max_frame_number == kernel::spec_max_frame_number` is discharged |
| 15 | Bug awareness | **PASS** | exec bodies trivially correct; no bugs_file needed |
| 16 | Cross-module regression | **PASS** | full `make verify` exit 0; `kernel::all` exit 0 (47/47 modules) after the `assume_specification` removals |
| 17 | Verification + build | **PASS** | `make verify` exit 0; `./z build -- all` exit 0 |

### Kernel cheating-count check (fixer's claim audited)
`git diff 33c9db43..HEAD -- src/kernel/` added-lines with cheating keywords are
**all comments**; removed-lines are real constructs:
`assume_specification[ ::arch::mem::PAGE_SIZE ]`, `[ ::arch::mem::FRAME_SIZE ]`,
`[ FrameNumber::into_raw_value ]`, `[ FrameNumber::from_raw_value ]`,
`#[verifier::external_type_specification]` + its `#[verifier::external_body]`
(the `ExFrameNumber` shim). So kernel's `external_body=23 cfg_gate=6` is
**pre-existing TCB**, not introduced here — the migration only *removed* trust.

### Fix Request
None. All 17 checklist items are PASS or FIXED with tool-verified evidence.
