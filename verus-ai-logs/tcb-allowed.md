# TCB Allowed List — Nanvix phys-mm

Any `external_body` outside this list must be removed.

## Allowed `external_body`

- `src/kernel/src/mm/phys/mod.rs::book_physical_memory_regions` — iterates a std
  `alloc::collections::LinkedList` via `for region in list.iter()`. The build
  toolchain's `vstd` (`/mnt/toolchain/verus/vstd`) ships ghost-iterator specs only
  for `slice`/`vec`/`VecDeque`/`BTree*`/`Hash*` iterators, **not** `LinkedList`, and
  the orphan rule forbids supplying `View`/`ForLoopGhostIterator` impls for the
  foreign `linked_list::Iter` from the `kernel` crate (E0117). Genuine Verus
  front-end limitation, not a proof gap or code bug — full analysis in
  `verus-unsupported.md`. The abstract booking effect is specified over
  `PhysMemView` and discharged (no `admit`) by
  `lemma_book_region_reserves_region_frames` in `mod.proof.rs`. `ensures` keeps the
  caller-relevant guarantee: allocator stays initialized and well-formed.
- `src/kernel/src/mm/phys/mod.rs::book_mmio_regions` — same std-`LinkedList`
  iteration limitation as above. Abstract effect (skip-if-not-covered; book tracked
  frames) discharged by `lemma_book_mmio_skip_untracked` /
  `lemma_book_mmio_books_tracked`. `ensures` keeps allocator initialized and
  well-formed across booking.
- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::deref`
- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::deref_mut`
- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::clear`- `src/libs/bump_allocator/src/lib.rs::FixedSizeBumpAllocator::alloc` — materializes a
  `&'static mut [u8; N]` from a backend-provided address (`usize as *mut`); raw-memory
  op Verus cannot verify without a `PointsTo` for the externally-owned `BssStorage`
  region. Mirrors `src/libs/raw-array`. `ensures` states alignment + in-bounds over
  the abstract `bump_view`.
- `src/libs/bump_allocator/src/lib.rs::FixedSizeBumpAllocator::alloc_as` — delegates to
  `alloc` and re-materializes the slot as `&'static mut MaybeUninit<T>`; same rationale.
  `ensures` adds the `size_of::<T>()`/`align_of::<T>()` vs `(N, A)` guard arms.

## Skip / exclude from current proof target

- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::get_mut`
- `src/kernel/src/mm/phys/frame.rs::init`

## Allowed `external_body` — frame allocator core (`frame.rs`)

`frame.rs` is now the proof target. Two layers form the trust boundary of the
global frame-allocator singleton; everything else in the module (the `frame::*`
free-function shims) is **body-verified** against these contracts.

### 1. The singleton bridge: `instance()`

`instance()` returns `&'static mut Inner` obtained from the module-level
`static mut INSTANCE` via `unsafe { INSTANCE.assume_init_mut() }`. The verifier
does not support `static mut` paths, and a zero-argument uninterpreted
`phys_view()` cannot be tied to that static by any verifiable means. `instance()`
is therefore the bridge axiom: `requires phys_view().initialized` (the real panic
gate) and `ensures (*result).inv() && (*result)@ == phys_view().frames`. This is
what lets the query shims (`is_covered`, `refcount`) be fully body-verified.

- `src/kernel/src/mm/phys/frame.rs::instance` — `static mut` singleton accessor;
  bridges the live `Inner` to the abstract `phys_view().frames`.

### 2. The `Inner::*` methods

Each `Inner::*` method has a rich, do-not-modify `old(self)@ → final(self)@`
transition contract, but its **body** cannot be translated by Verus: every method
uses the `error!` / `debug_assert_eq!` macros (whose expansion needs
`core::fmt::Arguments`, unsupported — the same limitation already recorded for
`UserFrame::drop`) and the `arch` newtypes `FrameNumber` / `FrameAddress`
(external types with no `external_type_specification`). They are `external_body`
with their pre-existing `#[verus_spec]` contracts honored as trust boundaries.

- `Inner::alloc`, `Inner::alloc_contiguous`, `Inner::free`, `Inner::share`,
  `Inner::refcount`, `Inner::book`, `Inner::is_covered`, `Inner::alloc_range`
  (all in `src/kernel/src/mm/phys/frame.rs`).

### Note on the body-verified shims

The `frame::*` free-function shims are no longer `external_body`. The pure-query
shims `is_covered` and `refcount` are proven outright via the `instance()` bridge.
The aggregate query shim `free_count` is also fully body-verified via
`lemma_free_count` (`frame.proof.rs`): it counts the clear bitmap indices as the
image of an injective `i -> i * page_size` map and discharges
`free_frames.len() + usage == num_bits` without any `admit()`.

The mutating reservation shims (`alloc`, `alloc_contiguous`, `book`,
`alloc_range`, `share`) are body-verified and carry the **strong** post-state
contract via the `tracked PhysAuth` carrier threaded through `&mut`: each names
both `old(auth)@` (pre-state) and `final(auth)@` (post-state) and asserts the
exact `FrameAllocView` transition (`spec_alloc_one` / `spec_alloc_set` /
`spec_share`), with a meaningful `Err` arm (`final(auth)@ == old(auth)@`). They
carry no `admit()`.

## Allowed `external_body` — `frame::free` (`frame.rs`, `Drop` path)

`frame::free` is reached only from `UserFrame::drop` / `KernelFrame::drop`. Their
trait-fixed `drop(&mut self)` signature is `opens_invariants none` + `no_unwind`,
so the `Drop` path **cannot** receive a `Tracked<&mut PhysAuth>` carrier and
**cannot** open a global invariant to reach the authority. Without the token the
body cannot discharge `instance()`'s `phys_view().initialized` precondition nor
`Inner::free`'s `frame.inv()` precondition, and the contract must stay
precondition-free for `Drop` soundness. The function is therefore `external_body`
with its weak, always-true `ensures phys_view().inv()` honored as a trust boundary
(the logging branch performs no state change). This replaces the previous deferred
`proof! { admit(); }`; the trust surface is identical, but `external_body` is the
governed mechanism. It is the single deliberate, caller-justified `Drop`-path
exception — **not** the forbidden weakening, which concerned the reservation ops
that now carry the strong `PhysAuth`-threaded post-state.

- `src/kernel/src/mm/phys/frame.rs::free` — releases one reference on `Drop`.
  `ensures phys_view().inv()` on every path; no precondition (`Drop`-safe).

## Allowed `external_body` — `UserFrame::drop` (`upool.rs`)

`UserFrame::drop` releases the frame's reference by calling `frame::free` and, on
the error path, logs via the `error!` macro. The `error!`/`write!` expansion uses
`core::fmt` Debug formatting (`{:?}`), which Verus cannot translate to VIR
("Unsupported constant type"). The function is therefore `external_body`; its
`#[verus_spec]` contract is honored by trusting that body. The contract is not a
new guarantee — `ensures phys_view().inv()` is exactly the post-state already
established by the `external_body` `frame::free` shim it calls, and the logging
branch performs no state change. `opens_invariants none` / `no_unwind` record that
`drop` opens no invariant and cannot unwind (errors are logged, not propagated).

- `src/kernel/src/mm/phys/upool.rs::<UserFrame as Drop>::drop` — releases one
  reference on scope exit. `ensures phys_view().inv()` on every path.

## Allowed `external_body` — `PhysMemoryManager` (`manager.rs`)

`PhysMemoryManager` is a **stateless facade** over the global frame allocator
(`Upool` has no fields; kernel frames are drawn from the global `frame::*`
statics through `static mut PHYS_MEMORY_MANAGER`). Its target methods therefore
have no verifiable body: they mutate global `static mut` state, call un-specced
upstream allocator primitives, and use side-effecting combinators
(`inspect_err`/`and_then`/`ok_or_else`) and `error!`/`warn!` macros that are not
ghost-gated and have no `vstd` specs. They form a trust boundary identical in
character to the `frame.rs` shims: each is `external_body` with a `#[verus_spec]`
contract stated over the do-not-modify `phys_view()` / `FrameAllocView` and the
returned frame values (monotone post-state facts, as there is no
`old(phys_view())`). Abstract laws are carried by `manager.proof.rs` lemmas.

- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::init` — writes the
  global `static mut PHYS_MEMORY_MANAGER` (`MaybeUninit::write`) and flips an
  `AtomicBool`. `ensures` keeps the allocator initialized and well-formed.
- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::alloc_user_frame` —
  draws a frame from the global allocator and wraps it as a `UserFrame`.
  `ensures` (on success) places the returned frame's address in
  `allocated_frames` and states page alignment.
- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::check_user_watermark` —
  reads the build-time `config::kernel::KERNEL_WATERMARK` and the global free
  count. `ensures` (on success) gives `spec_watermark_ok` and keeps
  `free_frames` finite.
- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::alloc_many_user_frames`
  — loop-allocates `count` user frames into a caller `&mut Vec`, with all-or-
  nothing cleanup. `ensures` gives `len == count` and each frame allocated on
  success, empty vector on error.
- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::alloc_kernel_frame` —
  like `alloc_user_frame` but yields a `KernelFrame`.
- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::alloc_many_kernel_frames`
  — like `alloc_many_user_frames` plus the contiguity guarantee
  (`kernel_frames_contiguous`) required for kernel stacks.

## Allowed `external_body` — not-yet-verified dependencies of `mm::phys::kframe`

`KernelFrame::new` is the first verified (`#[verus_spec]`) exec body in the proof
target that calls cross-module address-conversion / mapping helpers. Those helpers
live in `hal::mem` and `mm::virt`, which are **not yet verified**, so they are
declared outside any `verus!` macro and Verus would otherwise reject the call.
Each is given a temporary `external_body` `#[verus_spec]` contract so the verified
`new` body type-checks; the contracts are removed when their home modules are
verified. None weakens an existing guarantee — they only name facts `new` already
relies on (newtype address identity) or state nothing observable (the
identity-map side effect, which no `mm::phys` contract names).

- `src/kernel/src/hal/mem/types/address/frame.rs::FrameAddress::into_raw_value` —
  returns the frame's raw `usize` address; delegates through the `sys::mm::Address`
  trait. `ensures result as int == self@` (the newtype identity already assumed by
  `manager.rs`'s `kernel_frames_contiguous` reasoning). Removed when `hal::mem` is
  verified.
- `src/kernel/src/mm/virt/identity_map.rs::identity_map_page` — lazily installs a
  kernel identity-map PTE (page tables drawn from a BSS pool; no recursive frame
  allocation). The mapping side effect is not part of the physical-frame
  abstraction, so the contract states nothing abstract (`ensures true`). Removed
  when `mm::virt` is verified.

## Allowed `assume_specification` — `sys::mm::Address` trait method

`KernelFrame::new` also calls `<PageAligned<PhysicalAddress> as Address>::from_raw_value`,
a method of the external **`sys::mm::Address`** trait. A trait-impl method cannot be
given a standalone `external_body` contract without marking the whole trait `impl`
block verified (which would pull every sibling method into scope), so it is specced
with `assume_specification` in `kframe.spec.rs`. This mirrors the existing
`assume_specification` trust boundaries the codebase already draws at the
`sys`/`arch` library edge (`::arch::mem::PAGE_SIZE` in `frame.rs`, `Error::new` in
`libs/error`). The contract is trivial (the returned address value is not consumed
by any `mm::phys` fact); it is removed when `hal::mem` / the `Address` trait are
verified.

- `<crate::hal::mem::PageAligned<T> as crate::hal::mem::Address>::from_raw_value`
  (declared in `src/kernel/src/mm/phys/kframe.spec.rs`).

## Allowed `external_body` — `hal::mem::PageAligned` (`page.rs`, proof target)

`PageAligned<T>` is the proof target of the `hal::mem::types::address::aligned::page`
module. Its in-scope constructor cannot be body-verified in place: its body checks
page alignment via `<T as Address>::is_aligned(PAGE_ALIGNMENT)`, where the `Address`
trait method is unspecced and `PAGE_ALIGNMENT` is an `arch` `Alignment` enum constant
the Verus front-end cannot translate (`error: arch::x86::mem::constants::PAGE_ALIGNMENT
is not supported`). Both live in the not-yet-verified `sys`/`arch` libraries. The
function is therefore `external_body` with its `#[verus_spec]` contract honored as a
trust boundary — the same pattern as `FrameAddress::into_raw_value`. The contract is
the real caller guarantee (newtype identity + page-alignment invariant), not a
weakening; it is discharged when the `Address` trait and the `Alignment` encoding are
verified.

- `src/kernel/src/hal/mem/types/address/aligned/page.rs::PageAligned::from_address` —
  validating, identity-preserving constructor. On success `result@ == spec_addr(&addr)`
  and `result.inv()`; on failure `spec_addr(&addr) % spec_page_size() != 0`; success
  holds iff the input address is page-aligned. Removed when `sys::mm::Address` /
  `Alignment` are verified.

## Allowed `assume_specification` — `hal::mem::PageAligned` `Address::into_raw_value`

`<PageAligned<T> as Address>::into_raw_value` is a method of the external
**`sys::mm::Address`** trait. A per-method `external_body`/`#[verus_spec]` would
require marking the whole `impl Address for PageAligned<T>` verified, which currently
triggers a Verus front-end panic (`vir/src/traits.rs:511 assertion failed:
!method_impls.contains(&p)`) on this generic trait impl. It is therefore specced with
`assume_specification` in `page.spec.rs`, mirroring the existing
`<PageAligned<T> as Address>::from_raw_value` trust boundary (`kframe.spec.rs`) and the
`::arch::mem::PAGE_SIZE` boundary (`frame.rs`). The `@`-based contract is expressible
because `PageAligned<T>: View` now holds for every `T: Address` (unconditional `View`
delegating to the ghost `spec_addr`). The real obligation is the inner
`<T as Address>::into_raw_value` newtype identity, discharged when the `Address` trait
is verified.

- `<crate::hal::mem::PageAligned<T> as crate::hal::mem::Address>::into_raw_value`
  (declared in `src/kernel/src/hal/mem/types/address/aligned/page.spec.rs`).
  `ensures result as int == addr@`.

## Allowed `external_type_specification` — `hal::mem::types::address::phys` (proof target)

`PhysicalAddress` is the proof target of the `hal::mem::types::address::phys`
module. It projects to / from the `arch` crate's `FrameNumber`, a foreign type
from a non-Verus-enabled crate with no `View`/datatype registration reachable
here (orphan rule). To let `FrameNumber` appear in spec-fn parameters and the
library-edge `assume_specification`s, it is registered as an opaque external
datatype via `external_type_specification` + `external_body` — the exact idiom
already approved for `ExLinkedList` (`mm/phys/mod.spec.rs`). Its internals are not
modeled; its abstract index is projected by the uninterpreted
`spec_frame_raw_value`.

- `src/kernel/src/hal/mem/types/address/phys.spec.rs::ExFrameNumber` —
  `external_type_specification` wrapper registering the foreign `arch::FrameNumber`
  as opaque. No body verified; carries no abstract guarantee beyond opacity.

## Allowed `assume_specification` — `hal::mem::types::address::phys` library edge

`PhysicalAddress`'s verified constructors/projections call `arch`/`sys`
library-edge items whose home crates are not yet Verus-enabled. Each is specced
with `assume_specification` in `phys.spec.rs`, mirroring the existing
`::arch::mem::PAGE_SIZE` (`frame.rs`) and `sys::mm::Address`
(`kframe.spec.rs` / `page.spec.rs`) boundaries the codebase already draws. The
contracts state only the load-bearing newtype-identity / positivity / range facts
the proofs rely on; they introduce no new observable guarantee and are removed
when `arch`/`sys` are verified.

- `::arch::mem::FRAME_SIZE` — `result == spec_page_size() && spec_page_size() > 0`.
- `::arch::mem::FRAME_SHIFT` — `result < usize::BITS && pow2(result) == spec_page_size()`.
- `VirtualAddress::new` — `result@ == value as int` (newtype constructor).
- `<VirtualAddress as Address>::into_raw_value` — `result as int == addr@`.
- `FrameNumber::into_raw_value` — projects index in `0 ..= spec_max_frame_number()`.
- `FrameNumber::from_raw_value` — `Some` iff `value <= spec_max_frame_number()`,
  preserving the index; `None` otherwise.
  (all declared in `src/kernel/src/hal/mem/types/address/phys.spec.rs`.)
