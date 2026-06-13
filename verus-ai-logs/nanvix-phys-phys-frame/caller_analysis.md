# Caller Analysis: `mm::phys::frame`

## Script Output

See: `verus-ai-logs/nanvix-phys-phys-frame/find_callers_output.md`
(raw `find_callers_lsp.py` report).

Key facts from the script:

- **Crate:** `kernel`. **Depended on by:** none — every caller is intra-crate.
- All public free functions are `pub(super)`, i.e. visible only to the parent
  `mm::phys` module and its siblings (`mod.rs`, `manager.rs`, `upool.rs`,
  `kframe.rs`). There are **no external-crate callers and none are possible**.
- The script's per-function "Context" column is line-misaligned (it quotes the
  wrong source line). The call sites below were re-confirmed by direct grep/read
  of the caller files, not by trusting that column.
- Module shape: 19 exec functions = 10 `pub(super)` free functions (thin
  singleton wrappers) + 9 private `Inner` methods. Each free function is a
  one-line delegation `instance().<method>(...)`.

### Internal call graph (free fn → Inner method)

| Free fn (`pub(super)`) | Delegates to | Notes |
|---|---|---|
| `alloc` | `Inner::alloc` | via `instance()` |
| `alloc_contiguous` | `Inner::alloc_contiguous` | `Inner::alloc_contiguous` itself calls `Inner::alloc_range` (bitmap range) |
| `alloc_range` | `Inner::alloc_range` | boot-time region booking |
| `free` | `Inner::free` | |
| `free_count` | reads `Inner.bitmap` directly (`number_of_bits - usage`) | does not call an `Inner` method |
| `book` | `Inner::book` | |
| `is_covered` | `Inner::is_covered` | |
| `share` | `Inner::share` | |
| `refcount` | `Inner::refcount` | |
| `init` | (writes the singleton) | **skip/excluded** target |

`instance()` (private) is the choke point: called by all 9 non-`init` free
functions. It panics if the singleton has not been initialized, so every
public entry point implicitly requires `init()` to have run first.

## Trait Obligations

The frame module exposes no public trait impls. The *callers*, however, surface
the module through two `Drop` impls, which imposes constraints on `free`:

- Trait: `Drop` for `UserFrame` (`upool.rs:201`) — expected semantics: releasing
  the handle returns its physical frame via `frame::free`, ignoring the result,
  with `opens_invariants none` / `no_unwind`. `frame::free` must therefore be
  callable from a destructor: no precondition, no unwind, no invariant opening.
- Trait: `Drop` for `KernelFrame` (`kframe.rs:197`) — identical contract; the
  comment states this is the *sole, complete* deallocation step for a kernel
  frame.

## Caller Expectations

### `alloc() -> Result<FrameAddress, Error>`
Callers: `manager.rs:366` (`PhysMemoryManager::alloc_kernel_frame`),
`upool.rs:280` (`Upool::alloc`).
- Callers assume (Ok): the returned `FrameAddress` is valid/page-aligned
  (`frame.inv()`), was previously **free**, and is now reserved with refcount 1.
  `upool.rs` models this as `old@.free_frames.contains(uf@)` and
  `final@ == old@.alloc_one(uf@)`; the manager bridges it via
  `lemma_kernel_alloc_one` to its own partition.
- Callers assume (Err): allocator state is unchanged and the free pool was
  empty (`free_count() == 0` / `free_frames.is_empty()`); they just propagate
  the error with `?`.
- Callers don't care about: bitmap layout, which concrete frame number is
  returned, or how the refcount table is stored.

### `alloc_contiguous(count: usize) -> Result<FrameAddress, Error>`
Caller: `manager.rs:449` (`PhysMemoryManager::alloc_kernel_frames`).
- Caller assumes (Ok): `base.inv()` and, crucially,
  `base@ + count * page_size <= usize::MAX` — it relies on this to index every
  frame `base + i*PAGE_SIZE` in a loop without overflow (`lemma_contig_no_overflow`).
  All `count` frames are drawn from the free set and reserved with refcount 1.
- Caller assumes (Err): state unchanged; propagated with `?`.
- Caller requires `count > 0` (precondition).
- Caller doesn't care about: where in physical memory the run sits, only that
  it is contiguous and non-overflowing.

### `alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error>`
Caller: `mod.rs:81` (`book_physical_memory_regions`, boot path).
- Caller assumes (Ok): every frame in the region was free and is now reserved
  (`phys_view().frames.all_reserved(phys_regions_frame_set(...))`).
- Caller assumes (Err): allocator state unchanged; propagated with `?`. (Boot
  fails loudly — a region that can't be fully booked is a layout bug.)
- Caller doesn't care about: per-frame refcount values (all become 1), or the
  order frames are booked in.

### `book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>`
Caller: `mod.rs:125` (`book_mmio_regions`, boot path), always guarded by
`is_covered`.
- Caller assumes (Ok): the (covered, previously free) frame is now reserved so
  `alloc` will never hand it out. Refcount becomes 1.
- Caller assumes (Err): state unchanged; propagated with `?`.
- Caller doesn't care about how reservation is recorded.

### `is_covered(phys_addr: PageAligned<PhysicalAddress>) -> bool`
Caller: `mod.rs:124` (`book_mmio_regions`), used as a gate before `book`.
- Caller assumes: `true` iff the allocator tracks that frame (it is in the
  allocated or free set). Used to **skip** MMIO frames above RAM (e.g. LAPIC at
  `0xFEE0_0000`) that the bitmap does not cover.
- Pure query: caller assumes no state change.
- Caller doesn't care about the distinction between allocated vs free here —
  only "covered at all".

### `free(frame: FrameAddress) -> Result<(), Error>`
Callers: `manager.rs:370` and `manager.rs:485` (error-cleanup paths),
`upool.rs:202` (`UserFrame::drop`), `kframe.rs:198` (`KernelFrame::drop`).
- Callers assume: **best effort, result ignored.** Every call site discards the
  `Err` (logs a warning at most). No abstract postcondition is consumed.
- Hard constraints from the `Drop` callers: must be `no_unwind` and
  `opens_invariants none` (callable from a destructor). The free-function spec
  already reflects this (`ensures true`, `opens_invariants none`, `no_unwind`).
- Semantics callers rely on (informally, encoded in `Inner::free`): decrements
  the refcount; the bitmap bit is cleared only when the last owner releases the
  frame; double-free is rejected. This is the single complete deallocation step
  for a frame handle.
- Callers don't care about success/failure (cleanup is opportunistic).

### `free_count() -> usize`
Caller: `manager.rs:329` (watermark check before satisfying user allocations).
- Caller assumes: equals the number of free frames,
  `== phys_view().frames.free_count()`, and uses it to enforce the kernel
  watermark (`free_count() < watermark_threshold` ⇒ reject).
- Pure query: no state change.
- Caller doesn't care how the count is computed (bitmap `number_of_bits - usage`).

### `share(frame: FrameAddress) -> Result<(), Error>`
Caller: `upool.rs:164` (`UserFrame::share`, copy-on-write aliasing).
- Caller requires `frame.inv()`.
- Caller assumes (Ok): the frame is (still) allocated —
  `phys_view().frames.allocated_frames.contains(frame@)` — and a new reference
  exists; the caller mints a second `UserFrame` aliasing the same physical frame.
  A matching extra `free` must later be issued.
- Caller assumes (Err): either the frame is not allocated, or its refcount is
  already saturated at 255 (`refcounts[frame@] >= 255`).
- Caller doesn't care about the exact incremented value, only allocated-ness.

### `refcount(frame: FrameAddress) -> Result<u8, Error>`
Caller: `upool.rs:191` (`UserFrame::refcount`).
- Caller requires `frame.inv()`.
- Caller assumes (Ok): the frame is allocated and the returned `count` equals
  `phys_view().frames.refcounts[frame@]`.
- Caller assumes (Err): the frame is not currently allocated.
- Pure query: no state change.

### `init(bitmap: Bitmap) -> Result<(), Error>` *(skip/excluded)*
Caller: `mod.rs:175` (`mm::phys::init`, boot). Out of proof scope; listed in
`verus-ai-logs/tcb-allowed.md` and treated as `external_body`. Callers rely on
it establishing the singleton + `phys_view().initialized` (asserted via
`lemma_frame_initialized`) before any other free function runs.

## Abstract Resource

This module manages the **global pool of physical page frames**: a fixed set of
covered frames partitioned into *allocated* vs *free*, where each allocated frame
carries a reference count (1..=255) modelling shared ownership for copy-on-write.
Callers obtain frames (`alloc`/`alloc_contiguous`), reserve them out-of-band at
boot (`book`/`alloc_range`), share/release ownership (`share`/`free`), and query
state (`free_count`/`refcount`/`is_covered`).

## Key Invariants (caller perspective)

- **Allocated and free sets are disjoint**, and a covered frame is in exactly one
  of them; `is_covered ⇔ allocated ∨ free`.
- **`alloc`/`alloc_contiguous` only return frames that were free**, and never
  return a booked/reserved frame. Returned addresses are page-aligned, valid
  (`.inv()`), and a contiguous run does not overflow `usize::MAX`.
- **Refcount tracks ownership:** allocate/book sets it to 1, `share` increments
  (saturating at 255 → error), `free` decrements and only returns the frame to
  the free set when it reaches 0. A frame is allocated **iff** its refcount > 0.
- **`free` is total and side-effect-only from the caller's view** — safe to call
  from `Drop`, never unwinds, opens no invariants, result ignorable.
- **`free_count` faithfully reports the size of the free partition** (used to
  enforce the kernel watermark).
- **All operations preserve `FrameAllocView::wf`** (alignment, disjointness,
  allocated↔refcount consistency, refcount ≤ 255).
- **Liveness depends on `init`:** every operation panics if the singleton is
  uninitialized, so callers assume `init()` ran once at boot first.

## Pre-existing Specs (from upstream verification)

- Source: added while verifying the surrounding `mm::phys` modules (`manager`,
  `upool`, `kframe`, `mod`) top-down against this module.
- View type: **exists** — `FrameAllocView { allocated_frames, free_frames,
  refcounts }` in `mod.spec.rs`, with `FrameAllocView::wf`. `Inner`'s `View`
  impl, `Inner::inv`/`internal_inv`, and `frame_addr_of` live in
  `frame.proof.rs` / `frame.spec.rs`. (All listed as **do-not-modify**.)
- `Inner` methods (private) with **full** top-level specs (do-not-modify):
  `alloc`, `alloc_contiguous`, `free`, `share`, `refcount`, `book`, `is_covered`,
  `alloc_range`. These give precise pre/post over `self@: FrameAllocView`.
- Free functions (`pub(super)`) carry **dependency-contract** specs and are
  `external_body`: `alloc`, `alloc_contiguous`, `free_count`, `free`, `share`,
  `refcount` have `verus_spec`; `is_covered`, `book`, `alloc_range` currently
  have **no** `verus_spec` annotation (plain wrappers); `init` is excluded.

### Assessment

- **Coverage:** partial at the free-function layer. The `Inner` methods are
  fully specified, but the free-function wrappers expose only what their
  current callers needed:
  - `alloc` promises just `Ok(frame) => frame.inv()` (drops the free→allocated
    transition that `Inner::alloc` proves and that `upool::Upool::alloc` /
    `manager` re-derive via lemmas over their own partitions).
  - `free` promises `ensures true` (best-effort) — adequate for the `Drop`
    callers but conveys no state change.
  - `is_covered`, `book`, `alloc_range` have **no** wrapper spec yet, so their
    boot-path callers (`book_mmio_regions`, `book_physical_memory_regions`) are
    `external_body` and assert region-booking facts as lemmas instead.
- **Strength:** the wrapper specs are deliberately weak (they defer to
  `phys_view().frames` and to per-caller lemmas). `share`/`refcount`/`free_count`
  wrapper specs are tied to the global `phys_view().frames` partition rather than
  a local `self@`, because the singleton has no spec-readable receiver.
- **View design:** caller-abstract and stable. `FrameAllocView` names only the
  three caller-visible facts (allocated set, free set, refcounts) and hides every
  storage detail (`Bitmap`, `REFCOUNT_STORAGE`, `MaybeUninit`, `AtomicBool`).
  No field is biased toward a single caller; `refcounts` serves `share`/`refcount`
  (upool), `allocated_frames`/`free_frames` serve `alloc`/`book`/`alloc_range`
  (manager + mod), and `covers`/`reserved`/`free_count` helpers are layered on top
  in `mod.spec.rs`. This is the right caller-driven abstraction.
