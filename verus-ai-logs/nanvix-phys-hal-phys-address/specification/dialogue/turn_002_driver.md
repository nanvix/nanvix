## Turn 2: Re-verification of all three FAIL items + full checklist

### Progress
- **Done (PASS/FIXED):** ALL 17 checklist items.
- **Current:** Final re-verification of FIX-1 (admit removal), FIX-2 (`from_number` caller-usable
  contract), FIX-3 (tautological error arm) — all confirmed FIXED.
- **Remaining:** none.

### Verification (commands run, independent of fixer claims)

1. **`make verify-kernel` (full crate)** → exit 0.
   `cheating: assume=0 external_body=25 admit=0 trusted=0 cfg_gate=10`.
   **`admit` dropped 2 → 0.** `grep` of `cheating-detail.txt` for `address/phys.rs` → **no matches**
   (no admit, no external_body on any in-scope phys function).

2. **`make verify-kernel MODULE=hal::mem::types::address::phys` (forced fresh, `touch`ed source, NOT
   cached)** → `4 verified, 0 errors`, `admit=0 trusted=0`. Real proofs, not vacuous.

3. **`spec_drift.py git-diff phys.rs --before <view-design-end> --after HEAD`** → 0 ensures removed,
   0 requires removed; only additions. **No original guarantee weakened.**

4. **`./z build`** → `[OK] Build complete.` (erased/non-Verus dual compilation succeeds).

5. **Grounding of the fixer's `spec_max_frame_number()` interpretation verified against arch source:**
   - `src/libs/arch/.../frame/number.rs:29` → `FrameNumber::MAX = MAX_ADDRESS / FRAME_SIZE - 1`.
   - `constants.rs:90` → `MAX_ADDRESS = usize::MAX`; `:104` `FRAME_SIZE = PAGE_SIZE`; `:97`
     `FRAME_SHIFT = PAGE_SHIFT`; `:40/:47` `PAGE_SHIFT=12`, `PAGE_SIZE=4096` (`assert PAGE_SIZE==1<<PAGE_SHIFT`).
   - So `spec_max_frame_number() == usize::MAX/spec_page_size() - 1`, `pow2(FRAME_SHIFT)==FRAME_SIZE`,
     and `spec_page_size()>0` are all **TRUE arch facts**, not invented assumptions. The
     `from_raw_value`/`into_raw_value` assume_specs match `number.rs` behavior exactly.
   - `spec_max_frame_number` is **module-local** (grep across kernel: no other user) → no cross-module
     coupling introduced by interpreting it.

### Item-by-item result

| # | Item | Verdict |
|---|------|---------|
| 1 | In-scope exec fns have requires/ensures | PASS — `from_mmio_address`, `from_number`, `into_frame_number` all annotated; none in `coverage-unverified.txt`. |
| 2 | Caller coverage | **FIXED** — `from_number` now ensures `result@ % spec_page_size()==0` (for `PageAligned::from_address`) and `result.inv()` (for later `into_frame_number`). |
| 3 | View consistency / maintains `inv()` | **FIXED** — `from_number` now establishes `result.inv()`; all specs reference `self@`/`inv()`/`spec_frame_number`. |
| 4 | No tautological ensures | **FIXED** — `Err(_) => true` removed; replaced by `result is Ok`. |
| 5 | No subsumed ensures | PASS — `from_number`'s alignment is NOT derivable from `inv()` (inv is only an upper bound), and both alignment & `inv()` are load-bearing for the caller; not trivially subsumed. |
| 6 | Error paths meaningful | **FIXED** — `from_mmio_address` (infallible body) now guarantees `result is Ok` instead of an unconstrained `Err`. |
| 7 | No assume_spec for internal code | PASS — all target `::arch`/`::sys` (external crates). |
| 8 | vstd searched first | PASS — proofs use vstd `power2::pow2`, `div_mod` lemmas, `bits::lemma_usize_shr_is_div`. |
| 9 | Specs usable by caller | **FIXED** — alignment + `inv()` now directly available; no caller-side re-derivation of div/mod needed. |
| 10 | Trait obligations | PASS — in-scope fns inherent; trait methods out of scope/untouched. |
| 11 | Spec completeness (advisory) | PASS — all three contracts complete. |
| 12 | Loop invariants | PASS (N/A — no loops). |
| 13 | No cheating on own functions | **FIXED** — `admit=0` (fresh module run); no `external_body`/`trusted`/`assume` on in-scope phys functions. `ExFrameNumber` external_type_spec is the standard external (arch) type registration. |
| 14 | No specs weakened | PASS — `spec_drift` additions-only; `spec_max_frame_number` interpreted to its TRUE arch value (strengthening, grounded in source). |
| 15 | Bug awareness | PASS — logic sound; nothing to record. |
| 16 | Cross-module regression | PASS — `make verify-kernel` (all modules) exit 0. |
| 17 | Verification + build | PASS — module: 4 verified / 0 errors / admit=0; `./z build` OK. |

### Fix Request
None. All previously-failing items (admit-on-own-functions, tautological/meaningless error ensures,
unusable `from_number` contract) are FIXED with real proofs, verified by fresh tool runs. The three
in-scope functions verify with `admit=0` and `0 errors`, the full crate has no regressions, the erased
build succeeds, and no spec was weakened. Spec phase is complete.

### Decision
**RESOLVED.** STOP file created.
