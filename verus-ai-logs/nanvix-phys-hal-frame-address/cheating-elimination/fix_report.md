# Cheating Elimination Report: hal-frame-address

## Cheating Counts (before → after)

Counts are scoped to the in-scope module `hal::mem::types::address::frame`
(`frame.rs` / `frame.spec.rs` / `frame.proof.rs`). The verify-script "Global"
aggregates (`external_body=20 admit=12 cfg_gate=19`) cover the whole kernel crate
and originate entirely in out-of-scope modules.

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 1      | 1*    | 0          |
| assume_specification | 0      | 0     | 0          |
| cfg-gated exec       | 0      | 0     | 0          |

\* The single `external_body` is `FrameAddress::from_raw_value`, which is
**TCB-allowed** — explicitly listed in `verus-ai-logs/tcb-allowed.md`
(`src/kernel/src/hal/mem/types/address/frame.rs::FrameAddress::from_raw_value`).
Per the task's stated exception, listed functions may keep `external_body`.

## Items Eliminated

- None required elimination. The in-scope module was already free of `admit()`,
  `assume()`, `assume_specification`, and cfg-gated exec code. All four verifiable
  in-scope functions carry real specs/proofs and verify in-body:
  - `FrameAddress::into_raw_value` — verified (`result as int == self@`).
  - `FrameAddress::from_frame_number` — verified via `lemma_frame_base_aligned`.
  - `FrameAddress::into_frame_number` — verified via `lemma_aligned_div_mul`.
  - `FrameAddress` (struct) — `external_derive`, no cheating.

- `FrameAddress::from_raw_value` (`external_body`): attempted in-body elimination
  per the verus-constraints escalation ladder. Removing `external_body` fails to
  compile because its callee `<PhysicalAddress as Address>::from_raw_value`
  (`phys.rs:193`, out of scope, no `#[verus_spec]`) is `external`:

  ```
  error: cannot use function
  `...::phys::PhysicalAddress::from_raw_value` which is ignored because it is
  either declared outside the verus! macro or it is marked as `external`.
    --> src/kernel/src/hal/mem/types/address/frame.rs:103:43
  ```

  Verus's only suggested fix is an `assume_specification` (cheating, and explicitly
  removed earlier per `frame.spec.rs:11-15` because the callee is intra-crate and
  not TCB-sanctioned). The legitimate fix — a real spec on the unlisted
  `PhysicalAddress` `Address` impl — is out of scope ("Do not touch unlisted
  functions"). The `external_body` is therefore the sanctioned, TCB-listed choice
  and was restored verbatim. Recorded in `verification_todo.md`.

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-frame-address/verification_todo.md)

- `FrameAddress::from_raw_value`: blocked by `external`/spec-less callee
  `<PhysicalAddress as sys::mm::Address>::from_raw_value`
  (`error: cannot use function ... which is ignored because it is ... marked as
  external`). Eliminated automatically once the HAL physical-address layer carries
  its own verified `#[verus_spec]`. TCB-allowed in the interim.

## AST Consistency

- Zero mismatches confirmed: YES. The working tree `frame.rs` is byte-identical to
  the original baseline (`git diff` against the START commit / `verus-ai-prove` is
  empty). No exec-code changes, no cfg gating added, semantics/time/space complexity
  unchanged.

## Result: PASS
