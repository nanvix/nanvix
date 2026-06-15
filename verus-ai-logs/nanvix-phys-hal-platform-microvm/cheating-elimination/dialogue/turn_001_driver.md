## Turn 1: Full checklist walkthrough — hal::platform::microvm

### Progress
- Done (PASS, independently verified): all 12 checklist items.
- Current: final verdict — RESOLVED.
- Remaining: none.

### Verification

Scope target: free function `gva_to_gpa` in
`src/kernel/src/hal/platform/microvm/{mod.rs, mod.spec.rs, mod.proof.rs}`.
Branch `verus-ai/hal-platform-microvm`, working tree clean (all committed).

**1. Zero admit() — PASS.**
`grep admit\(` over the module dir → one hit: `mod.proof.rs:10`, inside a
comment ("Body is `admit()` during the specification phase"). No `admit()` in
code. `make verify-kernel` cheating gate: `admit=0`.

**2. Zero assume() — PASS.**
`grep assume\(` over module dir → no matches. Gate: `assume=0`.

**3. Zero trusted functions — PASS.**
No `#[trusted]` / `trusted` in module. Gate: `trusted=0`.

**4. Zero exec_allows_no_decreases_clause — PASS.**
Gate: `no_decreases=0`. The only exec fn in scope (`gva_to_gpa`) is
non-recursive; no decreases clause involved.

**5. Zero cfg-gated exec code — PASS.**
Only `verus_keep_ghost` cfg in scope is `mod.rs:9,11`, each immediately
followed by `include!("mod.spec.rs")` / `include!("mod.proof.rs")`. The gate's
`count_cfg_gates` explicitly excludes `include!`/`use`/`mod`/derive/logging
targets, so the module contributes 0. The other `#[cfg(feature=...)]` (whp/pit/
smp/stdio) are pre-existing platform feature gates seen identically by build and
Verus — not verification-evasion gates. The 7 crate-wide `cfg_gate` hits are all
outside scope (`hal/mem/*`, `mm/phys/*`).

**6. Zero external_body (unless TCB-allowed) — PASS.**
`grep external_body` over module dir → no matches.
`cheating-detail.txt` lists all 25 crate-wide `external_body`; `grep microvm`
→ NONE. No external_body in scope, so TCB-allowed listing is moot for this module
(confirmed `tcb-allowed.md` has no microvm entry, which is correct).

**7. AST consistency — PASS.**
`git diff dev..HEAD -- microvm/mod.rs` → exactly 2 hunks, both ghost:
(a) `use vstd::prelude::*;` + the two `#[cfg(verus_keep_ghost)] include!`
lines; (b) the `#[verus_spec(...)]` attribute on `gva_to_gpa`. The exec body is
byte-identical: `pub fn gva_to_gpa(gva: usize) -> usize { gva }`. Filtering the
added `+` lines for any non-ghost/non-comment exec content returns only
attribute comments and the closing `)]`. Zero exec-semantics mismatches.

**8. All exec rewrites carry VERUS REWRITE comment + reproducer — PASS (N/A).**
No exec rewrites were performed (exec body unchanged from `dev`). Nothing to
annotate.

**9. Each surviving external_body confirmed in TCB-allowed — PASS (N/A).**
Zero external_body in scope.

**10. No specs weakened — PASS.**
`dev` had no contract on `gva_to_gpa`; this work *introduces* the spec. The
ensures is maximally strong and exact: `result == gva` plus the View
restatement `result as nat == MicrovmTranslationView{}.spec_gva_to_gpa(gva as
nat)` with `spec_gva_to_gpa(x) == x`. `lemma_translation_injective` proves
`injective()` with an empty (non-admit) body, valid from the `open` identity
definition. No guarantee is weakened or dropped.

**11. Cross-module regression — PASS.**
`make verify-kernel`: exit 0, module set `<all>`, status exit 0 (cached, no
recompilation). No verification failures in any module. All crate-wide cheating
items are pre-existing TCB boundaries in other modules; none introduced here.

**12. Verification + build — PASS.**
- `make verify-kernel`: exit 0; `assume=0 admit=0 trusted=0 no_decreases=0`;
  `gva_to_gpa` verified against its `#[verus_spec]` contract.
- `./z build -- all-kernel MACHINE=microvm`: `[OK] Build complete`, exit 0.
- Forced recompile (`touch mod.rs` + `check-kernel`): `build-finished
  success:true`, zero `warning` artifacts emitted. 0 errors, 0 warnings.

### Fix Request
None. Every checklist item PASSES with independently reproduced tool evidence
(grep over module dir, `cheating-detail.txt`, `git diff dev..HEAD`,
`make verify-kernel`, `./z build`). The fixer's PASS report is accurate and the
in-scope module `hal::platform::microvm` is free of cheating constructs.

Verdict: RESOLVED.
