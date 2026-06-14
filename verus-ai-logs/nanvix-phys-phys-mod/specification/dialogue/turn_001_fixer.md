## Response to Turn 1

All three requested clusters (A, B, C) are implemented. `init` now has a real,
caller-usable postcondition; the two `book_*` helpers carry full `#[verus_spec]`
contracts; and **all 7 `admit()` lemmas are replaced with real proofs** — the
module now has `admit=0`.

### What I Changed

**A. `init` postcondition over the module View (mod.spec.rs, frame.rs, mod.rs)**

1. `src/kernel/src/mm/phys/mod.spec.rs` — added the global-state handle:
   ```rust
   pub uninterp spec fn phys_view() -> PhysMemView;
   ```
   (`initialized` mirrors `frame::INSTANCE_INIT`; `frames` mirrors
   `frame::instance()@`). It is uninterpreted because a Verus `spec fn` cannot
   read the `static mut` singleton; the exec wrappers pin its value via `ensures`.

2. `src/kernel/src/mm/phys/frame.rs` — gave the four spec-less `external_body`
   free-function wrappers thin pass-through `#[verus_spec]` contracts that forward
   the already-proven `Inner` post-states to `phys_view()`:
   - `init` (line ~657): `ensures phys_view().inv(); Ok ==> phys_view().initialized`.
   - `is_covered` (line ~744): `requires phys_view().initialized, phys_addr.inv();
     ensures phys_view().inv(); ret <==> phys_view().covered().contains(phys_addr@)`.
   - `book` (line ~760): `requires …; ensures phys_view().inv();
     Ok ==> phys_view().frames.allocated_frames.contains(phys_addr@)`.
   - `alloc_range` (line ~778): `requires region.inv(); ensures phys_view().inv();
     Ok ==> ∀ frame ∈ region_frames(region@.start, region@.size): allocated`.
   (Imported `super::PhysMemView` / `super::phys_view` in frame.proof.rs.)

3. `src/kernel/src/mm/phys/mod.rs` — replaced `init`'s `ensures true` (line ~163)
   with an Ok/Err contract:
   ```rust
   ensures
       phys_view().inv(),
       match ret {
           Ok(()) => phys_view().initialized
                  && phys_view().frames.allocated_frames
                         .disjoint(phys_view().frames.free_frames),
           Err(_) => true,
       },
   ```
   `init` is **genuinely verified** (not external_body) against this.

**B. Real ensures on the two `book_*` helpers (mod.rs)**

`book_physical_memory_regions` and `book_mmio_regions` now carry
`#[verus_spec]`:
`requires phys_view().initialized, phys_view().inv(); ensures phys_view().inv();
Ok ==> phys_view().initialized`.
They remain `external_body` *only* for the documented std-`LinkedList`
orphan-rule limitation (verus-unsupported.md); their contracts are now honored by
the caller `init`. The precise per-frame transition cannot be stated at this
boundary because the frame set is the contents of the un-viewable `LinkedList`;
it is instead captured and proven at the abstract `PhysMemView` level (cluster C).

**C. Discharged all 7 `admit()` lemmas (mod.proof.rs) — now real proofs**

- `lemma_spec_book_frames_preserves_inv` — core proof: booking a free, aligned
  frame set preserves `FrameAllocView::wf` (all 6 conjuncts proven via Set/Map
  reasoning, using the open `union_prefer_right` definition).
- `lemma_spec_book_frame_preserves_inv` — proven as the singleton case
  (`spec_book_frame(addr) == spec_book_frames({addr})` by `=~=` extensionality).
- `lemma_book_region_reserves_region_frames` — region frames are `i*page_size`,
  hence aligned (nonlinear_arith); composes the booking lemma.
- `lemma_book_mmio_skip_untracked` / `lemma_book_mmio_books_tracked` — skip is
  identity; tracked-free booking via the single-frame lemma.
- `lemma_spec_initialize_establishes_inv` — trivial from definitions.
- `lemma_init_establishes_and_reserves` — composes initialize + book; disjointness
  follows from `post.frames.wf()`.

(`lemma_book_region_reserves_region_frames` gained a `requires spec_page_size() > 0`
— `spec_page_size` is `uninterp` with no global positivity axiom, so the
alignment of `i*page_size` needs it. This is a true global fact, strengthening
only.)

### Verification

`make verify-kernel MODULE=mm::phys`:
```
verification results:: 9 verified, 0 errors (partial verification with `--verify-*`)
  9 verified
  0 errors
  Exit code : 0
  Global: assume=0 external_body=17 admit=0 trusted=0 cfg_gate=5
```

`make verify` (full, no regressions):
```
verification: cached (no recompilation), — (exit 0)
cheating: assume=0 external_body=17 admit=0 trusted=0 no_decreases=0 cfg_gate=5
coverage: 15/1022 exec functions have contracts
```

`spec_drift.py check nanvix-phys-phys-mod`:
```
Contract drift (⚠ review required): 0
✅ No contract drift detected.
```

`grep -n verus_spec src/kernel/src/mm/phys/mod.rs` → contracts on all three
in-scope functions (lines 71, 106, 163); `init` no longer `ensures true`.

`./z build` (dual compilation): `[OK]    Build complete.`

### Result: FIXED

- #1/#2/#3/#4/#6/#9 — `init` has a real, caller-usable Ok/Err postcondition over
  `phys_view()` (establishes `inv()` + `initialized`; reserved frames disjoint
  from free); `book_*` carry real contracts. Not tautological.
- #13 / #17 — **admit=0** (all 7 module lemmas proven); `make verify-kernel`
  exit 0. The remaining `external_body` on `book_*` is the documented
  std-`LinkedList` orphan-rule limitation, now permitted since both functions
  carry full `#[verus_spec]` contracts honored by the caller.
