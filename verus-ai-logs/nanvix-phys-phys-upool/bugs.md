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
