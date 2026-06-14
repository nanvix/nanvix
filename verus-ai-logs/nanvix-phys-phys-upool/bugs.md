# Bugs — mm::phys::upool

No code bugs found during the specification phase.

## Notes / deferred modeling

- `phys_view()` is a no-argument `uninterp spec fn` (effectively a global
  constant), so before/after global-state transitions cannot be expressed at
  this phase. Consequently the `add_ref` / `release` refcount-transition
  semantics for `UserFrame::share` and `UserFrame::drop` (the "+1" / "release"
  effects on `phys_view().frames`) are not realized as ensures clauses here.
  Only snapshot facts are asserted (e.g. `share` ensures the frame is contained
  in `allocated_frames` on success). This is an intentional, sound limitation
  per `view_design.md` §8, not a bug; it is deferred to the proving phase.

## Proving phase outcome

No code bugs found during proving. All six `UserFrame` methods
(`new`, `address`, `leak`, `share`, `refcount`, `drop`) verify against their
existing contracts with no proof body needed — the `UserFrame::inv` ⇔
`FrameAddress::inv` alignment equivalence discharges the frame-layer
preconditions automatically.

`Upool::new` and `Upool::alloc` are marked `#[verus_verify(external_body)]`
(both listed in `tcb-allowed.md`). This is required, not a weakening:
- `Upool::new` constructs the `external_body` (opaque) `Upool` struct, which
  Verus forbids in a checked body ("constructor for an opaque datatype").
- `Upool::alloc`'s `old(self)@ -> final(self)@` transition (`alloc_one`) is over
  the `uninterp` `Upool::view`, which has no axiom connecting it to the global
  `phys_view()` that `frame::alloc` actually mutates; the transition is therefore
  not derivable in a checked body. Same parameter-free-global-ghost limitation
  noted above for `share`/`drop`.

Module result: `make verify-kernel MODULE=mm::phys` → 42 verified, 0 errors;
upool files contain 0 `admit`, 0 `assume`.
