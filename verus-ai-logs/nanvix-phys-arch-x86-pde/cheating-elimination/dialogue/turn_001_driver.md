## Turn 1: Exec rewrites must carry VERUS REWRITE comment + minimal reproducer (`frame_address`)

### Progress
- Done (PASS):
  1. Zero `admit()` — PASS
  2. Zero `assume()` — PASS
  3. Zero trusted functions — PASS
  4. Zero `exec_allows_no_decreases_clause` — PASS
  5. Zero cfg-gated exec code (pde module) — PASS
  6. Zero unlisted `external_body` (pde module) — PASS
  9. Each surviving `external_body` TCB-listed — PASS (none in pde)
  10. No specs weakened (spec-drift) — PASS
  11. Cross-module regression (`make verify-arch`, whole crate) — PASS
  12. Verification + build (0 errors / 0 warnings) — PASS
- Current (FAIL): item 8 — "All exec rewrites have VERUS REWRITE comment and minimal reproducer"
  (and the coupled AST-consistency item 7).
- Remaining: none — item 8 is the only blocker.

### Verification

Ran `make verify-arch`:
```
Exit code : 0
cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=2
```
`verus-logs/cheating-detail.txt` shows every flagged construct is OUTSIDE the pde module:
```
x86/mem/paging/mod.rs:80   invlpg            : external_body   (TCB-listed)
x86/mem/paging/table.proof.rs:16 lemma_entry_roundtrip : assume (TCB-listed)
x86/mem/paging/table.rs:209 read             : external_body   (TCB-listed)
x86/mem/paging/table.rs:246 write            : external_body   (TCB-listed)
```
- `cfg_gate=2`: the counter (`scripts/verify.sh:478` `count_cfg_gates`) explicitly skips
  `include!`/`use` targets. The two counted gates are `pte.rs:85` and `pte.rs:307`
  (`#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]`) — NOT pde.
  pde.rs lines 9/11 are `#[cfg(verus_keep_ghost)] include!(...)` → skipped → pde cfg_gate = 0. PASS.
- `external_body=3` / `assume=1`: none in pde.rs/pde.spec.rs/pde.proof.rs; all four are in
  `mod.rs`/`table.rs`/`table.proof.rs` and all four appear in `verus-ai-logs/tcb-allowed.md`. PASS.
- admit / trusted / no_decreases = 0. PASS.

Build check (same flags Verus uses), clean:
```
cargo build -p arch ... --target build/targets/x86-kernel.json
    Finished `dev` profile ... in 0.62s   (0 errors, 0 warnings)
```

Spec-drift (item 10): `git diff 1b6d97508 HEAD -- pde.rs`. All changes are additive ghost
annotations (`#[verus_verify]`, `#[verus_spec]`, `proof!{...}`), import additions, and a new
`impl TableEntry for PageDirectoryEntry` delegating to existing `from_raw_value`/`into_raw_value`.
No `ensures` removed/weakened; `frame_address` even GAINED a contract. PASS.

**The one failure — item 8.** The only exec-body change in the diff is `frame_address`
(pde.rs:420-424):
```rust
pub fn frame_address(&self) -> usize {
    let raw: usize = self.frame.into_raw_value();
    proof! { lemma_frame_address(raw); }
    raw << crate::mem::FRAME_SHIFT
}
```
Original exec was a single expression:
```rust
self.frame.into_raw_value() << crate::mem::FRAME_SHIFT
```
Stripping the ghost `proof!` line, the exec AST still differs from the original: an intermediate
`let raw` binding was introduced. This is a **semantically-equivalent exec rewrite forced by a
Verus limitation** — `into_raw_value()`'s bound postcondition (`0 <= self@ <= spec_max()`) must
land in context *before* the overflow-bearing shift so `lemma_frame_address(raw)` can be invoked
between the call and the shift; the operand must be named because exec calls cannot appear inside
`proof!`.

This is the EXACT pattern that, elsewhere in this same repository, is required to carry a
`// VERUS REWRITE:` comment plus a minimal reproducer:
`src/kernel/src/hal/mem/types/address/phys.rs:142-148` (`from_number`,
`frame.into_raw_value() * FRAME_SIZE` split identically) — comment + reproducer
`verus-ai-logs/nanvix-phys-hal-phys-address/cheating-elimination/repro/from_number.rs`.

`pde.rs::frame_address` has **neither** the comment nor a reproducer
(`grep -rn "VERUS REWRITE" pde.rs` → no matches). This FAILS item 8, and the undocumented
AST-shape change is exactly what item 7 (AST consistency) requires to be a documented
semantically-equivalent rewrite. Justification is not enough — the codebase convention demands the
in-source comment and the runnable reproducer.

### Fix Request

1. Add a `// VERUS REWRITE:` comment immediately above `pub fn frame_address` in
   `src/libs/arch/src/x86/mem/paging/pde.rs`, mirroring `phys.rs:142-148`. It must state:
   - original exec was `self.frame.into_raw_value() << crate::mem::FRAME_SHIFT` (single expression);
   - the `let raw` binding is mandatory so `into_raw_value()`'s bound postcondition
     (`0 <= self@ <= FrameNumber::spec_max()`) is in scope before the shift, allowing
     `lemma_frame_address(raw)` to discharge the no-overflow + alignment `ensures` between the call
     and the shift (an exec call cannot live inside `proof!`, so the operand must be named);
   - same value, same operations — semantically equivalent;
   - a `Reproducer:` path line pointing at the file in step 2.

2. Create the minimal reproducer at
   `verus-ai-logs/nanvix-phys-arch-x86-pde/cheating-elimination/repro/frame_address.rs`,
   following the format of
   `verus-ai-logs/nanvix-phys-hal-phys-address/cheating-elimination/repro/from_number.rs`:
   a `bad` function (single-expression `raw << SHIFT`, no intermediate binding) that FAILS Verus,
   and a `good` function (with the `let raw` binding + lemma) that PASSES. Include the `verus ...`
   run commands in the header comment, and show it actually exercises the shift/lemma path.

3. Verify:
   - `grep -rn "VERUS REWRITE" src/libs/arch/src/x86/mem/paging/pde.rs` → returns the new comment.
   - Run the reproducer both ways and paste the FAIL/PASS output proving the rewrite is necessary.
   - `make verify-arch` → still exit 0, cheating counts unchanged
     (`assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=2`).

Do NOT change the exec semantics of `frame_address`; only add the comment and the reproducer file.
