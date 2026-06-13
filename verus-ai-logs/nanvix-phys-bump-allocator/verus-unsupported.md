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

## 2. Raw-pointer materialization from an integer address

Two distinct front-end errors, both rooted in turning a backend-provided
`usize`/`*mut u8` address into a typed `&'static mut` reference:

- **`alloc`** (`lib.rs:295`, after the `break` is rewritten to a plain `break`):
  ```
  error: Verus does not support this cast: `usize` to `*mut [u8; N/#0]`
     --> src/libs/bump_allocator/src/lib.rs:301:27
      |
  301 |         Ok(unsafe { &mut *(ptr as *mut [u8; N]) })
      |                           ^^^^^^^^^^^^^^^^^^^^^
  ```
- **`alloc_as`** (`lib.rs:335`):
  ```
  error: The verifier does not yet support the following Rust feature:
  dereferencing a raw pointer. Currently, Verus only supports raw pointers
  through the permissioned raw_ptr interface:
  https://verus-lang.github.io/verus/verusdoc/vstd/raw_ptr/index.html
     --> src/libs/bump_allocator/src/lib.rs:335:26
      |
  335 |     Ok(unsafe { &mut *(slot.as_mut_ptr() as *mut MaybeUninit<T>) })
  ```

`vstd::raw_ptr` is the only supported route, and it requires a `PointsTo`/
`PointsToRaw` permission token for the memory. That memory is owned by the
external `BssStorage` backend (`S::as_mut_ptr()`); there is **no** permission
source for it. Fabricating one is exactly the unsafe `BssStorage` contract and
would require `external_body`, which is forbidden here (`bump_allocator` is not in
`tcb-allowed.md`). Therefore `alloc`/`alloc_as` cannot carry a verified body in
this phase.

## 3. `core::sync::atomic::AtomicUsize` value is not spec-readable

Attaching `BumpView` as the `View` of `FixedSizeBumpAllocator` requires reading the
dynamic `allocated` count from the `next_slot: AtomicUsize` cursor inside
`spec fn view()`. There is no spec accessor:
```
error[E0061]: this method takes 1 argument but 0 arguments were supplied
    --> src/libs/bump_allocator/src/lib.spec.rs:156:39
     |
 156 |     allocated: self.next_slot.load() as nat,
     |                               ^^^^-- argument #1 of type `Ordering` is missing
```
`load` is an `exec` method, uncallable from `spec`. vstd confirms the limitation
in `vstd/std_specs/atomic.rs`:
> "This provides NO support for reasoning about the values inside the atomics.
>  If you need to do that, see `vstd::atomic` or `vstd::atomic_ghost` instead."

Reading `allocated` in spec would require replacing `AtomicUsize` with
`vstd::atomic_ghost` machinery (an `AtomicInvariant` carrying the ghost state),
whose value is openable only inside `proof`/`exec` `atomic_with_ghost!` blocks —
**not** inside a deterministic `spec fn view()`. So `BumpView` cannot be the
literal `impl View` of the allocator; the `v → v'` transition must instead be
threaded as a ghost token through `&self`. This is the proof-phase modeling work
the design doc (§7 "Modeling note") already defers.

## Consequence

`align_up` and the `BumpView` abstract model verify cleanly. `alloc` and
`alloc_as` cannot be body-verified in this phase; their intended caller contracts
are captured as proof-lemma targets in `lib.proof.rs`
(`lemma_geometry`, `lemma_exhausted_boundary`, `lemma_alloc_transition`) and
tracked in `verification-todo.md`.
