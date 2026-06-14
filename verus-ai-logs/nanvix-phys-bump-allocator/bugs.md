# Bugs — `bump_allocator` (`src/libs/bump_allocator/src/lib.rs`)

## Specification phase

No code bugs found while specifying the in-scope functions
(`align_up`, `alloc`, `alloc_as`, `as_mut_ptr`).

- `align_up` is total and correct: it guards `alignment == 0` and uses
  `checked_mul`, matching `align_up_spec`. Verified with `0 errors`.
- `alloc` defensively uses `checked_add` / `checked_mul` for every address
  computation and validates bounds and alignment before handing out a slot, so
  there is no reachable overflow, out-of-bounds, or misalignment fault on the
  success path.
- `alloc_as` validates `size_of::<T>()` and `align_of::<T>()` before touching
  storage.

The only obstacles encountered were Verus front-end limitations (not bugs);
see `verus-unsupported.md` and `verification-todo.md`.

## Proving phase

No code bugs found. All in-scope functions verify (`make verify-bump-allocator`:
`6 verified, 0 errors`, `admit=0`, `assume=0`), and `make verify` passes across
every module (`bitmap`, `sys`, `nanvix-slab`, `bump-allocator`, `kernel`).

- `align_up` and the `BumpView` geometry/capacity/transition lemmas
  (`lemma_geometry`, `lemma_exhausted_boundary`, `lemma_alloc_transition`) verify
  with fully checked proof bodies — no `admit()` or `assume()` remain.
- `alloc` / `alloc_as` were restored to `#[verus_verify(external_body)]` (already
  registered in `tcb-allowed.md`). They cannot be given a checked body in this
  crate: the success path materializes a `&'static mut` slot from a backend raw
  address (`usize as *mut`) over externally-owned `BssStorage` memory with no
  `PointsTo` source, and the CAS loop uses a `break` value Verus rejects
  ("complex break expressions"). Both carry their full caller-facing
  `#[verus_spec]` contracts (alignment, in-bounds, size/align gating), which were
  not weakened.
