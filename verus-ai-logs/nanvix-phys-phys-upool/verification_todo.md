# Verification TODOs: phys-upool

No remaining proof gaps. The `mm::phys::upool` module verifies cleanly
(`make verify-kernel MODULE=mm::phys` → 42 verified, 0 errors).

There are **no** `admit()` or `assume()` anywhere in `upool.rs`,
`upool.spec.rs`, or `upool.proof.rs`. The proof file is empty
(`verus! { }`) — all `UserFrame::*` target functions discharge their
contracts directly from their bodies plus the spec `UserFrame::inv`.

## Authorized `external_body` (NOT proof gaps — listed in `verus-ai-logs/tcb-allowed.md`)

These three are permitted exceptions and cannot be verified in-body for a
structural reason, not a missing proof:

- `Upool` (struct) — opaque facade carrying no spec-readable state; its
  real backing store is the global frame allocator.
- `Upool::new` — `ensures result@.wf()`. `View for Upool` is
  `uninterp spec fn view(&self) -> FrameAllocView;`, so the post-state view
  is an uninterpreted function whose `.wf()` cannot be established from the
  body `Self { _private: () }`. In-body verification is impossible without a
  concrete view definition; the trust obligation is tracked by the
  `external_body` boundary.
- `Upool::alloc` — `ensures` references `old(self)@.free_frames.contains(uf@)`
  and `final(self)@ == old(self)@.alloc_one(uf@)`. The body delegates to
  `frame::alloc()`, which mutates the global `phys_view().frames` partition
  rather than `self`; with `self@` uninterpreted there is no in-body bridge
  between the global allocator transition and the pool view. Authorized
  `external_body` until the `frame` free-function layer is verified.
