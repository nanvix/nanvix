## Turn 1: Full checklist walkthrough — arch-x86-pte (proving phase)

### Progress
- Done (all PASS): every checklist item below was verified with tool output / code reads.
- Current: full-checklist sweep complete.
- Remaining: none.

### Verification

Scope = `x86::mem::paging::pte` (`pte.rs`, `pte.spec.rs`, `pte.proof.rs`).

Fresh, non-cached run (after `touch pte.rs`):
```
verification results:: 47 verified, 0 errors
Exit code : 0
cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=4
```

**1. No specs weakened** — PASS.
- This is bottom-up speccing: no prior specs existed to weaken. No `spec-drift-check`
  skill/tool exists in the repo; verified manually via git diff.
- `git diff 61b47c783 HEAD -- pte.rs` (pre-spec baseline → HEAD) is purely additive:
  `use vstd::prelude::*;`, two `include!` of spec/proof, `#[verus_verify]`,
  `#[verus_spec(...)]` contracts, one ghost `proof! { use_type_invariant(frame); }` in
  `new`. **No exec logic changed/weakened.** The only body edit (`proof!` block) is
  ghost-only and compiles out in non-ghost builds; `Self { flags, frame }` is unchanged.
- Added specs are strong/faithful: `new` ensures full `result@ == spec_pte_flags_new(...)`
  and `result@ == spec_pte_new(flags@, frame@) && result.inv()`; `is_present` ensures
  exact `result == self@.flags.present`. View is `closed` (encoding hidden) — no leakage,
  no weakening.

**2. Zero remaining admit()** — PASS. `grep admit pte*.rs` = 0 hits; crate report `admit=0`.

**3. Zero external_body unless in TCB file** — PASS for pte. `grep external_body pte*.rs`
= 0 hits. The crate's 3 `external_body` are all in **sibling** modules and all listed in
`verus-ai-logs/tcb-allowed.md`:
- `x86/mem/paging/mod.rs:80 invlpg` (TCB-allowed, inline-asm boundary)
- `x86/mem/paging/table.rs:209 read` (TCB-allowed, usize→ptr volatile boundary)
- `x86/mem/paging/table.rs:246 write` (TCB-allowed, usize→ptr volatile boundary)
None in pte.

**4. Zero assume/assume_specification** — PASS. `grep assume pte*.rs` = 0; crate `assume=0`.

**5. No cfg-gated exec code (branches/expressions/match arms)** — PASS. The only `cfg`
in pte: lines 9/11 `#[cfg(verus_keep_ghost)] include!(...)` (spec/proof gating, standard)
and lines 85/307 `#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]`
(conditional **lint-allow** on the two constructors). Neither gates an exec branch,
expression, or match arm; function bodies are identical across configs. No exec divergence.

**6. Cheating audit (exact counts + locations)** — PASS.
- pte module: `admit=0`, `external_body=0`, `assume=0`, cfg-gated-exec=0.
- Crate `cfg_gate=4` enumerated (verify.sh heuristic): `pde.rs:83`, `pde.rs:307`,
  `pte.rs:85`, `pte.rs:307` — **all** are `#[cfg_attr(verus_keep_ghost, allow(...))]`
  lint attributes (heuristic false-positives), none gate exec code. pde:83/307 are
  out-of-scope siblings.
- Crate `external_body=3`: invlpg, table::read, table::write — all TCB-allowed (item 3).

**7. Claimed Verus limitation has isolated reproducer** — PASS (N/A for pte). The pte
module claims **no** Verus limitation: zero external_body / assume / admit in pte*.rs.
(Sibling table/mod limitations are documented in their own `verus-unsupported.md`,
referenced from tcb-allowed.md — outside this scope.)

**8. Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`)** — PASS.
`grep "VERUS REWRITE" pte*.rs` = 0 hits. The git diff confirms **no exec rewrites** were
made; the sole body addition is the ghost `proof!` in `new`. Nothing to scrutinize.

**9. Cross-module regression (verify whole crate)** — PASS. `make verify-arch` runs
`Module: <all>` over crate `arch`: **47 verified, 0 errors, exit 0**. All modules pass.

**10. Verification + build: 0 errors, 0 warnings** — PASS. Fresh run: 47 verified, 0
errors; `cargo` (build-std) `Finished dev profile` with no warnings/errors emitted.
The verus invocation performs the full crate compile, so build is covered.

### Fix Request
None. All ten checklist items PASS with concrete tool evidence. No fixes required.

The `status: CHEATING_DETECTED` banner is driven solely by (a) 3 TCB-allowed sibling
`external_body` functions and (b) 4 `cfg_attr(allow)` lint heuristic hits — none attributable
to a real cheat in the pte module. Marking RESOLVED.
