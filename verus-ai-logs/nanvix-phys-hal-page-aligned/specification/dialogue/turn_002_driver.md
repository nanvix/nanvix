## Turn 2: No cheating on module's own functions (verify fix) — FIXED; full checklist RESOLVED

### Progress
- Done (PASS/FIXED): every checklist item. See verdicts below.
- Current: verified the Turn-1 FAIL fix (`admit` in `from_address`).
- Remaining: none.

### Verification of the Turn-1 fix

The fixer removed `proof! { admit(); }` from `from_address` and supplied the missing
dependency specs. I verified each claim independently:

**1. `admit` actually gone from the module.**
```
$ grep -nE 'admit|assume|external_body|trusted' page.rs page.spec.rs page.proof.rs
  page.spec.rs:7: pub assume_specification[ ::arch::mem::PAGE_ALIGNMENT ] ...   # constant model only
$ grep -i aligned/page verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt
  (no match, exit 1)
```
`from_address: admit` is gone. `from_address`'s `ensures` is **unchanged** (confirmed by
reading page.rs:42-56) — still `Ok(r) => spec_aligned(addr@) && r@ == addr@ && r.inv()`,
`Err(_) => !spec_aligned(addr@)`.

**2. The new dependency specs are real and faithful, not new trust surface.**
- `sys/mm/alignment.rs:156` — `pub open spec fn spec_align_value(Alignment) -> int`: a
  concrete `match` mapping each variant to its byte value (`Align4096 => 4096`). Concrete
  definition, not `uninterp`.
- `sys/mm/address/mod.rs:102` — `Address::is_aligned` now has
  `ensures result matches Ok(aligned) && aligned == (self@ % spec_align_value(align) == 0)`.
- `page.spec.rs:7-10` — `PAGE_ALIGNMENT` model ensures
  `spec_align_value(result) == spec_page_size()` (the page-alignment↔page-size link).

**3. The "always `Ok`" strengthening on `is_aligned` is faithful** (it's a trusted trait
contract no impl re-proves, so I checked every concrete impl returns `Ok`):
```
phys.rs:210        Ok(self.0.is_aligned(align))           # always Ok
virt.rs:221        Ok(self.is_aligned(align))             # always Ok
page.rs:136        self.0.is_aligned(align)               # delegates -> Ok
pgtab.rs:112       self.0.is_aligned(align)               # delegates -> Ok
```
And the value is faithful: `VirtualAddress::is_aligned` computes
`value & (align-1) == 0`, which equals `value % align == 0` because every `Alignment`
variant is a power of two — matching `spec_align_value`. No `Address` impl is
`#[verus_verify]` (only `external_derive` on the struct), so no impl is forced to re-prove,
and adding facts to a trait method cannot break existing caller proofs.

**4. `from_address` now verifies WITHOUT `admit` (fresh, non-cached run).**
Commit `0827f3f53`: `kernel::all` → **50 verified, 0 errors, admit=24** (fresh, not cached).
My own re-run:
```
$ make verify-kernel
  Exit code : 0
  cheating: assume=0 external_body=14 admit=24 ...   # was 25; the from_address admit is the one removed
```

**5. No specs weakened** (`spec_drift.py git-diff --before verus-ai/phys-frame --after HEAD`):
- `page.rs`: 0 contract drift (2 fns added, 0 removed).
- `sys/mm/address/mod.rs`: 0 contract drift. All changes additive/strengthening.

**6. Cross-module regression** — I ran the full `make verify` (the `is_aligned` trait spec
lives in `sys`, a dependency of everything):
```
bitmap  : Exit 0
sys     : Exit 0
nanvix-slab : Exit 0
bump-allocator : Exit 0
kernel  : Exit 0
```

**7. Normal build** — `./z build -- check-kernel` → `build-finished success:true`, `[OK] Build complete.`
The new `spec_align_value` (`#[cfg(verus_keep_ghost)]`) and `is_aligned` `#[verus_spec]`
are erased in normal builds; compilation unaffected.

### Final checklist verdicts (all PASS/FIXED)

| # | Item | Verdict | Evidence |
|---|------|---------|----------|
| 1 | fn_coverage | PASS | 17/17 matched; `from_address` has spec; `into_raw_value` via trait |
| 2 | Caller coverage | PASS | matches `caller_analysis.md` |
| 3 | View consistency | PASS | `self@`/`inv()`, View=`int` per `view_design.md` |
| 4 | No tautological ensures | PASS | `Err => !spec_aligned`; `is_aligned` spec is non-trivial |
| 5 | No subsumed ensures | PASS (note) | `r.inv()` derivable but kept for caller usability |
| 6 | Meaningful error paths | PASS | match Ok/Err style |
| 7 | No assume_spec for ws-internal | PASS (note) | `PAGE_ALIGNMENT` is an external `arch` **const** that cannot carry `#[verus_spec]`; minimal trusted constant model (temporarily allowed) |
| 8 | vstd searched | PASS | no vstd alignment constant |
| 9 | Specs written for caller | PASS | `r.inv()` exposed directly |
| 10 | Trait obligations | PASS | `into_raw_value` contract on trait decl; `is_aligned` faithful |
| 11 | Spec completeness (advisory) | PASS | address value + alignment invariant covered |
| 12 | Loop invariants | PASS | no loops |
| 13 | No cheating own functions | **FIXED** | `admit` removed; verifies fresh (50 verified, 0 errors) |
| 14 | No specs weakened | PASS | drift = 0 on both files; all additive |
| 15 | Bug awareness | PASS | `bugs.md` VERUS-TOOL-1 recorded |
| 16 | Cross-module regression | PASS | all 5 crates exit 0 |
| 17 | Verification + build | PASS | `make verify` exit 0; `check-kernel` build success |

### Fix Request
None — all items PASS/FIXED. Creating `STOP` = `RESOLVED`.

Note on whole-crate `status: CHEATING_DETECTED`: the remaining `admit=24` / `external_body=14`
are **pre-existing placeholders** in `mm/phys/*` and the `FrameAddress` shims — none in this
module's own functions (`grep aligned/page cheating-detail.txt` → empty). Those belong to
other modules' verification scopes and are out of scope for `hal-page-aligned`.
