# Cheating Elimination Report: arch-frame-number

## Cheating Counts (before → after)

Scope = the `arch-frame-number` module only:
`src/libs/arch/src/x86/mem/paging/frame/number.rs` (+ `.spec.rs` / `.proof.rs`),
functions `FrameNumber`, `FrameNumber::from_raw_value`, `FrameNumber::into_raw_value`.

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

The in-scope module contains **zero** cheating items. `make verify-arch` verifies the
crate with exit code 0; `FrameNumber::from_raw_value` and `FrameNumber::into_raw_value`
are verified in-body against their `#[verus_spec]` contracts, and the `FrameNumber`
type invariant (`inv`) is discharged by verification.

## Items Eliminated

None required — the module was already free of `admit`/`assume`/`external_body`/
`assume_specification`. The two `#[cfg(verus_keep_ghost)]` attributes in `number.rs`
(lines 9, 11) gate only the `include!` of the ghost `number.spec.rs`/`number.proof.rs`
files. This is the standard project-wide ghost-include convention (present in every
file of the `arch` crate — `mod.rs`, `table.rs`, `pte.rs`, `pde.rs`, `lib.rs`), not
prohibited exec-behavior cfg-gating; it is byte-identical to the `verus-ai-prove` base.

### Crate-wide cheating reported by `make verify-arch` (all OUT OF SCOPE)

`make verify-arch` runs over the whole `arch` crate, so its cheating summary
(`assume=0 external_body=3 admit=1 cfg_gate=4`) aggregates other modules:

- `x86/mem/paging/mod.rs:80 invlpg` — `external_body` — **in `tcb-allowed.md`** (inline asm).
- `x86/mem/paging/table.rs:209 read` — `external_body` — **in `tcb-allowed.md`** (usize→ptr).
- `x86/mem/paging/table.rs:246 write` — `external_body` — **in `tcb-allowed.md`** (usize→ptr).
- `x86/mem/paging/table.proof.rs:8 lemma_entry_roundtrip` — `admit` — belongs to the
  separate `table` proof target (`nanvix-phys-arch-paging-table`), not this module.
  Touching it is forbidden by the hard rule "Do not touch unlisted functions."

None of these are in the `arch-frame-number` scope and none were introduced here.

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-frame-number/verification_todo.md)

None. There are no in-scope proof gaps; no `verification_todo.md` was created.

## AST Consistency

- Zero mismatches confirmed: YES.
  `git diff verus-ai-prove -- src/libs/arch/src/x86/mem/paging/frame/` is empty — the
  exec source, signatures, and cfg-gating are byte-identical to the base branch. No exec
  code was changed, so semantics, time complexity, and space complexity are preserved.

## Result: PASS

The `arch-frame-number` module has zero cheating items and verifies cleanly. The only
cheating reported by `make verify-arch` originates in out-of-scope modules: three
`external_body` functions all listed in `tcb-allowed.md`, and one `admit` owned by the
separate `table` proof target (unlisted function — must not be touched).
