# Verification TODOs: hal-frame-address

## Remaining deferred items (not proof gaps in scope)

- `FrameAddress::from_raw_value` (`src/kernel/src/hal/mem/types/address/frame.rs:102`)
  — carried as TCB-listed `#[verus_verify(external_body)]` (see
  `verus-ai-logs/tcb-allowed.md`). It cannot be verified in-body **yet** because its
  callee `<PhysicalAddress as sys::mm::Address>::from_raw_value`
  (`phys.rs:193`, an unlisted/out-of-scope function whose `impl Address for
  PhysicalAddress` block carries no `#[verus_spec]`) is `external`/spec-less.

  Evidence (external_body removed, `make verify-kernel
  MODULE=hal::mem::types::address::frame`):

  ```
  error: cannot use function
  `kernel::hal::mem::types::address::phys::PhysicalAddress::from_raw_value`
  which is ignored because it is either declared outside the verus! macro or it is
  marked as `external`.
    --> src/kernel/src/hal/mem/types/address/frame.rs:103:43
  ```

  Verus's only suggested fix is an `assume_specification` for that intra-crate callee
  — itself a cheating construct, and one deliberately removed (see
  `frame.spec.rs:11-15`) because the callee is not sanctioned in `tcb-allowed.md`.
  The legitimate fix (a real `#[verus_spec]` on `PhysicalAddress`'s `Address` impl)
  is out of scope for this module ("Do not touch unlisted functions").

  **Resolution path:** eliminated automatically once the HAL physical-address layer
  (`PhysicalAddress as Address`) is verified and carries its own `#[verus_spec]`;
  `from_raw_value`'s body then translates and is verified in-body. No proof gap on
  any in-scope function (`into_raw_value`, `into_frame_number`, `from_frame_number`)
  — all verify with 0 errors.
