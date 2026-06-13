## Response to Turn 2

### What I Changed

Applied the `raw-array` `external_body` + `verus_spec` pattern to `alloc`/`alloc_as`
exactly as requested, with **one mechanical deviation forced by a Verus front-end
panic** (documented below): the abstract view is exposed as a free uninterpreted
`bump_view(self)` function instead of an `impl View`/inherent `view()`. It is read by
callers identically to `self.view()`.

1. `src/libs/bump_allocator/src/lib.spec.rs` (~line 158–179) — replaced the attempted
   `impl View for FixedSizeBumpAllocator { uninterp spec fn view(&self) -> BumpView; }`
   with a free, uninterpreted accessor (the panic-free analog of `raw-array`'s
   `uninterp spec fn view(&self) -> Seq<T>`):
   ```rust
   pub uninterp spec fn bump_view<const N: usize, const A: usize, S: BssStorage>(
       a: &FixedSizeBumpAllocator<N, A, S>,
   ) -> BumpView;
   ```
   Also retained (added earlier in this turn during the attempt) the uninterpreted
   slot-address accessor, since Verus references — unlike raw pointers — expose no
   spec-readable address (`slot as int` fails with `&mut [u8; N]: Copy is not
   satisfied`):
   ```rust
   pub uninterp spec fn slot_ref_addr<T: ?Sized>(r: &T) -> int;
   ```

2. `src/libs/bump_allocator/src/lib.rs` — `BumpAllocError` enum (~146), `BssStorage`
   trait (~191) and `FixedSizeBumpAllocator` struct (~218) carry `#[verus_verify]`
   (required so the methods’ `#[verus_spec]` can name the types).

3. `src/libs/bump_allocator/src/lib.rs::alloc` (~271) — `#[verus_verify(external_body)]`
   + `#[verus_spec]`. The `Ok` arm carries alignment + in-bounds over `bump_view(self)`
   (not `true`); `Err(_) => true` (the `allocated+1`/uniqueness transition is deferred
   to the proving phase per view-design §7, captured by `lemma_alloc_transition`):
   ```rust
   requires bump_view(self).inv(),
   ensures match result {
       Ok(slot) => {
           let v = bump_view(self);
           let a = slot_ref_addr(slot);
           &&& a % (v.unit_align as int) == 0
           &&& v.base <= a
           &&& a + (N as int) <= v.base + (v.storage_size as int)
       },
       Err(_) => true,
   }
   ```

4. `src/libs/bump_allocator/src/lib.rs::alloc_as<T>` (~344) —
   `#[verus_verify(external_body)]` + `#[verus_spec]` with the size/align guard arms
   and an `Ok` arm carrying `size_of::<T>()==N && align_of::<T>()<=A` plus the same
   alignment + in-bounds facts (`vstd::layout::size_of`/`align_of`):
   ```rust
   requires bump_view(self).inv(),
   ensures match result {
       Ok(slot) => { /* size_of==N, align_of<=A, aligned, in-bounds over bump_view */ },
       Err(BumpAllocError::SizeMismatch)      => vstd::layout::size_of::<T>()  != N as nat,
       Err(BumpAllocError::AlignmentMismatch) => vstd::layout::align_of::<T>() >  A as nat,
       Err(_) => true,
   }
   ```

5. `verus-ai-logs/tcb-allowed.md` — registered both functions under "Allowed
   `external_body`" with a note that the raw-memory materialization mirrors
   `src/libs/raw-array`.

6. `verus-ai-logs/nanvix-phys-bump-allocator/view_design.md` — updated the View
   attachment note to record the `bump_view` free-fn decision and its rationale.

#### Why a free `bump_view` and not `impl view()` (exact error)

I applied the requested `impl ... { uninterp spec fn view(&self) -> BumpView; }`
verbatim. The Verus **front end panics** while lowering this `include!`-composed crate
(a second `impl` block on `FixedSizeBumpAllocator` collides with the exec-method
`impl`). Exact output of `make verify-bump-allocator` with the `impl view()` form:

```
thread 'rustc' (1155145) panicked at vir/src/context.rs:337:13:
assertion failed: !trait_impl_map.contains_key(&trait_impl.x.impl_path)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
  process didn't exit successfully: `/home/ruize/toolchain/verus/verus ... --crate-name bump_allocator ...` (exit status: 1)
  verification: 0 verified, compilation/setup error (verus did not run) (exit 101)
```

An inherent `impl ... { spec fn view }` form panics identically. The free
`bump_view(self)` adds no second impl block, so it lowers cleanly while delivering the
same `self.view()` semantics the reviewer asked for (full `BumpView`, `inv()` as
`requires`, `base`/`unit_align`/`storage_size` in `ensures`). This is the "paste the
exact new Verus error" path the request authorized.

### Verification

`make verify-bump-allocator`:
```
verification results:: 5 verified, 0 errors
  Exit code : 0
  cheating: assume=0 external_body=2 admit=3 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 3/6 exec functions have contracts
```
- `coverage-unverified.txt` is now `fmt, new, default` — **`alloc` and `alloc_as` are
  gone** (coverage 3/6, target met). `as_mut_ptr` is a body-less trait method, not in
  the denominator.
- `external_body=2` (justified by the two new `tcb-allowed.md` entries; reported, not
  hidden). `admit=3` unchanged (the proof-lemma placeholders; did not grow).

`make verify` (no regressions): every crate exits 0, `0 errors` (kernel: `1 verified,
0 errors`).

`make build` / dual compilation: `cargo build` OK; `cargo test` → 3 unit tests + 1
doctest pass.

### Result: FIXED
