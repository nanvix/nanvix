## Turn 1: Full checklist walkthrough — arch-frame-number (proving)

### Progress
- Done (all PASS): No specs weakened; Zero admit; Zero external_body (module);
  Zero assume/assume_specification; No cfg-gated exec; Cheating audit;
  Verus-limitation reproducer (N/A); Exec rewrites minimal; Cross-module
  regression; Verification + build.
- Current: —
- Remaining: none

### Verification

Source files read:
- `number.rs` — struct `FrameNumber(usize)`, `MAX`, `NULL`, `from_raw_value`,
  `into_raw_value` (body uses `proof! { use_type_invariant(self); }`).
- `number.spec.rs` — `View` (`closed` view → `int`), `spec_max()` (open spec),
  `inv()` (`#[verifier::type_invariant]`).
- `number.proof.rs` — `verus! { }` (empty; no proof obligations needed).

Commands run:

1. `make verify-arch` (fresh, after `touch number.rs` to defeat cache):
   ```
   verification results:: 47 verified, 0 errors
   Exit code : 0
   cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=0
   ```
   Module scope = `<all>` (whole arch crate) → cross-module regression covered.
   No `warning` lines in the log.

2. Cheating detail (`verus-ai-logs/verify-arch/verus-logs/cheating-detail.txt`):
   ```
   x86/mem/paging/mod.rs:80   invlpg : external_body
   x86/mem/paging/table.rs:209 read   : external_body
   x86/mem/paging/table.rs:246 write  : external_body
   ```
   All three are listed in `verus-ai-logs/tcb-allowed.md` (invlpg = inline-asm
   hardware boundary; read/write = `usize`→ptr volatile page-table memory).
   **None are in the frame/number module under review.**

3. `grep external_body|admit|assume|VERUS REWRITE` over
   `src/libs/arch/src/x86/mem/paging/frame/` → only hit is the word "assumed"
   inside a `//` comment in `number.spec.rs:25`. No code matches.

4. `git diff 753e1e6af(proving START) HEAD -- number.rs number.spec.rs
   number.proof.rs` → **empty**. Proving phase introduced no changes; the specs
   are byte-identical to the speccing-phase-approved versions (speccing STOP
   present), so no guarantee was weakened.

### Per-item results
- **No specs weakened** — PASS. Empty diff since proving START; contracts
  (`from_raw_value` two-arm spec, `into_raw_value` `result as int == self@` +
  bound, `NULL@ == 0`, `closed` view, type-invariant `inv`, `spec_max`) intact.
- **Zero admit** — PASS. Tool `admit=0`; proof file empty.
- **Zero external_body (this module)** — PASS. The 3 crate-wide hits are all
  TCB-allowed and outside this module.
- **Zero assume/assume_specification** — PASS. Tool `assume=0`.
- **No cfg-gated exec code** — PASS. Tool `cfg_gate=0`. The two
  `#[cfg(verus_keep_ghost)]` attributes guard `include!` of the spec/proof
  (ghost-only) files, not exec branches/expressions/match arms.
- **Cheating audit** — PASS. admit=0, external_body=3 (all TCB-allowed, 0 in
  module), assume=0, trusted=0, no_decreases=0, cfg_gate=0.
- **Verus-limitation reproducer** — PASS (N/A). No Verus limitation claimed in
  this module (no external_body/assume here).
- **Exec rewrites minimal/equivalent** — PASS. No `// VERUS REWRITE` comments;
  exec bodies are the trivial original implementations.
- **Cross-module regression** — PASS. `make verify-arch` Module `<all>` →
  47 verified, 0 errors.
- **Verification + build** — PASS. Fresh cargo-mode run: 47 verified, 0 errors,
  exit 0, no warnings.

### Fix Request
None. All checklist items PASS with tool/diff evidence above.
