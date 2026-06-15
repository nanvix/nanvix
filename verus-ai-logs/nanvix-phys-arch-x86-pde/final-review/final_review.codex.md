# Final Verification Review — arch-x86-pde

## Spec Quality

In-scope external-top contracts are present and caller-oriented:
- `PageDirectoryEntryFlags::new`: `ensures result@ == spec_pde_flags_new(...)` (`pde.rs:84-96`), with explicit 8-flag projection (`pde.spec.rs:95-115`).
- `PageDirectoryEntryFlags::is_present`: `ensures result == self@.present` (`pde.rs:131-133`).
- `PageDirectoryEntry::new`: `ensures result@ == spec_pde_new(flags@, frame@)` and `result.inv()` (`pde.rs:312-316`).
- `PageDirectoryEntry::is_present`: `ensures result == self@.flags.present` (`pde.rs:386-387`).
- `PageDirectoryEntry::frame_address`: product and alignment postconditions (`pde.rs:415-419`).

View/invariant quality:
- Views are `closed` and abstract (`pde.spec.rs:71`, `134`), hiding encoding details.
- Mathematical type usage is appropriate (`frame: int` in `PdeView`, `pde.spec.rs:128`).
- `PageDirectoryEntry::inv()` is meaningful (`0 <= frame <= FrameNumber::spec_max()`, `pde.spec.rs:145-147`).
- `PageDirectoryEntryFlags::inv()` is vacuous (`true`, `pde.spec.rs:88-90`), justified because all bit combinations are legal.

Tautology/subsumption check:
- No tautological ensures found.
- `frame_address` alignment clause (`pde.rs:418`) is mathematically implied by the product clause (`pde.rs:417`); retained as explicit caller-facing convenience.

## Caller Coverage (Covered N/Total + Missing list)

### Numbered invariants
Covered **6/6**.
1. Constructor fidelity (flags): covered by `pde.rs:84-96`, `132-133`, and `pde.spec.rs:95-115`.
2. Constructor fidelity (entry): covered by `pde.rs:312-316`, `386-387`, `415-419`.
3. Presence delegation: covered by `pde.rs:386-390` and `131-136`.
4. Frame alignment: covered by `pde.rs:418`.
5. Purity/totality: all in-scope contracts have no `requires` and use pure signatures (`&self` queries / value constructors) at `pde.rs:84,131,312,386,415`.
6. Encoding independence: covered by closed views (`pde.spec.rs:71,134`) and abstract ensures (no raw bit-layout commitments).

### Per-function expectations
Covered **15/15** (3 expectation bullets × 5 functions).
- `PageDirectoryEntryFlags::new`: covered (`pde.rs:84-96`, `pde.spec.rs:95-115`, closed view `pde.spec.rs:71`).
- `PageDirectoryEntry::new`: covered (`pde.rs:312-316`, plus downstream query specs `386-387`, `415-419`).
- `PageDirectoryEntry::is_present`: covered (`pde.rs:386-390`).
- `PageDirectoryEntryFlags::is_present`: covered (`pde.rs:131-136`).
- `PageDirectoryEntry::frame_address`: covered (`pde.rs:415-419`).

**Total covered: 21/21. Missing: none.**

## Proof Completeness

Counts in `pde.rs`, `pde.spec.rs`, `pde.proof.rs`:
- `admit()`: **0**
- `external_body`: **0**

No blocker found for this criterion.

## TCB Compliance

Verifier-reported arch cheating details (`verus-ai-logs/verify-arch/verus-logs/cheating-detail.txt`) include:
- `x86/mem/paging/mod.rs:80 invlpg: external_body`
- `x86/mem/paging/table.rs:209 read: external_body`
- `x86/mem/paging/table.rs:246 write: external_body`

All reported `external_body` items are listed in `tcb-allowed.md`:
- `table.rs::Table::<E>::read` (`tcb-allowed.md:37`)
- `table.rs::Table::<E>::write` (`tcb-allowed.md:47`)
- `mod.rs::invlpg` (`tcb-allowed.md:70`)

`pde.rs/pde.spec.rs/pde.proof.rs` contain **no `external_body`**.

## Guardrails Compliance (exact counts)

Across the three pde files:
- `admit`: **0**
- `assume`: **0**
- `external_body`: **0**
- `assume_specification`: **0**
- cfg-gated exec (`cfg(not(verus_keep_ghost))`): **0**

Note: `cfg(verus_keep_ghost)` appears **2** times in `pde.rs` (`lines 9,11`) only for `include!("pde.spec.rs")` / `include!("pde.proof.rs")`; this is standard spec/proof inclusion, not exec gating.

## AST Consistency (PASS/FAIL + reasoning)

**PASS**.

`ast_consistency.py ... summary` reports exactly one mismatch: `PageDirectoryEntry::frame_address` (matched=22, mismatched=1).

`diff --name "PageDirectoryEntry::frame_address"` shows only:
- original: `self.frame.into_raw_value() << FRAME_SHIFT`
- verified: introduced intermediate `let raw` then same shift.

This matches the pre-approved deviation category “intermediate value for assertions” (ast-consistency skill table).

Justification evidence exists in source:
- `// VERUS REWRITE` explanation at `pde.rs:420-427`
- reproducer reference at `pde.rs:428`
- referenced reproducer file exists: `.../cheating-elimination/repro/frame_address.rs`.

## Verification (PASS/FAIL)

**PASS**.

Ran required command:
`make verify-arch VERUS_EXECUTABLE_DIR=$HOME/toolchain/verus 2>&1 | tail -30`

Observed result (latest run):
- `verification results:: 48 verified, 0 errors`
- `Exit code : 0`

(Overall script still prints `status: CHEATING_DETECTED` due out-of-scope trusted items in other arch files, already TCB-accounted above.)

## Bug Summary

`bugs.md` claims no bugs found. Independent review is consistent with that claim for in-scope targets:
- No functional defect found in the 5 in-scope functions.
- No `external_body` in pde files that could mask a defect.

## Issues (highest priority first)

1. **None (blocking).**
2. **Informational:** `frame_address` alignment ensure (`pde.rs:418`) is subsumed by the product ensure (`pde.rs:417`), but retained for direct caller usability.

## Result: PASS

All strict pass criteria are satisfied for the in-scope pde verification target.
