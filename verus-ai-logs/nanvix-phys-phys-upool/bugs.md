# Bugs — `mm::phys::upool`

None.

No code bugs were found in the eight in-scope functions (`UserFrame::{new,
address, leak, share, refcount, drop}`, `Upool::{new, alloc}`). All specs map to
the code's actual behavior without weakening any inherited contract.

## Notes (not bugs)

- `UserFrame::drop` is marked `external_body` because the `error!` logging macro on
  its error path expands to `core::fmt` `write!`, which Verus cannot translate to
  VIR ("Unsupported constant type"). This is a Verus tooling limitation, not a code
  bug. The `drop` contract (`ensures phys_view().inv()`) is discharged by the
  `frame::free` shim it calls. Recorded in `verus-ai-logs/tcb-allowed.md`.

- `phys_view()` is a zero-argument `uninterp spec fn` (a constant), so before/after
  state transitions are not expressible. All contracts are monotone single-state
  facts over `phys_view()`. The `spec_add_ref` / `spec_drop_ref` / `spec_release`
  transitions sketched in `view_design.md` are therefore not realizable and were
  intentionally omitted; this is consistent with the existing `frame.rs` /
  `manager.rs` contract style.
