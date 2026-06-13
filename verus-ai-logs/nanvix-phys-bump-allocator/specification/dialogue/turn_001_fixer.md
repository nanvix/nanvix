## Response to Turn 1

I treated the request as "try every approach and show the result," not as a
justification exercise. I empirically attacked all five sub-requests. One is now
**FIXED** (`as_mut_ptr`); the other four (`alloc`/`alloc_as` contracts, the `View`
attachment, the loop invariant) hit **three independent, hard Verus blockers** that
cannot be cleared without either `external_body` (forbidden — `bump_allocator` is
not in `tcb-allowed.md`) or a structural redesign that belongs to the proving
phase. Exact errors from each attempt are below.

### What I Changed

1. **`src/libs/bump_allocator/src/lib.rs:198` — `BssStorage::as_mut_ptr` now has a
   real, non-tautological spec** (was a bare `#[verus_spec]`):
   ```rust
   #[verus_spec(result =>
       ensures
           result as int == base_of::<Self>(),
   )]
   fn as_mut_ptr() -> *mut u8;
   ```
2. **`src/libs/bump_allocator/src/lib.spec.rs:41` — added the backing ghost
   constant** (`view_design.md` §4.2 `base_of`):
   ```rust
   /// Ghost constant: the stable base address revealed by a backend `S`'s
   /// `as_mut_ptr()`. Uninterpreted because a static's address is opaque to Verus.
   pub uninterp spec fn base_of<S: ?Sized>() -> int;
   ```
   This encodes the **stability** half of the §4.2 backend contract (every call
   returns the same address, which `BumpView::base` is pinned to). The
   `A`-alignment / `>= STORAGE_SIZE`-bytes half is **not statable at the trait
   level** — `BssStorage` has no alignment parameter (`A` belongs to the allocator,
   not the trait) — and is the unsafe TCB duty of the backend. The kernel's
   unverified `PageTableBss::as_mut_ptr` impl trusts this ensures; `make verify`
   and `make verify-kernel` both stay at 0 errors (verified empirically below).
3. Updated `verus-unsupported.md`, `verification-todo.md` with the exact errors and
   the `as_mut_ptr` status change.

`align_up`, `BumpView` + `inv()`/geometry, and the three proof-lemma targets are
unchanged and still verify.

### Verification

```
make verify-bump-allocator
  verification results:: 4 verified, 0 errors
  Exit code : 0
  cheating: assume=0 external_body=0 admit=3 trusted=0 no_decreases=0 cfg_gate=0
  status: CHEATING_DETECTED          # admit=3 = spec-phase lemma placeholders only

make verify   (regression — all crates)
  bitmap: 0 errors | sys: 0 errors | nanvix-slab: 0 errors
  bump-allocator: 4 verified, 0 errors | kernel: 1 verified, 0 errors

cargo build  -> Finished, 0 errors
cargo test   -> 3 passed; 0 failed (+ 1 doc-test ok)   # behavior unchanged
```

### Result: NEEDS_GUIDANCE

`as_mut_ptr` is fixed. `alloc`/`alloc_as` (and therefore the §5 caller coverage and
the ≥4/6 coverage target) are blocked by three Verus limitations. I tried each path
and captured the exact compiler output.

**Blocker A — `alloc` body, integer→pointer cast** (after I rewrote the
`break current;` value-break, which itself is unsupported, see Blocker C):
```
error: Verus does not support this cast: `usize` to `*mut [u8; N/#0]`
   --> src/libs/bump_allocator/src/lib.rs:301:27
    |
301 |         Ok(unsafe { &mut *(ptr as *mut [u8; N]) })
    |                           ^^^^^^^^^^^^^^^^^^^^^
```

**Blocker B — `alloc_as` body, raw-pointer deref:**
```
error: The verifier does not yet support the following Rust feature:
dereferencing a raw pointer. Currently, Verus only supports raw pointers
through the permissioned raw_ptr interface:
https://verus-lang.github.io/verus/verusdoc/vstd/raw_ptr/index.html
   --> src/libs/bump_allocator/src/lib.rs:335:26
    |
335 |     Ok(unsafe { &mut *(slot.as_mut_ptr() as *mut MaybeUninit<T>) })
    |                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```
Both A and B materialize a `&'static mut` over memory owned by the external
`BssStorage` backend. The only supported route is `vstd::raw_ptr`, which requires a
`PointsTo`/`PointsToRaw` permission token for that memory. **There is no permission
source** — the backend hands out a bare `*mut u8`. Fabricating one is precisely the
unsafe `BssStorage` contract and needs `external_body`, which the task's hard rule
forbids (no `bump_allocator` entry in `verus-ai-logs/tcb-allowed.md`). So a
*verified body* for `alloc`/`alloc_as` is impossible in this phase, and Verus
verifies every `#[verus_spec]` body unless it is `external_body` — there is no way
to attach the §5 contract without also verifying the body.

**Blocker C — `View` attachment, atomic value not spec-readable** (request #4). I
tried `impl View for FixedSizeBumpAllocator` reading the cursor in `view()`:
```
error[E0061]: this method takes 1 argument but 0 arguments were supplied
    --> src/libs/bump_allocator/src/lib.spec.rs:156:39
     |
 156 |     allocated: self.next_slot.load() as nat,
     |                               ^^^^-- argument #1 of type `Ordering` is missing
```
`load` is `exec`, uncallable from `spec`, and there is **no** spec accessor for a
`core::sync::atomic::AtomicUsize` value. vstd confirms it
(`vstd/std_specs/atomic.rs`):
> "This provides NO support for reasoning about the values inside the atomics. If
>  you need to do that, see `vstd::atomic` or `vstd::atomic_ghost` instead."

So `BumpView` **cannot be the literal `impl View`** of the allocator: its only
dynamic field (`allocated`) lives behind interior mutability. Making it readable
means replacing `AtomicUsize` with `vstd::atomic_ghost` (an `AtomicInvariant`
carrying the ghost state), whose value is openable only inside `atomic_with_ghost!`
`proof`/`exec` blocks — never inside a deterministic `spec fn view()`. The `v → v'`
transition therefore has to be threaded as a ghost token through `&self`. This is
the exact "Modeling note (deferred to proof phase)" in `view_design.md` §7; it is
**not** fixable by changing `BumpView`'s fields or `inv()` (the design is correct —
the obstacle is the atomic representation, not the abstraction).

**Blocker C′ — value-`break` (incidental, fixable but pointless):**
```
error: The verifier does not yet support the following Rust feature:
complex break expressions
   --> src/libs/bump_allocator/src/lib.rs:277:17
    |
277 |             break current;
```
I rewrote it to a `mut idx` + plain `break` (behavior-preserving) to get past it and
reach Blocker A; I then reverted it since `alloc` stays unannotated and the rewrite
yields no verification benefit on its own.

#### Guidance needed (to actually specify `alloc`/`alloc_as`)
Pick one, all outside the spec-phase hard rules:
1. **Add `FixedSizeBumpAllocator::alloc` and `alloc_as` (or a small raw-pointer
   materialization helper they share) to `verus-ai-logs/tcb-allowed.md`**, so the
   `&'static mut`-from-address step may be `external_body` + a trusted §5 spec
   (this is what `src/libs/raw-array` does for the same raw-memory pattern); or
2. **Approve the `vstd::atomic_ghost` structural rewrite** (replace `AtomicUsize`
   with an `AtomicInvariant`-backed cursor + threaded `PointsTo` permissions for
   the storage region) as proof-phase work — this is the only way to get both a
   spec-readable `allocated` *and* a permissioned pointer, and it is substantial.

Until one is granted, the §5 contracts and the ≥4/6 coverage target are
unreachable; the contracts remain captured as the proof-lemma targets in
`lib.proof.rs` and are tracked in `verification-todo.md`.
