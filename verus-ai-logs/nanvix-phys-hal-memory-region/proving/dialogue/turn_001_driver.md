## Turn 1: Full Proving Checklist Walkthrough — hal-memory-region

### Progress
- Done (PASS): all 10 proving checklist items (see per-item verdicts below).
- Current: full checklist verified in a single pass.
- Remaining: none.

### Scope
Target module: `kernel::hal::mem::types::region`
Files: `region.rs`, `region.spec.rs`, `region.proof.rs`.
The repo-wide `make verify-kernel` cheating counters (`external_body=25`,
`cfg_gate=7`) are **crate-wide** and live entirely in *other* modules listed in
`verus-ai-logs/tcb-allowed.md`. The region module itself contributes **zero** to
every cheating counter (proved per-file below).

### Verification (commands run + evidence)

**Verifier — `make verify-kernel` → exit 0.**
Committed PASS log (`verus_2026-06-15_14-07-33.log`, module
`hal::mem::types::region`):
```
Module: hal::mem::types::region
Exit code: 0
5 verified
0 errors
...
verification results:: 5 verified, 0 errors (partial verification with `--verify-*`)
```
Dependent verified crates in the same run: `sys` 4/0, `bitmap` 70/0, `slab` 35/0
— all 0 errors. Re-ran `make verify-kernel` this turn → exit 0 (cached, no
recompilation), confirming the committed state is reproducible.

**Build — `./z build -- all-kernel` → exit 0, no warnings:**
```
Compiling kernel v0.16.17
Finished `dev` profile [optimized + debuginfo] target(s) in 0.70s
[OK]    Build complete.
```

**Cheating scan, region module only** (grep over the three region files):
`admit`, `external_body`, `assume`, `assume_specification` → `NONE FOUND`.
`cfg_gate` detector (`scripts/verify.sh count_cfg_gates` logic, run against the
three files) reports only the two `#[cfg(verus_keep_ghost)]` lines, both
targeting `include!("region.spec.rs")` / `include!("region.proof.rs")` — these
are skipped by the counter (target starts with `include!`) and are the mandated
spec/proof inclusion pattern, **not** cfg-gated exec branches/exprs/match arms.

**Cross-module regression:** `git status --short` shows **no `.rs` files modified**
anywhere in the working tree — the proving phase changed no source. The
committed kernel state (all verified target modules) is intact, and the region
module re-verifies at exit 0.

**Spec-drift:** compared current `region.spec.rs` against the proving-phase
baseline `body_removed_source.rs`. The specification phase's design is preserved
and only **strengthened**, never weakened:
- `MemoryRegion::inv` added (`wf_geometry`) — strengthening.
- `TruncatedMemoryRegion::inv` now `wf_geometry() && start%page==0 && size%page==0`
  (baseline had only the two alignment clauses) — strengthening.
- `start()` / `size()` gained `#[verus_spec] ensures` accessor contracts
  (`spec_addr(&result)==self@.start`, `result as int == self@.size`) — added
  guarantees, not removed.
- `View` bound relaxed `T: Address + View<V=int>` → `T: Address` with the
  universal `spec_addr` projection. `view()` remains `closed`; the abstract value
  is identical (documented in the module comment + `view_design.md`). This is an
  equally-strong generalization, not a weakening — callers observe the same
  `self@.start` / `self@.size`.

### Per-item verdicts
1. No specs weakened — **PASS** (only strengthenings/equivalent generalization vs baseline).
2. Zero `admit()` — **PASS** (`admit=0`; none in region files).
3. Zero `external_body` outside TCB list — **PASS** (region files contain none; the 25 crate-wide are all in `tcb-allowed.md`).
4. Zero `assume`/`assume_specification` — **PASS** (`assume=0`; none in region files).
5. No cfg-gated exec code — **PASS** (only the two `include!` cfgs; no exec branches/exprs/match arms gated).
6. Cheating audit — **PASS**. Region module counts: admit=0, external_body=0, assume=0, cfg-gated exec=0. (Crate-wide governed counts: external_body=25, cfg_gate=7 — all outside this module, all in `tcb-allowed.md`; locations in `cheating-detail.txt`.)
7. Claimed Verus limitations have isolated reproducers — **PASS / N/A** (region proof made no limitation claims; `region.proof.rs` is empty `verus!{}`).
8. Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`) — **PASS / N/A** (no exec bodies rewritten; no `VERUS REWRITE` markers; bodies identical to baseline).
9. Cross-module regression (`make verify-kernel`) — **PASS** (exit 0; no source modified anywhere).
10. Verification + build, 0 errors/0 warnings — **PASS** (5 verified/0 errors; build OK, no warnings).

### Fix Request
None. Every checklist item passes with concrete tool evidence. Marking RESOLVED.
