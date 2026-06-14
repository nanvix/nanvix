## Turn 1: Full proving-phase checklist for arch-x86-pde

### Progress
- Done (all PASS): spec-drift, admit, external_body, assume, cfg-gated-exec,
  cheating-audit, verus-limitation-reproducer, exec-rewrites, cross-module
  regression, verification+build.
- Current: none — checklist complete.
- Remaining: none.

### Scope
Module under review: `src/libs/arch/src/x86/mem/paging/pde.rs`
(+ `pde.spec.rs`, `pde.proof.rs`). Verus command: `make verify-arch`.

### Verification (commands run + findings)

**Fresh (uncached) `make verify-arch`** — after `touch pde.rs`:
```
verification results:: 47 verified, 0 errors
Exit code : 0
cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=2
```
A second forced run grepping `warning` produced **no warnings**.

**Cheating-detail** (`verus-logs/cheating-detail.txt`):
```
- x86/mem/paging/mod.rs:80  invlpg : external_body
- x86/mem/paging/table.rs:209 read  : external_body
- x86/mem/paging/table.rs:246 write : external_body
```
All three are **outside** the pde module and **all three are explicitly listed
in `verus-ai-logs/tcb-allowed.md`** (inline-asm `invlpg`; int-to-ptr volatile
page-table `read`/`write`). pde.rs itself has **0** external_body.

**cfg_gate=2 location** (reproduced the scanner in `scripts/verify.sh:478`):
```
pde.rs:83  #[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]
pde.rs:307 #[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]
```
These sit on the two `new` constructors. They are **conditional lint-allow
attributes**, not cfg-gated exec code — the function bodies are byte-identical
in both configs; no branch/expression/match arm is gated. The
`verus_impl_method_marker` lint only exists under `verus_keep_ghost`, so the
`allow` **must** be `cfg_attr`-gated (a plain `#[allow(verus_impl_method_marker)]`
is an unknown-lint error in non-verus builds). The scanner flags it as a
false-positive because the following multi-line `#[verus_spec(... ensures ...)]`
attribute confuses its 5-line look-ahead. `grep` over the module confirms the
only other `#[cfg(verus_keep_ghost)]` lines are the `include!` of the
spec/proof files (excluded by the scanner). No genuine cfg-gated exec code exists.

**admit/assume**: live counts admit=0, assume=0; `grep -E "admit|assume"` over
`pde.proof.rs` and `pde.spec.rs` returns nothing.

**Spec drift**: the dedicated tool/skill is not installed locally; verified
manually via git. `git diff 69e026e11(caller-analysis START)..HEAD -- pde.rs`
shows the proving phase **only added** `#[verus_verify]`, `#[verus_spec]`
contracts, and `proof!` ghost calls — **no contract was weakened or removed**.
`git diff b36c5525a..cd31df99f -- pde.spec.rs` is empty. Contracts are strong and
faithful: `new` pins `result@ == spec_pde_{flags_,}new(...)`; `is_present`
pins `result == self@.{flags.}present`; `frame_address` pins
`result as int == self@.frame * FRAME_SIZE` **and** alignment `% FRAME_SIZE == 0`.
View is `closed` (encoding hidden). No drift.

**Exec rewrites** (`grep "VERUS REWRITE"` → none): the single body change is in
`frame_address`:
```
- self.frame.into_raw_value() << crate::mem::FRAME_SHIFT
+ let raw: usize = self.frame.into_raw_value();
+ proof! { lemma_frame_address(raw); }
+ raw << crate::mem::FRAME_SHIFT
```
Extracting the sub-expression into a local + inserting a ghost `proof!` call is
minimal and semantically equivalent (no runtime behavior change). The other
proof addition, `proof! { use_type_invariant(frame); }` in PDE::new, is
ghost-only.

**Verus-limitation reproducer**: the pde module claims **no** Verus limitation
(zero external_body / assume in pde.rs). `frame_address` discharges its bound via
`lemma_frame_address` (real proof, not an escape). N/A → PASS.

**Cross-module regression**: `make verify-arch` verifies the **entire** arch
crate: 47 verified, 0 errors. PASS.

### Verdict (every item)
1. No specs weakened — **PASS** (only contracts added; spec file unchanged).
2. Zero admit — **PASS** (admit=0).
3. Zero external_body unless in TCB list — **PASS** (pde: 0; crate's 3 all in
   `tcb-allowed.md`, none in pde).
4. Zero assume/assume_specification — **PASS** (assume=0).
5. No cfg-gated exec code — **PASS** (cfg_gate=2 are `cfg_attr` lint-allows on
   `new`, not exec code; scanner false-positive; required & non-removable).
6. Cheating audit (exact counts/locations) — **PASS** (admit=0; external_body=3 @
   mod.rs:80, table.rs:209, table.rs:246 — all TCB-allowed, none in pde; assume=0;
   cfg_gate=2 @ pde.rs:83,307 — benign conditional lint-allows).
7. Verus-limitation isolated reproducer — **PASS** (N/A; no limitation claimed in
   pde).
8. Exec rewrites minimal & semantically equivalent — **PASS** (single trivial
   let-binding extraction in `frame_address`; ghost-only otherwise).
9. Cross-module regression — **PASS** (47 verified, 0 errors).
10. Verification + build, 0 errors / 0 warnings — **PASS** (fresh run: 47
    verified, 0 errors, 0 warnings; arch crate compiles cleanly).

### Fix Request
None. All ten checklist items PASS with tool-backed evidence. Resolving.
