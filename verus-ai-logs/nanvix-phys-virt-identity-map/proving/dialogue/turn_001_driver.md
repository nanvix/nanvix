## Turn 1: Full Proving-Phase Checklist Walkthrough — `mm::virt::identity_map`

### Progress
- Done (PASS): all 10 checklist items verified PASS/FIXED (evidence below).
- Current: none remaining.
- Remaining: none.

### Verification

Command run: `make verify-kernel` → exit 0.
- `note: verifying module mm::virt::identity_map`
- Cheating check (script `scripts/verify.sh`): `assume=0 external_body=29 admit=0 trusted=0 no_decreases=0 cfg_gate=6` (kernel-wide totals).
- Build: `./z build -- all-kernel` → `Finished dev profile`, **0 warnings, 0 errors**.

Per-item findings (all evidence scoped to the three module files):

**1. No specs weakened.**
- `git diff db4eae985..HEAD -- identity_map.spec.rs` shows only the *authoring* of the spec during the specification phase (empty → 144 lines).
- During the proving phase proper (`git diff 7fb4a871c..HEAD`):
  - `identity_map.spec.rs`: **empty diff** (no change).
  - `identity_map.rs` (exec contracts / `ensures`): **empty diff** (no change).
  - Only `identity_map.proof.rs` changed.
- Guarantees in `spec.rs` (`inv`, `maps`, `spec_identity_map_page`, `max_frames`) and the exec `ensures` on `ensure_pt`/`ensure_pte`/`identity_map_page` are byte-identical to the specification phase. **No drift / no weakening. PASS.**

**2. Zero remaining `admit()`.**
- `proof.rs` diff `7fb4a871c..HEAD`: all 4 `admit();` bodies replaced with real proofs:
  - `lemma_map_idempotent`: `assert(v.mapped.insert(frame) =~= v.mapped);`
  - `lemma_map_on_success`: insert-membership asserts.
  - `lemma_map_monotone`: `subset_of` by-block with init/non-init cases.
  - `lemma_map_preserves_inv`: `assert forall ... implies ... by` with init/non-init cases.
- Script reports `admit=0` (global). The only textual `admit` is `proof.rs:16`, a stale comment ("Bodies are left as `admit()` during the specification phase"). **PASS** (no executable admit). *Minor: comment is now outdated but not a violation.*

**3. Zero `external_body` unless TCB-allowed — per-function audit.**
`cheating-detail.txt` lists exactly 3 `external_body` functions in this module; each is individually present in `verus-ai-logs/tcb-allowed.md`:
- `identity_map.rs:521 ensure_pt` → TCB line 212. ✔
- `identity_map.rs:610 ensure_pte` → TCB line 215. ✔
- `identity_map.rs:698 identity_map_page` → TCB line 209. ✔
Plus 4 `external_type_specification` opaque registrations (`ExTableIndex`, `ExPageDirectoryEntry`, `ExPageTableEntry`, `ExTable` in `spec.rs:48/52/56/66`) — standard `arch`-type trust-boundary idiom (mirrors `ExLinkedList`), not function bodies. **PASS.**

**4. Zero `assume`/`assume_specification`.**
- Script: `assume=0` (global).
- Grep hits at `identity_map.rs:534,542` are `MaybeUninit::assume_init_mut()` — a Rust std method, **not** a Verus `assume`. No external-bottom trust boundaries beyond the registered external types. **PASS.**

**5. No cfg-gated exec code.**
- cfg detector regex (`verify.sh:486`) targets `verus_keep_ghost` gates only.
- `identity_map.rs` cfg sites: lines 24 & 26 `#[cfg(verus_keep_ghost)]` gate `include!()` of `.spec.rs`/`.proof.rs` (standard ghost-include idiom, not exec branches); line 723 `#[cfg(feature = "test")]` gates the test module (not counted, standard). No cfg-gated divergent exec branches/expressions/match arms. **PASS.**

**6. Cheating audit — exact counts/locations (this module):**
- `admit` = 0 (executable); 1 stale comment at `proof.rs:16`.
- `external_body` = 3 functions: `ensure_pt` (521), `ensure_pte` (610), `identity_map_page` (698) — all TCB-allowed; + 4 external-type registrations (`spec.rs:48/52/56/66`).
- `assume` = 0.
- cfg-gated exec = 0 (only 2 ghost-include gates + 1 test-module gate).
**PASS.**

**7. Claimed Verus limitation has isolated reproducer.**
- The 3 `external_body` are **trust boundaries** (global `static`s `KERNEL_PD_PADDR`/`KERNEL_CR3` + raw page-table memory; `arch` paging types not Verus-annotated), not claimed front-end *bugs* requiring a reproducer. The TCB entries justify them as trust boundaries; the external types are registered via the standard `external_type_specification` idiom. No "Verus limitation" claim requiring an isolated reproducer is made in this module. **PASS.**

**8. Exec rewrites minimal / `// VERUS REWRITE`.**
- `identity_map.rs` diff during proving is **empty** — zero exec rewrites were introduced in the proving phase. No `// VERUS REWRITE` comments needed (none present). **PASS.**

**9. Cross-module regression (`make verify-kernel`, all modules).**
- Ran `make verify-kernel` (crate = kernel, module `<all>`): exit 0, cached, no errors across all verified modules. **PASS.**

**10. Verification + build: 0 errors, 0 warnings.**
- `make verify-kernel`: exit 0; module `mm::virt::identity_map` verified (commit `eac4f09f4`: "4 verified, 0 errors"); cheating check non-blocking (all external_body TCB-allowed).
- `./z build -- all-kernel`: `Finished dev profile`, 0 warnings, 0 errors. **PASS.**

### Fix Request
None. All checklist items PASS with concrete tool/diff evidence. The proving phase discharged all 4 `admit()`s with real proofs and modified nothing else (spec and exec contracts byte-identical to the specification phase). All `external_body` are individually TCB-allowed.

Optional (non-blocking) cleanup, not required for RESOLVED: update the stale comment at `identity_map.proof.rs:16` which still says bodies are left as `admit()`.
