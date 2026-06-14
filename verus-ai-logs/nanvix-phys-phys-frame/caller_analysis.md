# Caller Analysis: `mm::phys::frame`

## Script Output

See: `verus-ai-logs/nanvix-phys-phys-frame/find_callers_output.md`
(raw output of `find_callers_lsp.py` for `src/kernel/src/mm/phys/frame.rs`).

Summary from the script: 19 exec functions (10 `pub(super)` / trait-pub, 9
private), 1 type (`Inner`). The crate (`kernel`) has no external dependents, so
all callers are intra-crate, inside the `mm::phys` module tree. The script's
"Context" column quotes a misaligned source line; every call site below was
re-read from the actual code and corrected.

## Module Shape (validated)

`frame.rs` is a **module-level singleton**. There is no caller-visible struct:
the private `Inner` holds the `Bitmap` + `&mut [u8]` refcount table, lives in a
`static mut INSTANCE: MaybeUninit<Inner>`, and is reached only through the
private `instance()` accessor. Every `pub(super)` free function is a one-line
shim `instance().<method>(...)`. Callers therefore depend purely on the
**free-function API** and never name `Inner`, the bitmap, or the refcount array.

Internal call graph (private → reached only via its public shim):

| Public shim (`frame::*`) | Private method | Other internal callers |
|--------------------------|----------------|------------------------|
| `alloc`            | `Inner::alloc`            | — |
| `alloc_contiguous` | `Inner::alloc_contiguous` | calls `Inner::alloc_range` internally |
| `free`             | `Inner::free`             | — |
| `share`            | `Inner::share`            | — |
| `refcount`         | `Inner::refcount`         | — |
| `book`             | `Inner::book`             | — |
| `is_covered`       | `Inner::is_covered`       | — |
| `alloc_range`      | `Inner::alloc_range`      | — |
| `free_count`       | (reads `instance().bitmap`) | — |
| `init`             | (writes `INSTANCE`)       | — (Skip/excluded) |

`instance()` is the single private chokepoint (9 callers — every shim). It
`panic!`s if used before `init()`; callers therefore treat
`phys_view().initialized` as a global precondition.

## Trait Obligations

The frame functions themselves implement no trait. But the abstract resource is
consumed through two `Drop` impls, which constrain `free`:

- Trait: `Drop for UserFrame` (`upool.rs:198`) — calls `frame::free(self.addr)`,
  logs on error, never panics. Expected semantics: `free` must be callable from
  `Drop`, i.e. **`opens_invariants none` + `no_unwind`**, take no caller-side
  precondition, and preserve the subsystem invariant on every path.
- Trait: `Drop for KernelFrame` (`kframe.rs:189`) — identical contract on
  `frame::free(self.base)`.

These two `Drop`s are why `frame::free`'s shim contract carries **no `requires`**
and only `ensures phys_view().inv()` under `opens_invariants none / no_unwind`.

## Caller Expectations

### `alloc() -> Result<FrameAddress, Error>`
Callers: `Upool::alloc` (`upool.rs:268`), `PhysMemoryManager::alloc_kernel_frame`
(`manager.rs:353`), tests (`test.rs:58,88,115,144`).
- Callers assume (Ok): returned frame is page-aligned (`frame.inv()`), now in
  `allocated_frames`, with `refcounts[frame] == 1` (fresh, single owner). They
  immediately wrap it in `UserFrame`/`KernelFrame` whose `inv()` re-asserts this.
- Callers assume (Err): nothing about a returned frame; allocator unchanged and
  still `initialized` + `inv()`. (`alloc_kernel_frame` only frees on a *later*
  wrap failure, not on `alloc` failure.)
- Callers don't care about: which bit index / how the bitmap chose the frame.
- Would break callers: dropping page-alignment, handing out an already-allocated
  frame, or returning a frame not in `allocated_frames`/with refcount ≠ 1.

### `alloc_contiguous(count: usize) -> Result<FrameAddress, Error>`
Caller: `PhysMemoryManager::alloc_many_kernel_frames` (`manager.rs:426`).
- Precondition supplied by caller: `count > 0` (loop/capacity already checked).
- Callers assume (Ok): a base address such that `base + i*PAGE_SIZE`
  (`0 <= i < count`) are all freshly allocated, page-strided, single-ref frames.
  **Contiguity is load-bearing** — kernel stacks are identity-mapped and the
  hardware SP walks the run linearly.
- Callers assume (Err): allocator unchanged. The caller's own rollback
  (`frame::free` of each slot) assumes the run was either fully reserved or not.
- Don't care about: placement strategy. Would break callers: a non-contiguous or
  partially-reserved run on `Ok`.
- **Note:** the `frame::alloc_contiguous` free-function shim currently has *no*
  `#[verus_spec]` (only `Inner::alloc_contiguous` is specced). The caller
  (`alloc_many_kernel_frames`) is itself `external_body`, so it states the
  contiguity guarantee directly over `phys_view()` rather than via the shim.

### `free_count() -> usize`
Caller: `PhysMemoryManager::check_user_watermark` (`manager.rs:314`).
- Callers assume: returns the number of currently-free frames, used purely as a
  watermark gate (`free_count() < KERNEL_WATERMARK + count` ⇒ reject). The
  abstract spec the caller relies on is `spec_watermark_ok(phys_view().frames,
  count)` with `free_frames.finite()`.
- Don't care about: exact value beyond the comparison; no state change expected
  (pure query).
- **Note:** `frame::free_count` free-function shim has *no* `#[verus_spec]`; the
  watermark guarantee is expressed in the (external_body) `check_user_watermark`
  contract over `phys_view()`.

### `free(frame: FrameAddress) -> Result<(), Error>`
Callers: `UserFrame::Drop` (`upool.rs:199`), `KernelFrame::Drop` (`kframe.rs:190`),
`alloc_kernel_frame` rollback (`manager.rs:356`),
`alloc_many_kernel_frames` rollback (`manager.rs:439`), tests
(`test.rs:99,128,183`).
- Callers assume (always): subsystem invariant preserved; safe from `Drop`
  (`no_unwind`, `opens_invariants none`); errors are *returned values*, never
  panics/unwinds — every caller logs and swallows them.
- Refcount semantics callers rely on (from `Inner::free`): last reference returns
  the frame to `free_frames`; a shared frame just decrements. Tests assert the
  consequence: a leaked-then-freed frame succeeds once; a double-free fails.
- Callers don't care about: the precise refcount value — the shim deliberately
  cannot express the transition (no `old(phys_view())`).
- Would break callers: a `free` that panics/unwinds (breaks `Drop`), or that
  fails to eventually return a fully-released frame to the free pool.

### `is_covered(phys_addr: PageAligned<PhysicalAddress>) -> bool`
Caller: `book_mmio_regions` (`mod.rs:138`).
- Callers assume: `true` ⟺ the address is tracked by the allocator, i.e. in
  `phys_view().covered()` (allocated ∪ free). Used to **skip** MMIO frames above
  RAM (e.g. LAPIC at `0xFEE0_0000`) before calling `book`.
- Precondition supplied: `phys_addr.inv()`; allocator `initialized`.
- Don't care about: bitmap size details. Would break callers: reporting `true`
  for an untracked address (would make the following `book` fail and abort boot).

### `book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>`
Caller: `book_mmio_regions` (`mod.rs:139`, guarded by `is_covered`).
- Callers assume (Ok): the frame is now reserved (`allocated_frames` contains it);
  `alloc` will never hand it out. Allocator stays `initialized` + `inv()`.
- Callers assume (Err): allocator unchanged; boot may propagate the error.
- Don't care about: refcount bookkeeping (booked frames get refcount 1 internally).

### `alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error>`
Caller: `book_physical_memory_regions` (`mod.rs:90`).
- Callers assume (Ok): **every** frame of the region is reserved
  (`region_frames(start,size) ⊆ allocated_frames`). Used at boot to fence off
  unusable physical memory before any `alloc`.
- Callers assume (Err): allocator unchanged; the region was not fully free, so
  the error surfaces a memory-layout bug.
- Don't care about: per-frame iteration order. The caller is `external_body`
  (std `LinkedList` iteration is unverifiable), so it relies on this set-level
  contract, discharged by `lemma_book_region_reserves_region_frames`.

### `share(frame: FrameAddress) -> Result<(), Error>`
Caller: `UserFrame::share` (`upool.rs:145`).
- Callers assume (Ok): frame remains allocated and has gained a reference;
  `UserFrame::share` then returns a second handle aliasing the same frame
  (`handle@ == self@`). This is the copy-on-write building block.
- Callers assume (Err): no reference acquired; `self` untouched. Failure means
  the frame was not allocated, or refcount would overflow 255.
- Don't care about: the new exact count (shim can't state the increment).
- Would break callers: incrementing a free frame, or losing the
  "still allocated after Ok" guarantee.

### `refcount(frame: FrameAddress) -> Result<u8, Error>`
Caller: `UserFrame::refcount` (`upool.rs:181`).
- Callers assume (Ok): a pure query — frame is allocated and the returned `u8`
  equals `refcounts[frame]`; no state change.
- Callers assume (Err): frame is not allocated.
- Don't care about: anything beyond the count; must not mutate.

### `init(bitmap: Bitmap) -> Result<(), Error>` — **Skip/excluded**
Caller: `mm::phys::initialize` (`mod.rs:190`). Listed per `tcb-allowed.md`
(Skip/exclude). Establishes the singleton; on `Ok` the allocator is
`initialized` and `inv()` holds — this is the global precondition every other
`frame::*` call requires. Not a proof target here.

## Abstract Resource

`frame` manages **the system's single global pool of physical frames**: a fixed
universe of page-aligned physical addresses partitioned into *allocated* vs
*free*, with a per-frame *reference count* layered on top so a frame can be
shared (copy-on-write) and is only returned to the free pool when its last
reference is released. Callers see one process-wide allocator reached through
free functions; they never see the backing bitmap or refcount table.

Key operations on that resource:
- Reserve: `alloc` (one), `alloc_contiguous` (a contiguous run),
  `book` / `alloc_range` (boot-time reservation of known addresses/regions).
- Reference: `share` (+1), `free` (−1, release on reaching 0).
- Query: `refcount` (one frame's count), `free_count` (free-pool size),
  `is_covered` (is an address tracked at all).
- Lifecycle: `init` (one-shot, establishes the invariant).

## Key Invariants (caller perspective)

- **Initialized-before-use:** every operation except `init` requires
  `phys_view().initialized`; `instance()` panics otherwise.
- **Well-formedness preserved everywhere:** `phys_view().inv()`
  (`FrameAllocView::wf`) holds before and after every operation — including the
  `free` path invoked from `Drop`.
- **Partition:** `allocated_frames` and `free_frames` are disjoint; all addresses
  in both are page-aligned (`addr % spec_page_size() == 0`).
- **Refcount coupling:** `allocated_frames.contains(a)` ⟺ `refcounts[a] > 0`;
  free frames have no refcount entry; `0 < refcount[a] <= 255`.
- **Fresh-allocation shape:** `alloc`/`alloc_contiguous`/`book` produce frames
  with refcount exactly 1.
- **Release monotonicity:** `free` decrements; only the last reference moves a
  frame from `allocated_frames` to `free_frames`. Double-free is rejected.
- **Drop-safety:** `free` is `no_unwind` + `opens_invariants none` and never
  requires a precondition, so the two `Drop` impls are sound.
- **Coverage gate:** `is_covered(a)` ⟺ `a ∈ covered() = allocated ∪ free`;
  callers rely on this to skip untracked MMIO frames before `book`.

## Pre-existing Specs (from upstream verification)

- Source: added while verifying the surrounding `mm::phys` subsystem
  (`mod` / `manager` / `upool` / `kframe`), which consume `frame::*`.
- View type: **exists** — `FrameAllocView` (defined in `mod.spec.rs`, do-not-
  modify) with `allocated_frames`, `free_frames`, `refcounts`, and `wf()`; the
  `View for Inner` impl + `Inner::inv` live in `frame.proof.rs`/`frame.spec.rs`.
  A module-wide `phys_view() -> PhysMemView` wraps it for the free-function
  shims.
- Functions WITH top-level specs (do-not-modify): `Inner::alloc`,
  `Inner::alloc_contiguous`, `Inner::free`, `Inner::share`, `Inner::refcount`,
  `Inner::book`, `Inner::is_covered`, `Inner::alloc_range` (rich per-`Inner@`
  transition contracts), plus the free-function shims `alloc`, `free`,
  `is_covered`, `book`, `alloc_range`, `share`, `refcount`, and `init` (stated
  over `phys_view()`).
- Functions WITHOUT specs: the **`alloc_contiguous` and `free_count`
  free-function shims**, and the private `instance()`. (Their `Inner`
  counterparts / consumers carry the relevant contracts.)

### Assessment
- Coverage: **partial.** The `Inner::*` methods are fully specced and the most
  load-bearing shims are too, but the `alloc_contiguous` and `free_count` shims
  are unspecced — their only consumers (`alloc_many_kernel_frames`,
  `check_user_watermark`) are `external_body` and re-state the guarantees over
  `phys_view()` directly.
- Strength: **adequate for `Inner`, deliberately weak for shims.** Shim
  `ensures` are monotone post-state facts (e.g. "frame is now in
  `allocated_frames`") rather than full transitions, because `phys_view()` is a
  single fixed value with no `old(phys_view())` to diff against. This matches
  every documented caller need (especially the `Drop`-driven `free`).
- View design: **caller-abstract.** `FrameAllocView` exposes exactly the
  set/map abstractions callers reason about (allocated/free sets + refcount map)
  and hides the bitmap and BSS refcount array. `covered()`, `region_frames()`,
  and `spec_watermark_ok()` derive the higher-level predicates that `mod` and
  `manager` actually consume — no field is biased toward a single caller.
