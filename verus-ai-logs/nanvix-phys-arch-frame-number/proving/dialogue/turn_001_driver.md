## Turn 1: Full proving-phase checklist walkthrough — arch-frame-number

### Progress
- Done (all PASS): every checklist item below.
- Current: full review complete on first pass; no FAIL items.
- Remaining: none.

Scope of this review = the frame-number module:
- `src/libs/arch/src/x86/mem/paging/frame/number.rs`
- `src/libs/arch/src/x86/mem/paging/frame/number.spec.rs`
- `src/libs/arch/src/x86/mem/paging/frame/number.proof.rs`

### Verification (commands run)

1. `make verify-arch` → Exit code 0.
   - Verus result: `cached (no recompilation)`, exit 0 (prior full run PASS; git log:
     `99aa3b088 [verus] verify PASS (cheating detected): arch::all`).
   - `cargo` step: `Finished dev profile` — no errors, no warnings emitted.
   - Crate-wide cheating counter: `assume=0 external_body=3 admit=1 trusted=0
     no_decreases=0 cfg_gate=4`.

2. `cat verus-ai-logs/verify-arch/verus-logs/cheating-detail.txt`:
   ```
   x86/mem/paging/mod.rs:80      invlpg               external_body
   x86/mem/paging/table.proof.rs:8  lemma_entry_roundtrip  admit
   x86/mem/paging/table.rs:209   read                 external_body
   x86/mem/paging/table.rs:246   write                external_body
   ```
   → **None of the crate-wide violations are in the frame-number module.** All three
   `external_body` (`invlpg`, `table::read`, `table::write`) are listed in
   `verus-ai-logs/tcb-allowed.md`. The single `admit` (`table.proof.rs:8
   lemma_entry_roundtrip`) belongs to the separate `arch-paging-table` proving track,
   not this module.

3. Per-module grep over the three frame-number files:
   - `grep -nE 'admit|external_body|assume|assume_specification'` → 0 matches
     (only a prose comment in number.spec.rs:25 "...rather than assumed").
   - `grep -nE '#\[cfg|cfg!'` → only `number.rs:9` and `number.rs:11`, both
     `#[cfg(verus_keep_ghost)]` guarding `include!("number.spec.rs")` /
     `include!("number.proof.rs")` — the standard ghost-code inclusion idiom, NOT
     cfg-gated exec branches/expressions/match arms.
   - `grep -rn 'VERUS REWRITE'` → 0 matches.

4. Spec-drift check: compared current `number.rs` against the proving baseline
   `verus-ai-logs/nanvix-phys-arch-frame-number/body_removed_source.rs`. Every
   `#[verus_spec]` contract is byte-identical:
   - `NULL`: `ensures Self::NULL@ == 0` — unchanged.
   - `from_raw_value`: both `ensures` arms (`<= spec_max ==> Some && @==value`,
     `> spec_max ==> None`) — unchanged.
   - `into_raw_value`: `ensures result as int == self@`, `0 <= self@ <= spec_max()`
     — unchanged.
   Only the bodies were filled in with the original real implementation
   (`into_raw_value` adds a ghost-only `proof! { use_type_invariant(self); }`, no exec
   change). `number.spec.rs` is unchanged (`view` `closed`, `spec_max` `open`, `inv`
   as `#[verifier::type_invariant]`); `number.proof.rs` is empty (`verus! { }`).
   Confirmed the kernel side has *removed* its placeholder `external_type_specification`
   `ExFrameNumber` and the `assume_specification`s for `from_raw_value`/`into_raw_value`
   (frame.spec.rs:20-22, phys.spec.rs:9-22,74-76), now superseded by these real verified
   contracts — i.e. the proving phase made the guarantees stronger downstream, not weaker.

### Per-item verdict

- [x] **No specs weakened** — PASS. Contracts identical to `body_removed_source.rs`
  baseline; kernel placeholders superseded by these real specs.
- [x] **Zero remaining admit()** — PASS for module (0 admits in number files). The
  crate's 1 admit is `table.proof.rs:8`, out of scope (arch-paging-table track).
- [x] **Zero external_body unless TCB-allowed** — PASS. 0 in number files. Crate's 3
  (`invlpg`, `table::read`, `table::write`) are all in `verus-ai-logs/tcb-allowed.md`.
- [x] **Zero assume/assume_specification** — PASS. 0 in number files.
- [x] **No cfg-gated exec code** — PASS. Only `#[cfg(verus_keep_ghost)]` on `include!`
  of ghost spec/proof files; no exec branches/expressions/match arms gated.
- [x] **Cheating audit (exact counts/locations)** — PASS. Frame-number module:
  admit=0, external_body=0, assume=0, cfg-gated-exec=0. Crate-wide (out of scope):
  admit=1 (`table.proof.rs:8`), external_body=3 (all TCB-allowed), cfg_gate=4 (2 of
  which are number.rs's standard ghost-include guards).
- [x] **Claimed Verus limitation has isolated reproducer** — PASS (N/A). Frame-number
  claims no Verus limitation: `number.proof.rs` is empty, no external_body, no admit.
- [x] **Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`)** — PASS.
  No `// VERUS REWRITE` markers; bodies are the original implementations. The only added
  construct is a ghost `use_type_invariant` proof block (no runtime effect).
- [x] **Cross-module regression (`make verify-arch`)** — PASS. Exit 0; all verified
  modules pass (cached PASS from prior full run).
- [x] **Verification + build, 0 errors / 0 warnings** — PASS. `make verify-arch`
  exit 0; cargo `Finished dev profile` with no errors or warnings.

### Fix Request
None. All checklist items PASS with concrete evidence. No code changes required for
the frame-number proving phase. (The crate-wide `admit=1`/`external_body=3` are owned
by the `arch-paging-table` and `arch-paging-mod` tracks — three are TCB-allowed, and
the single admit is outside this module's scope.)
