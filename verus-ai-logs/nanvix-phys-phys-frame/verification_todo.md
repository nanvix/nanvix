# Verification TODO — `src/kernel/src/mm/phys/frame.rs`

Status after cheating-elimination pass: **31 verified, 0 errors** (module
`mm::phys`; full kernel crate: **32 verified, 0 errors**).

- `admit()` remaining: **0**
- `assume()` remaining: **0**
- `assume_specification` remaining: **0**
- non-allow-listed `external_body` remaining: **0**

## No remaining proof gaps

There are **no genuinely-stuck proofs** in `frame.rs`. The 6 mutating
free-function shims (`alloc`, `alloc_contiguous`, `free`, `share`, `book`,
`alloc_range`) that a previous pass left as `proof! { admit(); }` are now fully
body-verified: each threads a `Tracked<&mut PhysAuth>` carrier whose `old(auth)@`
names the pre-state and `final(auth)@` the post-state, and the post-call
`proof! { auth.v.frames = (*r)@; }` re-pins the ghost view to the post-mutation
`Inner` view. This discharges the strong `spec_alloc_one` / `spec_alloc_set` /
`spec_share` post-state contracts without any `admit()` — the exact obstacle the
old `verification-todo.md` recorded (a fixed, argument-less `phys_view()` cannot
distinguish pre/post states) is solved by carrying the state in the `PhysAuth`
token instead of re-reading `phys_view()`.

`free_count` is body-verified via `lemma_free_count` (`frame.proof.rs`).

## Allow-listed trust boundaries (intentionally `external_body`)

These remain `external_body` by design, each listed in
`verus-ai-logs/tcb-allowed.md` with justification (no proof gap):

- `Inner::alloc`, `Inner::alloc_contiguous`, `Inner::free`, `Inner::share`,
  `Inner::refcount`, `Inner::book`, `Inner::is_covered`, `Inner::alloc_range` —
  bodies use `error!` / `debug_assert_eq!` (need `core::fmt::Arguments`,
  unsupported by Verus) and the `arch` newtypes `FrameNumber` / `FrameAddress`
  (external types without `external_type_specification`). Their rich
  `old(self)@ → final(self)@` contracts are the trust boundary.
- `instance` — `static mut INSTANCE` bridge axiom; `static mut` paths are
  unsupported by the verifier.
- `free` (Drop path) — reached from `UserFrame`/`KernelFrame::drop`, which are
  `opens_invariants none` + `no_unwind` and cannot receive a `PhysAuth` carrier.
- `init` — skip/exclude target; materializes `&'static mut [u8]` and writes
  `static mut INSTANCE`.
