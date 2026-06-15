## Turn 1: Full Proving Checklist (all items)

### Progress
- Done (PASS): all 10 checklist items verified PASS with tool evidence.
- Current: final review / STOP.
- Remaining: none.

### Verification

Proof target scope: `gva_to_gpa` (free fn) + View `MicrovmTranslationView`
(`mod.spec.rs`) + lemma `lemma_translation_injective` (`mod.proof.rs`).

Commands run:
- `make verify-kernel` → Exit code 0. "note: verifying module
  hal::platform::microvm". Global cheating line:
  `assume=0 external_body=25 admit=0 trusted=0 no_decreases=0 cfg_gate=7`.
- `./z build` → Exit code 0, `[OK] Build complete.`, no compiler warnings
  (only a benign "Sysroot directory not found; skipping symlink update" info).
- `git diff c9088e380 HEAD` on the three module files (admit-commit → HEAD).

Item-by-item:

1. **No specs weakened — PASS.** `git diff c9088e380 HEAD -- mod.spec.rs`
   is EMPTY: the View, `spec_gva_to_gpa` (open identity), `inv`, and
   `injective` are byte-identical to the specification phase. The `#[verus_spec]`
   on `gva_to_gpa` (mod.rs) is unchanged: still ensures both `result == gva`
   AND `result as nat == (MicrovmTranslationView{}).spec_gva_to_gpa(gva as nat)`.
   The only proving-phase change is removal of `admit()` from the proof body —
   strictly strengthening, not weakening.

2. **Zero admit() — PASS.** Verus reports `admit=0`. `git diff` shows the prior
   `admit();` in `lemma_translation_injective` was deleted; the body is now empty
   and discharged directly from the `open` identity definition of
   `spec_gva_to_gpa`. `grep admit` over the three files finds only a historical
   doc comment in mod.proof.rs (not code).

3. **Zero external_body (unless in tcb-allowed) — PASS.** Module target has
   ZERO `external_body`. The global `external_body=25` are all in OTHER modules
   (`mm/phys/*`, `hal/mem/*`, `mm/virt/*`) per
   `verus-logs/cheating-detail.txt`, and every one is listed in
   `verus-ai-logs/tcb-allowed.md`. No microvm/mod.rs entry appears in the detail.

4. **Zero assume / assume_specification — PASS.** Verus reports `assume=0`.
   `grep` over the three module files finds "assume" only in `///` doc comments
   ("It assumes that the standard output device..."), never in code.

5. **No cfg-gated exec code — PASS.** The cheating scanner counts only
   `#[cfg(verus_keep_ghost)]` / `#[cfg(not(verus_keep_ghost))]` gates. microvm
   has exactly two `#[cfg(verus_keep_ghost)]` lines (mod.rs:9,11) and both gate
   `include!("mod.spec.rs")` / `include!("mod.proof.rs")` — the standard
   spec/proof inclusion idiom, explicitly excluded by the scanner (include!
   filter). They contribute 0. The `whp`/`pit`/`smp` feature cfgs are platform
   feature gates, not verus-ghost gates, and `gva_to_gpa` itself has no cfg
   gating. Module cfg_gate contribution = 0.

6. **Cheating audit (exact counts/locations) — PASS.**
   For target module `hal::platform::microvm`:
   - admit = 0
   - external_body = 0
   - assume / assume_specification = 0
   - cfg-gated exec (verus_keep_ghost) = 0 (2 include! gates excluded)
   Evidence: `cheating-detail.txt` has no microvm line; verify summary `admit=0`,
   `assume=0`; grep confirms no in-code occurrences.

7. **Claimed Verus limitation reproducer — PASS (N/A).** This module claims NO
   Verus limitation: it has no external_body, no assume, no rewrite worked around
   a front-end gap. Nothing to reproduce.

8. **Exec rewrites minimal & semantically equivalent — PASS.** No
   `// VERUS REWRITE` comments exist in the module. `gva_to_gpa`'s body is the
   identity `gva` (unchanged from original), matching the `result == gva`
   ensures. No exec rewrites were introduced.

9. **Cross-module regression — PASS.** `make verify-kernel` verifies the whole
   `kernel` crate (cargo mode, `<all>` modules), Exit code 0, no verification
   errors anywhere. All previously verified modules still pass.

10. **Verification + build — PASS.** `make verify-kernel` Exit 0;
    `./z build` Exit 0, "[OK] Build complete.", 0 errors, 0 compiler warnings.

### Fix Request

None. Every checklist item is PASS with concrete tool evidence. No code changes
required. Marking the proving phase RESOLVED.
