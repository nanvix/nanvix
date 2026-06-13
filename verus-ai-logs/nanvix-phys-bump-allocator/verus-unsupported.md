# Verus Unsupported Constructs — `bump_allocator` (`src/libs/bump_allocator/src/lib.rs`)

Genuine Verus front-end limitations encountered while specifying the in-scope
exec functions. Per the **verus-constraints** skill these are recorded here and
**not** worked around (no exec rewrite, no `external_body`, no
`assume_specification`). The affected functions are therefore left unannotated so
the crate still compiles under Verus and the remaining items (`align_up`, the
`BumpView` model, and the proof-lemma targets) verify with `0 errors`.

## 1. `break` with a value (`break <expr>;`)

- **Function:** `FixedSizeBumpAllocator::alloc` (`lib.rs:~265`).
- **Exact error:**
  ```
  error: The verifier does not yet support the following Rust feature:
  complex break expressions
     --> src/libs/bump_allocator/src/lib.rs:298:17
      |
  298 |                 break current;
  ```
- **Minimal trigger:** a `loop { ...; break value; }` that yields a value out of
  the loop (the CAS reservation loop).

## 2. Dereferencing a raw pointer to form a reference (`&mut *(ptr as *mut T)`)

- **Functions:** `FixedSizeBumpAllocator::alloc` (`lib.rs:~287`,
  `&mut *(ptr as *mut [u8; N])`) and `FixedSizeBumpAllocator::alloc_as`
  (`lib.rs:356`, `&mut *(slot.as_mut_ptr() as *mut MaybeUninit<T>)`).
- **Exact error:**
  ```
  error: The verifier does not yet support the following Rust feature:
  dereferencing a raw pointer. Currently, Verus only supports raw pointers
  through the permissioned raw_ptr interface:
  https://verus-lang.github.io/verus/verusdoc/vstd/raw_ptr/index.html
     --> src/libs/bump_allocator/src/lib.rs:381:26
  ```
- **Minimal trigger:** turning a `*mut T` (derived from a backend address) into a
  `&'static mut T`. This is the inherently-`unsafe` core of the allocator and
  cannot be expressed in safe Verus without the `vstd::raw_ptr` `PointsTo`
  machinery, which would require rewriting the exec code.

## Consequence

`align_up` and the `BumpView` abstract model verify cleanly. `alloc` and
`alloc_as` cannot be body-verified in this phase; their intended caller contracts
are captured as proof-lemma targets in `lib.proof.rs`
(`lemma_geometry`, `lemma_exhausted_boundary`, `lemma_alloc_transition`) and
tracked in `verification-todo.md`.
