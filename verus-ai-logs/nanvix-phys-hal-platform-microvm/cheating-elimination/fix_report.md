# Cheating Elimination Report: hal-platform-microvm

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Counts are **module-scoped** (`src/kernel/src/hal/platform/microvm/`). The
`make verify-kernel MODULE=hal::platform::microvm` cheating check reports:

```
✅ No cheating detected in module hal::platform::microvm.
```

The crate-wide `Global:` line (`external_body=11 admit=27 cfg_gate=14`) belongs
to other, out-of-scope modules and is informational only; the module-scoped
result governs this phase.

## Items Eliminated
- None required. The only in-scope function, `gva_to_gpa` (mod.rs:430), was
  already a complete, non-cheating proof on the base branch:
  - Spec `spec_gva_to_gpa(gva: int) -> int { gva }` (mod.spec.rs:14) — `open`
    identity map, faithful to the MicroVM platform invariant (GVA == GPA,
    identity-mapped guest).
  - Exec body `pub fn gva_to_gpa(gva: usize) -> usize { gva }` with contract
    `ensures result as int == spec_gva_to_gpa(gva as int)`. Verus discharges
    this trivially (`gva as int == gva`).
  - No `admit`/`assume`/`external_body`/`assume_specification` anywhere in
    mod.rs, mod.spec.rs, or mod.proof.rs.
- The two `#[cfg(verus_keep_ghost)]` directives (mod.rs:9, 11) gate
  `include!("mod.spec.rs")` / `include!("mod.proof.rs")` — the repo's standard
  spec/proof-include pattern. They do **not** gate exec code and are not counted
  as cfg-gated exec cheating by the scanner (module `cfg_gate` = 0).

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-platform-microvm/verification_todo.md)
- None. No proof gaps remain.

## AST Consistency
- Zero mismatches confirmed: YES.
  `git diff verus-ai-prove-bottom-up -- src/kernel/src/hal/platform/microvm/`
  is empty — exec code (including `gva_to_gpa`'s body) is byte-identical to the
  base branch. Semantics, time complexity, and space complexity unchanged.

## Result: PASS
