## Turn 2: AST consistency / exec-rewrite documentation — FIXED

### Progress
- Done (PASS/FIXED):
  - Zero admit() — none in region.*; 12 admits are out-of-scope (`manager.proof.rs`,
    `identity_map.*`).
  - Zero assume() — `assume=0`.
  - Zero trusted functions — `trusted=0`.
  - Zero exec_allows_no_decreases_clause — `no_decreases=0`.
  - Zero cfg-gated exec code — only `#[cfg(verus_keep_ghost)] include!` of spec/proof.
  - Zero external_body in region — all 19 are in `mm/phys/*`; none in region.*.
  - **AST consistency / exec rewrite — FIXED** (this turn, see below).
  - **All exec rewrites have VERUS REWRITE comment + minimal reproducer — FIXED**.
  - For each surviving external_body listed in TCB — N/A for region (zero in module);
    the 19 global ones are pre-existing/out-of-scope and unchanged from base.
  - No specs weakened — `spec_drift.py check hal-memory-region` → exit 0, 0 drift.
  - Cross-module regression — `make verify-kernel` → exit 0.
  - Verification + build — `make verify-kernel` exit 0; `./z build` → "Build complete",
    0 compiler warnings.
- Current: final confirmation — all items PASS.
- Remaining: none.

### Verification
The fixer added a `VERUS REWRITE` comment + confirmed minimal reproducer above
`MemoryRegion::start` (region.rs:210–220). I verified independently:

- `grep -n "VERUS REWRITE\|Minimal reproducer" region.rs` →
  `210: // VERUS REWRITE (view-preserving clone): ...`
  `216: // Minimal reproducer (confirmed): ...`
  The comment names the rewrite (`self.start.clone()` → `self.start.clone_address()`),
  states the Verus limitation (`Clone::clone` unspecified, postcondition
  `result@ == self@.start` undischargeable), cites the spec'd trait method
  (`phys.rs:277-288`), and includes a reproducer with both arms
  (`Clone::clone` fails 0/1; `clone_address` verifies 1/0).
- `ast_consistency.py summary` → `matched=27 mismatched=1`. The lone MISMATCH is
  `MemoryRegion::start`, the exec-hash difference being exactly the documented,
  semantically-equivalent (view-preserving, same complexity) `clone_address`
  substitution required by the verified Verus limitation. This now satisfies the
  checklist clause "only semantically-equivalent rewrites for verified Verus
  limitations" because it is documented with comment + reproducer.
- `spec_drift.py check hal-memory-region` (baseline `c4b739de01c1` → HEAD) → exit 0,
  0 contract drift.
- `make verify-kernel` → `Exit code : 0`; module `hal::mem::types::region` verified,
  region-local cheating counts all zero. The global
  `assume=0 external_body=19 admit=12 trusted=0 no_decreases=0 cfg_gate=19` /
  `CHEATING_DETECTED` aggregate is entirely the pre-existing, out-of-scope
  `mm/phys` + `mm/virt` content (per `cheating-detail.txt`), unchanged this phase.
- `./z build` → kernel recompiled, `[OK] Build complete`. The single "Warning:
  Sysroot directory ... not found; skipping symlink update" is a `z`-script
  infrastructure message, not a rustc/kernel compiler warning — 0 code warnings.

### Fix Request
None. The single outstanding blocker from Turn 1 is resolved. All checklist items
PASS or FIXED with tool evidence. Marking the phase RESOLVED.
