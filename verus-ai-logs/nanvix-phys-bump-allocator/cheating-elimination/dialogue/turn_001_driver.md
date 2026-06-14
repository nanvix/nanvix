## Turn 1: Full Checklist Walkthrough — bump-allocator cheating-elimination

### Progress
- Done (PASS): all 12 checklist items (see Verification).
- Current: final sign-off.
- Remaining: none.

### Verification

Commands run (output read directly, not trusted from fix_report):

`make verify-bump-allocator` →
```
Exit code : 0
cheating: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0
```

`cargo build -p bump-allocator --locked` → `Finished` (0 errors, 0 warnings).

Cross-module `make verify-{bitmap,sys,nanvix-slab,bump-allocator,arch}` → every crate **Exit code 0**.

Per-item findings:

1. **Zero admit()** — PASS. Tool: `admit=0`. (Prior commits had `admit=3`; now eliminated.)
2. **Zero assume()** — PASS. Tool: `assume=0`. The `assume_specification [<usize>::div_ceil]` (lib.spec.rs:28) is a std-library contract, not `assume()`, and is not counted by the gate.
3. **Zero trusted** — PASS. Tool: `trusted=0`.
4. **Zero exec_allows_no_decreases_clause** — PASS. Tool: `no_decreases=0`.
5. **Zero cfg-gated exec code** — PASS. Tool: `cfg_gate=0`. Manually confirmed the only `cfg` uses are: `#![cfg_attr(not(any(test, feature="std")), no_std)]` (config), `#[cfg(verus_keep_ghost)]` guarding `include!` of spec/proof (ghost), and `#[cfg(test)]` test module. None gate divergent exec bodies.
6. **external_body only if TCB-listed** — PASS. Exactly two: `FixedSizeBumpAllocator::alloc` (lib.rs:286) and `FixedSizeBumpAllocator::alloc_as` (lib.rs:367). Both are explicitly listed in `verus-ai-logs/tcb-allowed.md` (lines 16–23) with raw-memory/`usize as *mut` rationale, and both retain full `#[verus_spec]` contracts (alignment + in-bounds over `bump_view`; plus `size_of`/`align_of` guards for `alloc_as`). Each addressed individually:
   - `alloc` — TCB-listed ✓, contract present ✓.
   - `alloc_as` — TCB-listed ✓, contract present ✓.
7. **AST consistency: zero mismatches** — PASS. `grep "VERUS REWRITE"` → none; no cfg-gated exec variants; all Verus-specific code is either attribute-only (`#[verus_verify]`, `#[verus_spec]`) or `#[cfg(verus_keep_ghost)]`-gated. The exec code compiles unchanged in non-Verus mode (`cargo build` succeeded), so the exec AST is identical across verified/unverified builds.
8. **All exec rewrites have VERUS REWRITE comment + reproducer** — PASS (vacuous). No exec rewrites exist (`grep "VERUS REWRITE"` empty).
9. **Each surviving external_body TCB-listed** — PASS (same as #6).
10. **No specs weakened (spec drift)** — PASS. `git diff` on the crate is empty (clean tree). Relative to prior committed state the change is strict strengthening (`admit=3 → 0`); both external_body sites keep their full `ensures`. No guarantee removed or relaxed.
11. **Cross-module regression** — PASS. All five Verus crates verify with exit 0. The `CHEATING_DETECTED` status on other crates reflects their own (out-of-scope) cheating counters, not a regression in verification.
12. **Verification + build: 0 errors, 0 warnings** — PASS. `make verify-bump-allocator` exit 0; `cargo build -p bump-allocator` clean.

### Fix Request
None. Every checklist item is PASS with concrete tool evidence. No code changes required.
