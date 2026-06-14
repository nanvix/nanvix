## Turn 2: No cheating on module's own functions (re-verify) — FIXED

### Progress
- Done (PASS in turn 1, unchanged): every-exec-has-contracts, caller-coverage,
  view-consistency, no-tautological-ensures, no-subsumed-ensures (advisory),
  error-path-ensures (N/A), no-assume_specification, vstd-searched (N/A),
  specs-for-caller, trait-obligations, spec-completeness (advisory),
  loop-invariants (N/A), bug-awareness, no-specs-weakened, cross-module-regression,
  verification+build.
- Current (was the only FAIL): **No cheating on module's own functions** — now FIXED.
- Remaining: none. Every checklist item is PASS or FIXED.

### Verification

I did **not** trust the fixer's report — I re-ran every tool and read the actual code.

**1. The `admit()` is gone from the source (read the file, not the report).**
`src/libs/arch/src/x86/mem/paging/pde.rs:414–418`:
```rust
pub fn frame_address(&self) -> usize {
    let raw: usize = self.frame.into_raw_value();
    proof! { lemma_frame_address(raw); }
    raw << crate::mem::FRAME_SHIFT
}
```
`grep -rn "admit|assume|external_body|trusted"` over pde.rs / pde.spec.rs /
pde.proof.rs → **NONE**.

**2. The replacement is a real proof, not another cheat.**
`pde.proof.rs` (was empty `verus! { }`) now contains `proof fn lemma_frame_address`
built entirely from vstd lemmas (`lemma2_to64`, `lemma_usize_shl_is_mul`,
`lemma_mod_bound`, `lemma_fundamental_div_mod`, `lemma_mul_inequality`,
`lemma_mod_multiples_basic`) plus one `by (nonlinear_arith)` overflow step. No
`admit`/`assume`/`external_body`/`trusted` anywhere in it.

**3. Forced clean re-verification (the turn-1 run was cached — I distrusted it).**
`touch pde.proof.rs pde.rs && make verify-arch`:
```
verification results:: 41 verified, 0 errors
  Exit code : 0
  cheating: assume=0 external_body=2 admit=1 trusted=0 no_decreases=0 cfg_gate=2
```
The proof genuinely verifies on a real recompile — not a stale cache. `admit`
count dropped 2 → 1.

**4. The remaining `admit`/`external_body` are NOT this module.**
`cheating-detail.txt`:
```
- x86/mem/paging/table.proof.rs:8 lemma_entry_roundtrip: admit
- x86/mem/paging/table.rs:209 read: external_body
- x86/mem/paging/table.rs:246 write: external_body
```
`grep -i pde cheating-detail.txt` → **NO PDE CHEATS**. The three remaining entries
all belong to the `table` module (out of scope for arch-x86-pde, flagged only by
the global counter). The module-level status `CHEATING_DETECTED` is driven solely
by those out-of-scope `table` entries.

**5. No spec weakened (re-checked).** `frame_address`'s `ensures` (pde.rs:409–413)
is byte-identical to turn 1:
```
result as int == self@.frame * (crate::mem::FRAME_SIZE as int),
result as int % (crate::mem::FRAME_SIZE as int) == 0,
```
The fix added a *proof*; it did not touch any `ensures`/`requires`/spec-fn
contract. So caller invariants 2 and 4 still hold at full strength, and downstream
kernel consumers see the identical contract (no regression).

**6. Build still passes.** `./z build` → exit 0, `[OK] Build complete.`

**7. Cross-module regression — PASS.** `make verify-arch` verifies all arch
modules: 41 verified, 0 errors, exit 0.

### Fix Request
None. The sole turn-1 FAIL (the `admit()` in `frame_address`) is FIXED and
independently verified:
- admit removed from pde.rs (grep: no matches),
- replaced by a genuinely-verifying vstd-based `lemma_frame_address` in pde.proof.rs,
- forced recompile → 41 verified / 0 errors,
- `ensures` unchanged (no weakening),
- build green.

All 18 checklist items are PASS or FIXED. Marking the specification phase RESOLVED.
