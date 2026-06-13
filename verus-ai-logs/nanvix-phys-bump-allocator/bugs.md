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
