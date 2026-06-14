# Verification TODO: phys-kframe

Scope (verification-order target functions): `KernelFrame::new`, `KernelFrame::drop`,
`KernelFrame::base`.

## Remaining proof gaps

None. There are **no** `admit()`/`assume()`/proof gaps in any of the in-scope
kframe functions (`new`, `drop`, `base`) or in `kframe.spec.rs` / `kframe.proof.rs`.

- `KernelFrame::base` — verified in-body (`requires self.inv()`,
  `ensures result@ == self@ && result.inv()`).
- `KernelFrame::drop` — verified in-body (`opens_invariants none`, `no_unwind`); makes
  no abstract postcondition (frame release via `super::frame::free` is best-effort).

## Trusted boundary (not a proof gap — TCB-allowed, recorded for honesty)

- `KernelFrame::new` (`kframe.rs:94`) — `external_body`, **explicitly listed in
  `verus-ai-logs/tcb-allowed.md`**. Its body calls `crate::mm::virt::identity_map_page`,
  whose precondition is the **global** `identity_map_view().inv()`. That ghost token is
  owned by the `mm::virt` identity-map singleton and is not realized in the `mm::phys`
  module, so the callee's `requires` cannot be discharged in-scope from the only available
  precondition (`base.inv()`). Eliminated (verified in-body) once `mm::virt`'s identity-map
  token is realized — same cross-module global-token deferral as `frame::book`/`frame::alloc`.
  Contract is non-trivial: `requires base.inv()`,
  `ensures Ok(kf) => kf@ == base@ && kf.inv()`.
