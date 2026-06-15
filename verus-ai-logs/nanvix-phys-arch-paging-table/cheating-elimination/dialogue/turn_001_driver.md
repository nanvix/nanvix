## Turn 1: Full checklist sweep — arch-paging-table cheating elimination

### Progress
- Done (all PASS, evidence below):
  1. Zero admit() — PASS
  2. Zero assume() — PASS
  3. Zero trusted functions — PASS
  4. Zero exec_allows_no_decreases_clause — PASS
  5. Zero cfg-gated exec code — PASS
  6. Zero external_body unless TCB-listed — PASS
  7. AST consistency: zero mismatches — PASS
  8. All exec rewrites have VERUS REWRITE comment + reproducer — PASS (no exec rewrites)
  9. Each surviving external_body confirmed in tcb-allowed.md — PASS
  10. No specs weakened (spec-drift) — PASS
  11. Cross-module regression (make verify) — PASS
  12. Verification + build (verify-arch + build) — PASS
- Current: none
- Remaining: none

### Verification

Scope files:
- `src/libs/arch/src/x86/mem/paging/table.rs`
- `src/libs/arch/src/x86/mem/paging/table.spec.rs`
- `src/libs/arch/src/x86/mem/paging/table.proof.rs`

**1–4. admit / assume / trusted / no_decreases.**
`make verify-arch` → `cheating: assume=0 external_body=4 admit=0 trusted=0
no_decreases=0 cfg_gate=4`. Exit code 0.
Grep of the three table files for `admit`/`assume`: only matches are inside
documentation comments (table.rs:232-233, table.proof.rs:13). No exec/proof
`admit()` or `assume()` calls. PASS.

**5. cfg-gated exec.**
`grep cfg(` in the three table files returns only:
- table.rs:9  `#[cfg(verus_keep_ghost)] include!("table.spec.rs");`
- table.rs:11 `#[cfg(verus_keep_ghost)] include!("table.proof.rs");`
These are spec/proof includes (imports) — the sanctioned pattern, not gated exec
logic. PASS.

**6 & 9. external_body inventory vs TCB list.**
`verus-logs/cheating-detail.txt` lists the 4 arch external_body functions:
- `mod.rs:80 invlpg` (out of table scope; TCB-listed lines 70-83)
- `table.proof.rs:16 lemma_entry_roundtrip` — TCB-listed (tcb-allowed.md:59-66)
- `table.rs:209 read` — TCB-listed (tcb-allowed.md:37-46)
- `table.rs:246 write` — TCB-listed (tcb-allowed.md:47-58)
Every in-scope external_body is individually present in
`verus-ai-logs/tcb-allowed.md` with int-to-ptr / volatile-memory and
codec-axiom rationale. No unlisted external_body. PASS.

**7 & 8. AST consistency.**
`python3 scripts/ast_consistency.py src/libs/arch/.../table.rs count`
→ `✅ Consistent: 7 functions, 2 structs match.`
Exec code is byte-identical (post ghost-strip) to the original, so there are
zero exec rewrites — item 8 is satisfied vacuously (no VERUS REWRITE markers
required). PASS.

**10. Spec drift.**
`spec_drift.py git-diff table.rs --before 4bab7e3c…` (cheating-elimination
phase-start SHA from pipeline_state.json) → Contract drift: 0, ensures removed:
0, requires added: 0. `✅ No contract drift detected.` Exit 0. PASS.

**11. Cross-module regression.**
`make verify` (bitmap, sys, nanvix-slab, bump-allocator, arch, kernel): every
crate `Exit code : 0`. The kernel (consumer of table::read/write/invlpg via
identity_map) verifies. Its admit=4/external_body=23 belong to other
in-progress kernel modules, not arch-paging-table. PASS.

**12. Verification + build.**
- `make verify-arch` → Exit code 0.
- Cargo build of the arch crate with the x86-kernel target/build-std flags →
  `Finished dev profile … in 1.20s`, EXIT 0, **0 errors, 0 warnings**.
PASS.

### Fix Request
None. All checklist items PASS with tool evidence. Recording RESOLVED.
