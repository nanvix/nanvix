## Turn 1: Full proving-phase checklist walkthrough — arch::x86::mem::paging::table

### Progress
- Done (PASS/FIXED, all tool-backed): every checklist item below.
- Current: full sweep completed in one turn — no FAIL items.
- Remaining: none.

### Verification

Tools run from repo root `/home/ruize/nanvix-phy-specs`:

1. `make verify-arch` → **exit 0** (whole `arch` crate). Cheating summary:
   `assume=0 external_body=4 admit=0 trusted=0 no_decreases=0 cfg_gate=4`.
   Detail file `verus-ai-logs/verify-arch/verus-logs/cheating-detail.txt`:
   - `x86/mem/paging/mod.rs:80 invlpg: external_body`
   - `x86/mem/paging/table.proof.rs:16 lemma_entry_roundtrip: external_body`
   - `x86/mem/paging/table.rs:209 read: external_body`
   - `x86/mem/paging/table.rs:246 write: external_body`
2. `python3 .../spec_drift.py check arch-paging-table` → baseline `83302a5f4e7d` vs HEAD:
   **0 functions changed, 0 contract drift, 0 ensures removed, 0 requires added.**
3. `make verify` (cross-module, all crates) → bitmap/sys/nanvix-slab/bump-allocator/arch/kernel
   **all exit 0**. (`CHEATING_DETECTED` status is the pattern-flag, not a verification failure.)
4. `./z build -- all` → **[OK] Build complete** (exit 0). The only "Warning:" printed is the
   z-script `sysroot-debug not found` note — a benign tooling message, not a code warning.
5. Read `table.rs`, `table.spec.rs`, `table.proof.rs`, `tcb-allowed.md`, `verus-unsupported.md`.

---

#### 1. No specs weakened — **PASS**
`spec_drift.py check` reports **0 contract drift** (0 ensures removed, 0 requires added) against
baseline `83302a5f4e7d`. The proving phase only replaced the spec-phase `admit()` in
`lemma_entry_roundtrip` with the idiomatic `external_body` axiom form and moved `read`/`write`
bodies behind `external_body` — contracts (`requires`/`ensures`) byte-identical.

#### 2. Zero remaining `admit()` — **PASS**
`make verify-arch` cheating scan: `admit=0` crate-wide. The former spec-phase `admit()` in
`lemma_entry_roundtrip` is gone (replaced by an `external_body` broadcast axiom). `grep` of the
three module files confirms no `admit`.

#### 3. Zero `external_body` unless TCB-listed — **PASS** (every function individually verified)
4 `external_body` total, each cross-checked against `verus-ai-logs/tcb-allowed.md`:
- `table.rs::Table::<E>::read` — listed (§ "introduced while speccing ...table", line 37). usize→ptr
  volatile load. ✓
- `table.rs::Table::<E>::write` — listed (line 47). usize→ptr volatile store, contents-free. ✓
- `table.proof.rs::lemma_entry_roundtrip` — listed (line 59). codec round-trip axiom. ✓
- `mod.rs::invlpg` — listed (§ "...paging (mod.rs)", line 70). inline-asm TLB flush; belongs to the
  separate `arch-paging-mod` proving target, out of this module's scope but properly TCB-listed. ✓
No unlisted `external_body`.

#### 4. Zero assume / assume_specification — **PASS**
`assume=0` crate-wide (verify-arch + verify scan). `grep` for `assume`/`assume_specification` in the
three module files: none.

#### 5. No cfg-gated exec code (branches/expressions/match arms) — **PASS**
In-scope `table.rs`: the only `#[cfg(verus_keep_ghost)]` gates are on `include!("table.spec.rs")` /
`include!("table.proof.rs")` (lines 9–12), which the detector explicitly excludes (`include!`
target). **0 cfg-gated exec code in the table module.**
The crate-wide `cfg_gate=4` are all out-of-scope tooling attributes in sibling modules:
`pte.rs:85`, `pte.rs:307`, `pde.rs:83`, `pde.rs:307`, each
`#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]` — a lint-`allow` marker
auto-attached to a verus impl method, **not** a gated branch/expression/match arm, and in the
separately-verified `arch-x86-pte` / `arch-x86-pde` modules. No exec-logic cfg gating anywhere.

#### 6. Cheating audit (exact counts + locations) — **PASS (reported)**
For the **table module** (in scope): `admit=0, assume=0, external_body=3, cfg_gate(exec)=0`.
- `external_body`: `table.rs:209 read`, `table.rs:246 write`, `table.proof.rs:16 lemma_entry_roundtrip`
  — all TCB-listed.
Crate-wide context: `external_body=4` (+`mod.rs:80 invlpg`), `cfg_gate=4`
(`pte.rs:85,307`, `pde.rs:83,307` — benign markers, other modules), `admit=0`, `assume=0`,
`trusted=0`, `no_decreases=0`.

#### 7. Claimed Verus limitation has an isolated reproducer — **PASS**
`verus-unsupported.md` documents the only claimed limitation (int-to-pointer cast) with an
**isolated** minimal reproducer for the specific construct (a standalone `load(base: usize)` doing
`base as *const u32` + `read_volatile`), plus the exact Verus error
`Verus does not support this cast: usize to *const u32` — not merely the full failing function.

#### 8. Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`) — **PASS**
`grep "VERUS REWRITE" table*.rs` → none. No exec-logic was rewritten. The only exec-body additions
are `proof! { ... }` ghost blocks (`pd_index`, `pt_index`, `into_raw`) and `proof!` does not alter
runtime semantics. Original exec computations (mask/shift, pointer materialization) unchanged.

#### 9. Cross-module regression — **PASS**
`make verify` (all crates incl. kernel): **every crate exit 0**. No cascade from the table-module
`external_body`/axiom into other modules (the `read`/`write` exec signatures are unchanged, so the
out-of-scope `identity_map.rs` callers do not regress).

#### 10. Verification & build: 0 errors, 0 warnings — **PASS**
`make verify-arch` exit 0 (cached; commit `ff5af3692` recorded `47 verified, 0 errors`). verus log
shows no `warning`. `./z build -- all` exit 0, `[OK] Build complete`, no code warnings.

### Fix Request
None — all 10 checklist items PASS with tool-backed evidence. No code changes required.
