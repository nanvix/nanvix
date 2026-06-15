# Verification TODOs — `mm::phys` cheating-gate blockers

## Gate blockers: 4 `admit()` in `manager.proof.rs`

These are the only cheating-gate blockers. Root cause: `phys_view()` is `uninterp`
(a single fixed ghost constant) yet models the **mutable, shared** global frame partition.
The §8 ghost-token attachment (`view_design.md`) that would make this coherent needs
`tracked` ghost state threaded through exec signatures/structs, which the source-integrity
rules forbid. Eliminated 6 of the original 10 admits (see `cheating-elimination/fix_report.md`);
`lemma_user_bulk_ok` was the most recent, closed with a real loop-invariant proof.

All four below assert `post == f(pre)` for an *arbitrary* `post` (or an `uninterp` view), so
they are unprovable as standalone lemmas — and, unlike `lemma_user_bulk_ok`, they remain
unprovable when inlined at the call site, because `self.upool@` structurally does not track the
operation in question.

- **`lemma_manager_attached`** — `m@ == phys_view().frames`.
  Both `Upool::view`/`PhysMemoryManager::view` and `phys_view()` are `uninterp` with no axioms
  → equating them is an underivable external-bottom axiom. Also mutually inconsistent with the
  manager `alloc` specs (`final(self)@ == old(self)@.alloc_one(..)`): a value cannot equal both
  a constant and that constant's `alloc_one`. Used in `alloc_many_user_frames` /
  `alloc_user_frame` to bridge `check_user_watermark`'s global `phys_view().frames.free_count()`
  to the manager's `self@`-phrased `user_alloc_ok` postcondition.
  **Unblock:** human-approved `axiom`/`assume_specification` realizing the attachment, OR a
  `tracked` token over the `frame::INSTANCE` / `PhysMemoryManager`/`Upool` singletons (exec
  signature/struct change — currently forbidden).

- **`lemma_kernel_alloc_one`** — `post == pre.alloc_one(addr)` where `pre=old(self)@`,
  `post=final(self)@`. `alloc_kernel_frame` allocates via the **global** `frame::alloc()` and
  never mutates `self.upool`, so the implementation has `final(self)@ == old(self)@`. The lemma
  asserts `old(self)@ == old(self)@.alloc_one(addr)` — **false for the implementation**.
  **Unblock:** correct the external-top `alloc_kernel_frame` spec to a frame-preserving
  `self@` transition + express the allocation via `phys_view()` (do-not-weaken / unlisted
  function — needs human review), OR record kernel allocations in `self` (semantically wrong;
  exec change).

- **`lemma_kernel_alloc_contiguous`** — same root cause as `lemma_kernel_alloc_one`, bulk
  kernel path (`alloc_many_kernel_frames`). Asserts a `self@` `book_all(..)` transition the
  global contiguous allocation does not perform on the pool view.

- **`lemma_user_bulk_err_restored`** — `m@ == pre` after `frames.clear()`. The K successful
  `self.upool.alloc()` calls advanced `self.upool@` by K `alloc_one`s; `clear()` drops the
  `UserFrame`s whose `Drop` calls the **global** `frame::free()`, which does not roll back
  `self.upool@`. The new strengthened loop invariant on `alloc_many_user_frames` now *proves*
  `self@ == g_old.book_all(user_addr_set(frames@))` with a non-empty set on the error path,
  which **directly contradicts** `m@ == pre` — demonstrating the error-path manager spec
  (`final(self)@ == old(self)@`) is **false for the implementation** without modeling `Drop`'s
  global `frame::free()` effect and reconciling it with the pool view.
  **Unblock:** §8 ghost token making the pool view reflect global frees, or a Drop-effect spec,
  or correcting the (unlisted) `alloc_many_user_frames` error-path external-top spec.

### Resolved this session
- **`lemma_user_bulk_ok`** — *eliminated.* Deleted the lemma and re-derived its facts inline as
  strengthened loop invariants on `alloc_many_user_frames`, using two new pure-spec helpers
  (`lemma_book_all_alloc_one`, `lemma_user_addr_set_push`) plus `lemma_book_all_empty` /
  `lemma_user_addr_set_empty` for the base case. Verified: 85 verified, 0 errors.

---

# Verification TODOs — phys-frame (`src/kernel/src/mm/phys/frame.rs`)

These are the frame-module functions that still carry `#[verus_verify(external_body)]`.
All are listed in `verus-ai-logs/tcb-allowed.md`, so they are *not* cheating-gate
blockers; this file is the honest hand-off recording **why** each body cannot yet be
verified and what unblocks it. Each blocker below was reproduced empirically by removing
`external_body` and running `make verify-kernel MODULE=mm::phys` (see the captured Verus
errors).

## Root cause (shared by `alloc` / `book` / `alloc_range`)

`instance()` (a trusted `external_body` materializer of the `static mut` singleton) pins
only the **pre-call** singleton state:

```
ensures (*r)@ == crate::mm::phys::phys_view().frames
```

`phys_view()` is an `uninterp spec fn` — within a wrapper it is a single fixed ghost value.
After `instance().<mutator>()` mutates `*r`, the real `(*r)@` advances but `phys_view()`
does not, so any postcondition phrased over the **post-mutation** `phys_view().frames`
(allocated/reserved) is not derivable. This is exactly the §8 ghost-token deferral in
`view_design.md`: the `v -> v'` transition is bridged by a tracked token over the singleton
in the proving phase. Verifying it now would require threading a tracked token through each
wrapper, which is impossible without changing the fixed `pub(super)` exec signatures
(`-> Result<…, Error>`), forbidden by the source-integrity rules.

## Remaining items

- **`alloc` (frame.rs ~1325)** — `external_body`.
  Verus error (probe): `postcondition not satisfied` on
  `phys_view().frames.allocated_frames.contains(frame@)`. The returned frame is newly
  allocated, so it is in the **post**-state allocated set but absent from the pre-state
  `phys_view().frames` that `instance()` pins (it was in `free_frames`, disjoint from
  `allocated_frames`). The `Err` arm (`free_frames.is_empty()`) *is* pre-state-expressible
  and provable; only the `Ok` arm is blocked. Unblocked by the §8 singleton ghost token.

- **`book` (frame.rs ~1436)** — `external_body`.
  Verus error (probe): `postcondition not satisfied` on
  `phys_view().frames.reserved(phys_addr@)` (i.e. `allocated_frames.contains`). Same
  post-mutation reference as `alloc`. Unblocked by the §8 singleton ghost token.

- **`alloc_range` (frame.rs ~1457)** — `external_body`.
  Verus error (probe): `postcondition not satisfied` on
  `phys_view().frames.all_reserved(region_frame_addrs(...))`. Same post-mutation reference,
  region-level. Unblocked by the §8 singleton ghost token.

- **`alloc_contiguous` (frame.rs ~1355)** — `external_body`.
  Verus error (probe): `postcondition not satisfied` on
  `base@ + (count as int) * spec_page_size() <= usize::MAX as int`. `Inner::alloc_contiguous`
  guarantees the booked frames `{base@ + i·PS : 0 ≤ i < count}` are a subset of the old free
  set, so the *last allocated* address `base@ + (count-1)·PS ≤ usize::MAX` follows from
  `internal_inv` (`frame_addr_of(i) ≤ usize::MAX` for `i < num_bits`). The wrapper instead
  claims the **one-past-the-end** address `base@ + count·PS ≤ usize::MAX`. When the range ends
  exactly at `num_bits` (`lo + count == num_bits`), this is `frame_addr_of(num_bits)`, which
  `internal_inv` does **not** bound (it only bounds indices `< num_bits`); the bound is not
  derivable (and can be false) under the current invariant. Unblocked by strengthening the
  allocator invariant with `num_bits · PS ≤ usize::MAX` (the manager bridges this in the
  proving phase), which is a do-not-modify spec at this layer.

- **`free` (frame.rs ~1394)** — `external_body`.
  Verus errors (probe): `callee may open invariants that caller cannot` and
  `cannot show this call will not unwind, in function marked 'no_unwind'`, both pointing at
  the `instance()` call. `free`'s contract is `opens_invariants none` + `no_unwind` (so it is
  callable from `UserFrame::drop` / `KernelFrame::drop`), but `instance()` is a plain
  `external_body` with no `opens_invariants`/`no_unwind` annotation and panics
  (`panic!("frame allocator used before init()")`) when the singleton is uninitialized — it
  may both open invariants and unwind. Unblocked when the singleton-access boundary exposes a
  `no_unwind` / `opens_invariants none` accessor (proving-phase ghost-token layer).

## Not in scope (kept `external_body` by design, per `tcb-allowed.md`)

- **`instance`** — trusted `static mut` → `&'static mut Inner` materializer (raw-memory op
  over externally-owned storage; no `PointsTo`). Explicitly allowed.
- **`init`** — listed under *Skip / exclude from current proof target*.

## `assume_specification` (frame.spec.rs, tcb-allowed)

- `<PageAligned<T> as Address>::into_raw_value` and `<PageAligned<T> as Deref>::deref` —
  trusted contracts for the not-yet-verified `hal::mem` address layer. Superseded (removed)
  when that layer is verified. Listed in `tcb-allowed.md`.
