## Turn 1: Full checklist walk-through (phys-mod / proving)

### Progress
- Done (PASS): all 10 checklist items verified PASS — see below.
- Current: final sign-off.
- Remaining: none.

Scope = `src/kernel/src/mm/phys/{mod.rs, mod.spec.rs, mod.proof.rs}`.
Cheating in `frame.rs`, `manager.rs`, `upool.rs`, `mm/virt/identity_map.rs`,
`macros.rs` is **out of phys-mod scope** (separate review tracks) and is only
relevant to item 9 (cross-module regression), where it must merely not regress.

### Verification (commands run + evidence)

**1. No specs weakened (spec-drift) — PASS**
`python3 scripts/spec_drift.py check phys-mod` (baseline `a3967834`, proving
phase-start SHA):
```
Functions with changes: 0
Contract drift: 0   (Ensures removed: 0, Requires added: 0)
Functions added/removed: 0
✅ No contract drift detected.
```
`git diff a3967834 HEAD` on the three files is **empty** — proving phase changed
no contracts. Original guarantees intact.

**2. Zero remaining admit() — PASS**
`grep -nE 'admit\s*\('` on the three scope files → 0 hits. (Kernel-wide admit=3 are
all in `mm/virt/identity_map.rs`, out of scope.)

**3. Zero external_body unless TCB-allowed — PASS**
Three external sites in scope, all individually present in `verus-ai-logs/tcb-allowed.md`:
- `mod.rs:73 book_physical_memory_regions` → tcb-allowed.md line 82.
- `mod.rs:103 book_mmio_regions` → tcb-allowed.md line 87.
- `mod.spec.rs:65 ExLinkedList` (`external_type_specification` + mandatory
  `external_body`) → tcb-allowed.md line 74.
Each carries full meaningful `requires`/`ensures` (frames reserved / covered-MMIO
reserved); none is a stubbed-out contract.

**4. Zero assume/assume_specification — PASS**
`grep -nE 'assume\s*\(|assume_specification'` on scope files → 0 hits.
`mod.proof.rs` contains only a documentation comment (no proof obligations).

**5. No cfg-gated exec code — PASS**
Ran the exact `count_cfg_gates` detector from `verify.sh` over `src/kernel/src`:
all 9 kernel cfg-gates are in `macros.rs`, `frame.rs`, `upool.rs` — **zero in
`mod.rs`**. The only cfg attrs in `mod.rs` (`#[cfg(feature="test")]`,
`#[cfg(verus_keep_ghost)]` on `use`/`include!`/`mod`) are not exec-branch gates.

**6. Cheating audit (exact counts + locations, phys-mod scope) — PASS**
- external_body functions: **2** — `mod.rs:73`, `mod.rs:103` (both TCB-listed).
- external_type_specification: **1** — `mod.spec.rs:65 ExLinkedList` (TCB-listed).
- admit: **0**; assume/assume_specification: **0**; cfg-gated exec: **0**;
  no_decreases: **0**; trusted: **0**.

**7. Verus limitation has isolated reproducer — PASS**
`bugs.md` isolates the *specific construct* (`for region in list.iter()` over
`alloc::collections::LinkedList`), not just the failing function: root cause is
`vstd` ships no `LinkedList`/`linked_list::Iter` model and Rust's orphan rule
(E0117) forbids the kernel crate from implementing vstd's `View` /
`ForLoopGhostIteratorNew` / `ForLoopGhostIterator` for the foreign type. Cited
resolution path (Vec/VecDeque are vstd-supported). This is a structural,
construct-level diagnosis, not "the expression fails".

**8. Exec rewrites minimal / VERUS REWRITE comments — PASS**
`grep 'VERUS REWRITE|VERUS DEVIATION'` on scope files → 0 hits. No exec rewrites;
`init` body is verified directly (no loop), calling the two helpers through their
contracts. Helper bodies are original (no semantic change), only annotated.

**9. Cross-module regression — PASS**
`make verify-kernel` verifies `<all>` modules (`mm::phys`, `::frame`, `::kframe`,
`::manager`, `::upool`, etc.). Exit code 0, verification PASS (cached). The
"CHEATING_DETECTED" status reflects pre-existing TCB-approved / other-module
boundaries, not a verification failure and not a regression introduced here
(scope diff is empty per item 1).

**10. Verification + build, 0 errors / 0 warnings — PASS**
- `make verify-kernel`: Exit code 0; full output contains only informational
  `note:` trigger messages (all in out-of-scope `frame.rs`); no `error:` /
  `warning:` lines.
- `./z build`: kernel compiles (`Compiling kernel v0.16.17` → `Finished dev
  profile`) with no warnings/errors; exit 0.

### Fix Request
None — every checklist item is PASS with tool-backed evidence. No code changes
required for the phys-mod proving phase.
