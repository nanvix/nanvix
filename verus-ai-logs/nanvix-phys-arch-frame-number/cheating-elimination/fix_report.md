# Cheating Elimination Report: arch-frame-number

## Scope

Target module: `src/libs/arch/src/x86/mem/paging/frame/number.rs`
In-scope functions: `FrameNumber::into_raw_value`, `FrameNumber::from_raw_value`,
`FrameNumber` (struct + `View`/type-invariant).

## Cheating Counts (before → after)
| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 0      | 0     | 0          |
| assume_specification | 0      | 0     | 0          |
| cfg-gated exec       | 0      | 0     | 0          |

(Counts are for the in-scope module `number.rs`/`number.spec.rs`/`number.proof.rs`.)

## Items Eliminated

None required. The module was already free of every cheating construct.

- `grep -rnE "admit|assume|external_body|assume_specification|external_type_specification"`
  over `src/libs/arch/src/x86/mem/paging/frame/` returns only a single match, which is a
  prose comment in `number.spec.rs:25` ("...discharged by verification rather than
  assumed"). No real cheating constructs exist.
- `FrameNumber::from_raw_value` and `FrameNumber::into_raw_value` carry full
  `#[verus_spec]` contracts and are proven from real bodies:
  - `into_raw_value` discharges its bound via `proof! { use_type_invariant(self); }`
    against the `#[verifier::type_invariant] inv()` (`0 <= self@ <= spec_max()`).
  - `from_raw_value` proves the `Some`/`None` arms directly against `Self::MAX` with the
    interpreted `spec_max()` definition.
- The `View for FrameNumber` mapping is `closed` and the type invariant `inv()` is a real
  `open spec fn`, both proven by verification (not assumed).

## Whole-crate `external_body` (out of scope, all pre-approved)

`make verify-arch`'s cheating check reports `external_body=3` for the whole `arch` crate.
All three are outside the target module and explicitly listed in
`verus-ai-logs/tcb-allowed.md`:

- `x86/mem/paging/mod.rs:80 invlpg` — inline `asm!` TLB flush (external-bottom).
- `x86/mem/paging/table.rs:209 read` — `usize`→`*const` volatile page-table read.
- `x86/mem/paging/table.rs:246 write` — `usize`→`*mut` volatile page-table write.

None belong to `arch-frame-number`; no action required or permitted (hard rule: do not
touch unlisted functions).

## Verification Result

`make verify-arch` → **Exit code 0** (verification succeeds).
The in-scope functions do not appear in `coverage-unverified.txt` (they are verified with
contracts). The crate-level `CHEATING_DETECTED` banner is driven solely by the 3
pre-approved `external_body` entries above.

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-frame-number/verification_todo.md)

None. No proof gaps remain in the module; no `verification_todo.md` entries were needed.

## AST Consistency
- Zero mismatches confirmed: YES
- `git diff verus-ai-prove-bottom-up -- src/libs/arch/src/x86/mem/paging/frame/` is empty:
  the exec source (`number.rs`) is byte-identical to the base branch. No cfg-gated exec
  code, no exec semantics/complexity changes.

## Result: PASS
