# Verification TODO — `src/kernel/src/mm/phys/frame.rs`

Status after this proving pass: **32 verified, 0 errors, 0 warnings.**
Remaining `admit()` count: **6** (the mutating free-function shims listed below).
No `assume()` remains. No spec was weakened. No `external_body` was added to any
current-module function.

## Discharged in this pass

- `free_count` — previously a deferred `admit()`, now **fully body-verified** via
  `lemma_free_count` in `frame.proof.rs`. The free set is the image of the clear
  bitmap indices `{ i | 0 <= i < num_bits && !set_bits.contains(i) }` under the
  injective map `i -> i * page_size`; cardinality reduces to
  `num_bits - usage` via `lemma_set_difference_len` + `lemma_map_size`. The only
  fact that cannot be re-derived inside the lemma is `num_bits >= 0` (the bitmap
  `inv()` references the crate-private backing slice and is opaque here); it is
  surfaced from the `usize` result of `number_of_bits()` at the call site — a
  pre-approved "intermediate value for assertions" exec deviation, commented in
  source.

## Genuinely-stuck: the 6 mutating shims

Functions: `alloc`, `alloc_contiguous`, `free`, `share`, `book`, `alloc_range`
(the free-function shims in `frame.rs`, each currently `proof! { admit(); }`
followed by a tail call into the corresponding `Inner::*` method).

### Why they cannot be discharged under the current (frozen) specs

1. The shim `#[verus_spec]` contracts state **post-state** membership facts about
   `phys_view()` — e.g. `Ok` ⇒ `phys_view().frames.allocated_frames.contains(addr@)`.

2. `phys_view()` is `uninterp spec fn phys_view() -> PhysMemView` (`mod.spec.rs`):
   a single **fixed, constant** value within any one proof. There is no
   `old(phys_view())` / argument, so a contract cannot distinguish the pre- and
   post-mutation states of the singleton.

3. The only bridge from the live singleton to the abstract view is `instance()`,
   whose ensures pins the reference to the **pre** state:
   `(*result)@ == phys_view().frames`. After the tail call mutates `*result`,
   that equality is stale; `phys_view().frames` still equals the *pre* state.

4. Therefore a mutating shim's post-state ensures (e.g. "the just-allocated frame
   is now in `allocated_frames`") is being asserted about the **pre** state, where
   that frame is still *free* — contradicted by `FrameAllocView::wf` disjointness.
   The obligation is not just hard, it is **false** against a constant
   `phys_view()`. Pure-query shims (`is_covered`, `refcount`, `free_count`) verify
   precisely because pre == post for them.

### Why each escape hatch is closed

- **Strengthen the proof** — impossible: the obligation is false against the fixed
  `phys_view()`, not merely unproven. No lemma/invariant can make a constant equal
  two different states.
- **Weaken/replace the shim spec** — forbidden (spec-design / spec-drift); upstream
  callers and the `make verify` goal depend on these post-state guarantees.
- **Modify `phys_view()` / `PhysMemView` to add `old`-style diffing** — forbidden:
  `mod.spec.rs` definitions are do-not-modify, and `phys_view()` is intentionally
  argument-free.
- **`external_body`** — forbidden on current-module functions by verus-constraints
  (unconditional in all phases). The shims are deliberately `admit()`-deferred
  rather than `external_body`, as recorded in `verus-ai-logs/tcb-allowed.md`.

### Resolution path (out of scope for this task)

Discharging these six requires a design change to the trusted boundary, e.g.
either (a) give `instance()` an ownership/token spec that lets the mutating
`Inner::*` methods re-pin `phys_view()` to their post-state, or (b) reframe
`phys_view()` as a state-indexed function with `old`/`new` views. Both touch
do-not-modify specs and are outside this module's allowed edits. Until then the
six `admit()`s stand as the documented, TCB-tracked deferral (see
`tcb-allowed.md`), not as proof gaps that further effort in this module can close.
