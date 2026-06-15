## Turn 2: Exec rewrites have VERUS REWRITE comment + minimal reproducer (`frame_address`) — FIXED

### Progress
- Done (PASS/FIXED):
  1. Zero `admit()` — PASS
  2. Zero `assume()` (pde) — PASS
  3. Zero trusted functions — PASS
  4. Zero `exec_allows_no_decreases_clause` — PASS
  5. Zero cfg-gated exec code (pde) — PASS
  6. Zero unlisted `external_body` (pde) — PASS
  7. AST consistency — PASS (only rewrite is the now-documented `frame_address`)
  8. All exec rewrites have VERUS REWRITE comment + reproducer — **FIXED this turn**
  9. Each surviving `external_body` TCB-listed — PASS (none in pde)
  10. No specs weakened — PASS
  11. Cross-module regression — PASS
  12. Verification + build (0 errors / 0 warnings) — PASS
- Current: item 8 (the only prior FAIL) — verified FIXED.
- Remaining: none.

### Verification

I did NOT trust the fixer's claims — I re-ran every check myself.

**1. VERUS REWRITE comment present** (pde.rs:420-428), placed after the `#[verus_spec]`,
immediately above `pub fn frame_address`. It documents: original single-expression form
`self.frame.into_raw_value() << crate::mem::FRAME_SHIFT`; the `let raw` binding is mandatory so
`into_raw_value()`'s bound postcondition lands in scope before the overflow-bearing shift, letting
`lemma_frame_address(raw)` discharge the no-overflow + alignment `ensures`; "same value, same
operations — semantically equivalent"; and a `Reproducer:` path line. Format matches the codebase
convention at `phys.rs:142-148`.
```
$ grep -n "VERUS REWRITE" src/libs/arch/src/x86/mem/paging/pde.rs
420:    // VERUS REWRITE: the original `self.frame.into_raw_value() << crate::mem::FRAME_SHIFT`
```
Exec body unchanged from turn 1 (still `let raw` + `proof!{ lemma_frame_address(raw); }` + shift) —
no semantics touched, only the comment added. Confirmed by reading pde.rs:415-430.

**2. Reproducer created and independently executed** —
`verus-ai-logs/nanvix-phys-arch-x86-pde/cheating-elimination/repro/frame_address.rs`. It models the
real situation (opaque `Copy` `FrameNumber` whose bound is exposed only via `into_raw_value()`'s
postcondition; `lemma_frame_address` external_body to isolate the *ordering* limitation). I ran it
both ways myself:

- `bad` (original single expression) FAILS — proving the rewrite is necessary:
```
$ verus frame_address.rs
error: postcondition not satisfied   (frame_address.rs:89  result == self.frame@ * FRAME_SIZE)
error: postcondition not satisfied   (frame_address.rs:90  result % FRAME_SIZE == 0)
verification results:: 4 verified, 1 errors
```
- `good` (the `let raw` + lemma rewrite) PASSES:
```
$ verus --verify-root --verify-function good frame_address.rs
verification results:: 1 verified, 0 errors (partial verification with `--verify-*`)
```

**3. `make verify-arch` re-run by me — exit 0, counts unchanged:**
```
Exit code : 0
cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=2
```
`cheating-detail.txt` still lists ONLY non-pde, TCB-listed constructs:
```
mod.rs:80 invlpg            : external_body   (tcb-allowed.md)
table.proof.rs:16 lemma_entry_roundtrip : assume (tcb-allowed.md)
table.rs:209 read           : external_body   (tcb-allowed.md)
table.rs:246 write          : external_body   (tcb-allowed.md)
```
No pde.rs / pde.spec.rs / pde.proof.rs entry in any cheating category. Build clean (verified turn 1,
unchanged: `cargo build -p arch` → Finished, 0 warnings).

### Fix Request

None. Item 8 is FIXED; all twelve checklist items are PASS/FIXED with tool-verified evidence.
Marking the cheating-elimination phase for arch-x86-pde RESOLVED.
