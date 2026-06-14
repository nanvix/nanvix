## Turn 1: Full checklist walk-through — arch-paging-table (proving)

### Progress
- Done (all PASS): spec-drift, zero-admit, external_body-allowlist, assume/assume_specification,
  cfg-gated exec, cheating-audit, Verus-limitation-reproducer, exec-rewrites,
  cross-module-regression, verification+build.
- Current: none.
- Remaining: none.

### Verification

Ran `make verify-arch` (exit 0). Cheating summary:
`assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=0`, status reports
CHEATING_DETECTED only because the global counter sees the 3 allow-listed `external_body`.

Per-item evidence:

**1. No specs weakened.**
- `git diff a21d2660c(spec END) HEAD -- table.spec.rs` → empty. `git diff 1a2cf2acd(proving
  START) HEAD -- table.spec.rs` → empty. The specification (views, `TableIndex::inv`,
  `spec_*_index`, `spec_entry_raw/from_raw`, `spec_table_word/read`, `TableView`, all
  `#[verus_spec]` contracts on `from_raw`/`raw`/`new`/`into_raw`/`pd_index`/`pt_index`/`read`/
  `write`) is byte-identical to the approved deliverable. No contract weakened.
- Only proving change: `table.proof.rs` removed the `admit()`-bodied broadcast lemma
  `lemma_entry_roundtrip`. Verified it is dead: `grep -rn lemma_entry_roundtrip src/` finds only
  one *comment* reference (table.rs:233), never `broadcast use`d. It was an unproven axiom that
  no proof/contract consumes (write carries no read-after-write `ensures`). Removing an unused
  admit-axiom is de-cheating, not weakening of any exposed guarantee. PASS.

**2. Zero admit.** Cheating check `admit=0`; proof.rs admit lemma removed. PASS.

**3. Zero external_body unless allow-listed.** `cheating-detail.txt` lists exactly:
  - `table.rs:209 read` — in `tcb-allowed.md` (line 27).
  - `table.rs:246 write` — in `tcb-allowed.md` (line 37).
  - `mod.rs:80 invlpg` — out of this module (arch-paging-mod), allow-listed (line 52).
  Every table.rs `external_body` is individually allow-listed. PASS.

**4. Zero assume/assume_specification.** `assume=0`; `grep assume src/.../table*.rs` → none. PASS.

**5. No cfg-gated exec code.** `cfg_gate=0`. The only `#[cfg(verus_keep_ghost)]` (table.rs:9,11)
  gate the `include!` of `.spec.rs`/`.proof.rs` ghost files — not exec branches/match arms. PASS.

**6. Cheating audit (exact counts/locations).**
  admit=0; assume=0; cfg-gated exec=0; external_body=3 → read (table.rs:209), write (table.rs:246),
  invlpg (mod.rs:80, out-of-module). All accounted for and allow-listed. PASS.

**7. Verus-limitation isolated reproducer.** `verus-unsupported.md §1` gives a minimal standalone
  reproducer (`fn load(base: usize) -> u32 { base as *const u32; read_volatile }`) for the specific
  `usize`→pointer cast construct, with the exact error — not just the full failing function. PASS.

**8. Exec rewrites minimal/semantically equivalent.** `grep "VERUS REWRITE" table*.rs` → none. No
  exec rewrites were introduced in proving (table.rs identical to spec END). PASS.

**9. Cross-module regression.** `make verify-arch` (= verus_cmd_all, all modules) exit 0; last
  non-cached arch run (commit 94a79af7f) = "47 verified, 0 errors". PASS.

**10. Verification + build.** `make verify-arch` exit 0, no `error`/`warning` lines in the run log.
  PASS.

### Fix Request
None. All 10 checklist items PASS with tool-output evidence. No code changes required.

Minor (non-blocking, not a checklist item): the comment at `table.rs:233` references the now-removed
`lemma_entry_roundtrip`. Optional doc cleanup; does not affect any spec/verification guarantee.
