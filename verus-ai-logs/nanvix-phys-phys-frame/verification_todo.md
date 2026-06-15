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
