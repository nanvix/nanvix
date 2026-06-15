## Turn 3: Item 8 — minimal reproducer (re-verification) → FIXED; full checklist RESOLVED

### Progress
- Done (PASS/FIXED): ALL 12 items.
  1 admit, 2 assume, 3 trusted, 4 no_decreases, 5 cfg-gated exec,
  6 external_body allow-list, 7 AST consistency, 8 exec-rewrite marker +
  reproducer (FIXED this turn), 9 external_body re-confirm, 10 spec drift,
  11 cross-module regression, 12 verification + build.
- Current: item 8 reproducer — re-verified.
- Remaining: none.

### Verification

**Reproducer now faithful (PASS).** I ran the fixer's rewritten
`reproducers/04_free_count_inline_fails.rs` through the real verifier myself:

1. As committed (FAIL form commented out):
   ```
   /mnt/toolchain/verus/verus .../04_free_count_inline_fails.rs
   verification results:: 4 verified, 0 errors
   ```
2. FAIL form temporarily uncommented (I programmatically un-commented the
   `free_count_inline` block and ran it):
   ```
   error: precondition not satisfied
      --> repro:125:14   proof! { lemma_free_count(b); }
       99 |  b@.num_bits >= 0,   ---------------- failed precondition
   verification results:: 4 verified, 1 errors
   ```

Both forms now behave exactly as documented. The defect I found in Turn 2 is
resolved: `Bitmap` is now inside an inner `mod bm` (closed view hidden across the
module boundary, mirroring the real cross-crate `::bitmap`), and the bogus
`result > 0` ensures was removed. The FAIL form fails on exactly the
`num_bits >= 0` precondition — the same error the real tree emits when the
expression is inlined in `frame.rs`.

**Source untouched (PASS).** `git status --short src/kernel/src/mm/phys/` → clean;
the fixer changed only the reproducer file. The `VERUS REWRITE` comment (Turn 1
fix) already cites this path; `free_count`'s exec statements are unchanged from the
base shim.

**Real-tree re-confirmed (items 11 & 12, PASS).**
- `make verify-kernel MODULE=mm::phys` → **Exit 0**; log
  `verus_2026-06-15_10-14-52`: `31 verified, 0 errors`. Cheating counters:
  `assume=0 admit=0 trusted=0 no_decreases=0`, `external_body=23` (all allow-listed),
  `cfg_gate=9` (all `#[cfg(verus_keep_ghost)]` ghost includes).
- `make verify-kernel` (all modules: `frame`, `kframe`, `manager`, `upool`, `mod`)
  → **Exit 0**.
- `./z build -- all-kernel` → `Finished ... [OK] Build complete.`, **0 warnings,
  0 errors**.

### Item-by-item final ledger
1. admit — PASS (admit=0). 2. assume — PASS (assume=0; only `MaybeUninit::assume_init_mut`
inside allow-listed `instance`). 3. trusted — PASS (trusted=0). 4. no_decreases —
PASS (no_decreases=0). 5. cfg-gated exec — PASS (only 2 `#[cfg(verus_keep_ghost)]`
ghost `include!`s in `frame.rs`). 6/9. external_body — PASS (11 in `frame.rs`, each
individually confirmed in `tcb-allowed.md`: 8 `Inner::*`, `instance`, `Drop`-path
`free`, skip/exclude `init`; no unlisted). 7. AST consistency — PASS (`Inner` struct
+ all method bodies byte-identical to base; deviations confined to comment + zero-cost
`let`/erased `proof!` in shims). 8. exec rewrite marker + reproducer — FIXED
(`VERUS REWRITE` comment on `free_count` + faithful, re-verified reproducer).
10. spec drift — PASS (`frame.spec.rs` unchanged; shim contracts strengthened via
`PhysAuth`, not weakened). 11. cross-module — PASS (full verify exit 0). 12.
verification + build — PASS (31 verified/0 errors, build 0 warnings).

### Fix Request
None. All checklist items are PASS or FIXED with tool-verified evidence. Writing
`STOP = RESOLVED`.
