# Verification TODO — `mm::phys::upool`

Honest hand-off of the genuinely-stuck `external_body` boundaries in `upool.rs`. These are
**not** `admit()`/`assume()` (there are none in this module) — they are two `external_body`
facade methods whose abstract contracts cannot be discharged from inside the `upool` module
because the bridging fact lives in the (unlisted, itself `external_body`, raw-memory) frame
free-function layer. Each entry records the exact Verus error and the rigorous reason no
in-scope rewrite removes it.

Escalation ladder followed (per `verus-constraints`): searched the existing spec vocabulary
(`phys_view()`, `FrameAllocView`, `alloc_one`, `free_count`, `PhysModView::inv`), wrote an
isolated reproducer (removed the `external_body` attributes and re-ran module verification to
capture the real errors), and tried every equivalent view rewrite (interpreted global view,
ghost field). All are recorded below.

---

## 1. `Upool::new` — `external_body`

**Contract:** `ensures result@.wf()`.

**Verus error when the attribute is removed** (`make verify-kernel MODULE=mm::phys`):

```
error: postcondition not satisfied
   --> src/kernel/src/mm/phys/upool.rs:245:13
245 |             result@.wf(),
    |             ^^^^^^^^^^^^ failed this postcondition
```

**Why it is irreducible within `upool`:**

- `view()` is `uninterp spec fn view(&self) -> FrameAllocView`. `result@.wf()` is a non-trivial
  conjunction over a value about which the logic knows nothing → unprovable.
- **Tried: interpret the view as the global allocator state** (`open spec fn view(&self) ->
  FrameAllocView { phys_view().frames }`). `new` then verifies (with `requires
  phys_view().frames.wf()`, which the verified caller `mm::phys::init` already establishes after
  `frame::init`). **But this is unsound for `alloc`** (see §2): `phys_view()` is a 0-argument
  `uninterp spec fn`, i.e. a logic constant, so `old(self)@ == final(self)@` and `alloc`'s
  `alloc_one` transition becomes `S == S.alloc_one(uf@)` together with `uf@ ∈ S.free_frames`,
  which is `false`. An `external_body` `alloc` would then *assume false*. The two methods must
  share one `view()`, so the view cannot be the global constant.
- **Tried: a ghost field** (`state: Ghost<FrameAllocView>` set by `new`, read by `view`). This
  cannot exist: `FrameAllocView` and `vstd::Ghost` are only present under
  `#[cfg(verus_keep_ghost)]`, so the field breaks the ordinary (non-`verus`) build; cfg-gating
  the field would diverge the exec struct (ast-consistency violation). Even if it existed, `new`
  could populate it but `alloc` still could not *prove* the transition (see §2).

**What would remove it:** the §8 ghost token in the frame free-function layer — `frame::init`
returning a `Tracked` permission that pins a spec-readable pool partition. That requires
modifying `frame.rs` (unlisted; hard rule forbids it; and `frame::init`/`frame::alloc` are
themselves `external_body` raw-memory ops). Out of the `upool` scope.

---

## 2. `Upool::alloc` — `external_body`

**Contract (on `Ok(uf)`):** `old(self)@.free_frames.contains(uf@)` and
`final(self)@ == old(self)@.alloc_one(uf@)`; (on `Err`) `final(self)@ == old(self)@` and
`old(self)@.free_count() == 0`.

**Verus error when the attribute is removed:**

```
error: postcondition not satisfied
   --> src/kernel/src/mm/phys/upool.rs:269:17
    |                   ^^^^^^ failed this postcondition
279 |       pub fn alloc(&mut self) -> Result<UserFrame, Error> {
```

**Why it is irreducible within `upool`:**

- The body calls `frame::alloc()`, whose contract only promises (post-state)
  `phys_view().frames.allocated_frames.contains(frame@)` / (`Err`)
  `phys_view().frames.free_frames.is_empty()`. It does **not** state that the returned frame was
  free in *this pool's* abstract partition, nor the full `alloc_one` free→allocated transition.
- Proving `old(self)@.free_frames.contains(uf@)` requires linking `frame::alloc`'s real
  allocation to `self@`'s previous free set. No such link exists without a permission token
  threaded out of `frame::alloc` (the deferred §8 ghost token).
- The same view-representation dilemma as §1 applies: the only view under which the `alloc_one`
  transition is even *expressible* is one that genuinely differs between `old(self)` and
  `final(self)` (an `uninterp`-on-`self` view, as today, or a ghost field). The `uninterp` view
  gives nothing to prove with; the ghost field does not exist in non-`verus` builds and, even if
  it did, the transition still cannot be *proved* from `frame::alloc`'s weak contract.

**What would remove it:** `frame::alloc` (unlisted, `external_body`) returning a `Tracked`
allocation token that proves the real allocation corresponds to a `free→allocated` partition
transition, threaded into a spec-readable `Upool` partition. Out of the `upool` scope; removed
*when the frame free-function layer is verified*, exactly as the sibling `frame::alloc`/`book`/
`share` boundaries (see `tcb-allowed.md`).

---

## Status

- `admit()` / `assume()` / `assume_specification` / `exec_allows_no_decreases_clause` in
  `upool`: **none** (0).
- `external_body` in `upool`: reduced from **3 → 2** (the `Upool` *struct* boundary was
  eliminated; it is now machine-verified). The remaining two (`new`, `alloc`) are genuine §8
  ghost-token boundaries documented above and in `tcb-allowed.md`.
- Verus verification: `make verify-kernel` and `make verify` both pass (Verus exit 0; 116
  verified, 0 errors). The remaining items require verifying the out-of-scope frame
  free-function layer and cannot be discharged from `upool` alone.
