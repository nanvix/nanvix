# TCB Allowed List — Nanvix phys-mm

Any `external_body` outside this list must be removed.

## Allowed `external_body`

- `src/kernel/src/mm/phys/frame.rs::instance` — materializes a `&'static mut Inner` from the
  module-level `static mut INSTANCE: MaybeUninit<Inner>` singleton (guarded by `INSTANCE_INIT`).
  Raw-memory op over externally-owned `static mut` storage Verus cannot verify without a
  `PointsTo` (mirrors `bump_allocator`/`raw-array`). `ensures` pins the singleton's abstract
  state to `phys_view().frames`, asserts `(*r).inv()`, and records `phys_view().initialized` —
  the §8 ghost-token attachment for the singleton frame allocator.
- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::deref`
- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::deref_mut`
- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::clear`
- `src/libs/bump_allocator/src/lib.rs::FixedSizeBumpAllocator::alloc` — materializes a
  `&'static mut [u8; N]` from a backend-provided address (`usize as *mut`); raw-memory
  op Verus cannot verify without a `PointsTo` for the externally-owned `BssStorage`
  region. Mirrors `src/libs/raw-array`. `ensures` states alignment + in-bounds over
  the abstract `bump_view`.
- `src/libs/bump_allocator/src/lib.rs::FixedSizeBumpAllocator::alloc_as` — delegates to
  `alloc` and re-materializes the slot as `&'static mut MaybeUninit<T>`; same rationale.
  `ensures` adds the `size_of::<T>()`/`align_of::<T>()` vs `(N, A)` guard arms.

## `external_body` introduced while speccing `arch::x86::mem::paging::table`

- `src/libs/arch/src/x86/mem/paging/table.rs::Table::<E>::read` — reads a `TableEntry` from a
  page-table slot through a raw pointer materialized from `self.base` (`usize as *const E::Word`).
  Verus does not support `usize`→pointer casts (int-to-ptr materialization of externally-owned,
  volatile page-table memory), so the body cannot be verified. Same trust-boundary rationale as
  `bump_allocator::alloc` (`usize as *mut`) and `frame::instance`. See
  `nanvix-phys-arch-paging-table/verus-unsupported.md`. **The function is not contract-free:** it
  carries a full `#[verus_spec]` pinned to the global page-table-memory ghost — `requires index@ <
  PAGE_TABLE_LENGTH`, `ensures result == spec_table_read::<E>(self@.addr, index@)` — exactly as
  `frame::instance` pins its result to `phys_view()` (parameter-free global ghost, no signature
  change).
- `src/libs/arch/src/x86/mem/paging/table.rs::Table::<E>::write` — writes a `TableEntry` into a
  page-table slot through a raw pointer materialized from `self.base` (`usize as *mut E::Word`).
  Same int-to-ptr / volatile externally-owned-memory limitation as `read`. Carries only the sound
  `requires index@ < PAGE_TABLE_LENGTH` (auto from `TableIndex::inv`); it has **no** contents
  `ensures`. A contents postcondition pinning the *pure* global ghost `spec_table_word(self@.addr,
  index@)` to the caller-chosen `entry` would be unsound for an `external_body` (assumed)
  contract: two writes of distinct entries to the same slot would assume
  `spec_entry_raw(e1) == spec_entry_raw(e2)`, which with `lemma_entry_roundtrip` derives
  `e1 == e2`, i.e. `false` (Turn 2 review #2/#3/#15; exploit reproduced). The genuine `old@ -> @`
  slot-update transition (`self@.entries[index@] == Some(entry)`, other slots framed) is therefore
  **deferred to the proving-phase page-table permission token**, exactly the `identity_map_view()`
  `v -> v'` deferral convention in `identity_map.spec.rs`.

## `external_body` introduced while speccing `arch::x86::mem::paging` (`mod.rs`)

- `src/libs/arch/src/x86/mem/paging/mod.rs::invlpg` — the body is a single
  `core::arch::asm!` block issuing the `invlpg` instruction, which flushes the CPU TLB
  entry for `vaddr`. Verus does not support inline-asm expressions, so the body cannot be
  verified — an external-bottom hardware trust boundary (same class as the volatile
  page-table access in `table::read`/`write` and the int-to-pointer materialization in
  `frame::instance`/`bump_allocator::alloc`). See
  `nanvix-phys-arch-paging-mod/verus-unsupported.md`. The effect is purely on hardware TLB
  state (outside Verus' memory model and invisible to every caller's Rust-visible state),
  so the faithful contract is **empty**: no `requires` (any `usize` is accepted), trivial
  `ensures` (returns `()`, no Rust-visible effect, every caller-side invariant preserved).
  This matches the inherited upstream
  `src/kernel/src/mm/virt/identity_map.spec.rs:151`
  `pub assume_specification[ ::arch::mem::paging::invlpg ](vaddr: usize);` (no
  `requires`/`ensures`). No exec signature changed.

## Skip / exclude from current proof target

- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::get_mut`
- `src/kernel/src/mm/phys/frame.rs::init`

## `external_body` introduced while speccing `mm::phys`

- `src/kernel/src/mm/phys/mod.rs::book_physical_memory_regions` — iterates an
  `alloc::collections::LinkedList` in a `for` loop. Verus has no `LinkedList` model and the
  orphan rule blocks providing one from the kernel crate (see
  `nanvix-phys-phys-mod/bugs.md`). Body cannot be verified; `ensures` states that, on `Ok`,
  every frame in `phys_regions_frame_set(&physical_memory_regions)` becomes reserved.
- `src/kernel/src/mm/phys/mod.rs::book_mmio_regions` — same `LinkedList` limitation.
  `ensures` states that, on `Ok`, every *covered* frame in `mmio_regions_frame_set(mmio_regions)`
  becomes reserved (uncovered MMIO frames are skipped, matching the `frame::is_covered` gate).

## Cross-module dependencies marked `external_body` (eliminated when their module is verified)

- `src/kernel/src/mm/phys/frame.rs::init` — also listed under skip; callable from verified `init`.
- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::init` — no specs yet; opaque callee.
- `src/kernel/src/mm/phys/upool.rs::Upool` (struct) and `Upool::new` — no specs yet; opaque
  type/callee needed so verified `init` can construct the user page pool.
- `src/kernel/src/mm/phys/upool.rs::Upool::alloc` — pool allocation primitive the manager's user
  paths call. `ensures` describes the free→allocated transition (`alloc_one`) and the empty-pool
  `Err` arm (`free_count() == 0`). Verified when `upool` is.
- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::new` — wraps a `FrameAddress` into an owning
  kernel-frame handle. `ensures Ok(kf) => kf@ == base@`. Verified when `kframe` is.
- `src/kernel/src/mm/phys/frame.rs::alloc` — singleton wrapper around `Inner::alloc`;
  `ensures Ok(frame) => frame.inv()`.
- `src/kernel/src/mm/phys/frame.rs::alloc_contiguous` — singleton wrapper around
  `Inner::alloc_contiguous`; `requires count > 0`, `ensures` page-aligned base plus the
  address-space range bound the manager's index arithmetic relies upon.
- `src/kernel/src/mm/phys/frame.rs::free_count` — reports the free-partition size
  (`== phys_view().frames.free_count()`).
- `src/kernel/src/mm/phys/frame.rs::free` — best-effort frame release used by manager error
  cleanup; no precondition, no abstract postcondition (callers ignore the outcome).
- `src/kernel/src/mm/phys/frame.rs::share` — singleton wrapper around `Inner::share` (the CoW
  reference-count bump). `requires frame.inv()`, `ensures Ok(()) => the frame is allocated in
  `phys_view().frames``. The per-frame `+1` lives in the global partition and is pinned to
  `phys_view().frames` in the proving phase. Sibling of `frame::alloc`/`free`; `external_body`
  until the free-function layer is verified.
- `src/kernel/src/mm/phys/frame.rs::refcount` — singleton wrapper around `Inner::refcount`.
  `requires frame.inv()`, `ensures Ok(count) => count == phys_view().frames.refcounts[frame@]`
  and `Err(_) => frame not allocated`. Pure read; `external_body` until the free-function layer
  is verified.
- `src/kernel/src/hal/mem/types/address/frame.rs::FrameAddress::from_raw_value` — succeeds only
  for page-aligned inputs, so `ensures Ok(fa) => fa.inv()`. Verified when the address layer is.
- `src/kernel/src/hal/mem/types/address/frame.rs::FrameAddress::into_raw_value` — the raw value is
  the abstract frame address (`ensures result as int == self@`). Verified when the address layer is.

## External-bottom: build-time constant accessor

- `src/kernel/src/mm/phys/manager.rs::kernel_watermark` — `external_body` accessor that returns
  the build-time constant `config::kernel::KERNEL_WATERMARK`. This constant is generated by the
  `config` crate's `build.rs` (from `kernel_config.toml`) and lives in a non-Verus dependency
  crate, so Verus cannot resolve its value. The accessor's `ensures ret as nat ==
  spec_kernel_watermark()` ties the runtime value to the abstract spec value. The companion spec
  function `spec_kernel_watermark()` (in `manager.spec.rs`) is therefore `uninterp` — a mechanical
  consequence of this external-bottom boundary, not a verification escape: the watermark threshold
  is genuinely outside Verus's view and callers do not depend on its concrete value (only on the
  fact that user allocations are gated by it; see `caller_analysis.md` line 121).

## `assume_specification` for not-yet-verified callees (eliminated when their module is verified)

These `assume_specification` (and one `external_type_specification`) declarations live in
`src/kernel/src/mm/phys/frame.spec.rs`. They give trusted contracts to callees of the (now
verified-in-body) `Inner` frame-allocator methods so those bodies translate. Each is superseded
by the real specification when its module is verified — the same "superseded when the address
layer is verified" rationale already used for `FrameAddress::from_raw_value`/`into_raw_value`.

- **External crate (`arch`) — genuinely outside the kernel crate, temporary placeholder:**
  - `::arch::mem::paging::FrameNumber` — `external_type_specification` (`ExFrameNumber`).
  - `::arch::mem::FRAME_SIZE` — `ensures result == spec_page_size()`.
  - `::arch::mem::paging::FrameNumber::from_raw_value` / `into_raw_value`.

- **Intra-crate (`kernel` crate `hal::mem::*`) — recorded here per review item #7. These are
  workspace-internal and not external dependencies; they are trusted only until the HAL
  address/region layer is verified, at which point these `assume_specification`s are removed:**
  - `crate::hal::mem::FrameAddress::from_frame_number` / `into_frame_number`
  - `crate::hal::mem::PhysicalAddress::into_frame_number`
  - `crate::hal::mem::PageAligned::<T> as Address::into_raw_value`
  - `crate::hal::mem::PageAligned::<T> as Deref::deref`

  (`crate::hal::mem::TruncatedMemoryRegion::<T>::start` / `size` were removed from
  `frame.spec.rs` once the `hal::mem::types::region` module gained real specifications —
  their real `#[verus_spec]` contracts now supersede the placeholders.)

- **Intra-crate placeholder in `hal::mem::types::address::frame.spec.rs` (bottom-up
  proving of `hal::mem::types::address::frame`):**
  - `src/kernel/src/hal/mem/types/address/frame.spec.rs::<PhysicalAddress as
    ::sys::mm::Address>::from_raw_value` — `ensures Ok(pa) => pa@ == value as int`,
    `Err(_) => true`. `PhysicalAddress` is the kernel-internal type in the sibling
    `hal::mem::types::address::phys` module; its `Address::from_raw_value` body
    (`phys.rs:185`) currently carries **no** `#[verus_spec]` (verified: `grep -n
    verus_spec` over `phys.rs:168-187` is empty). The verified
    `FrameAddress::from_raw_value` postcondition (`Ok(fa) => fa@ == raw_addr as int`)
    depends on this raw-value contract, so the placeholder is genuinely required for
    bottom-up verification of the frame module and cannot be deleted. It is trusted only
    until the HAL address layer (`phys`) is verified, at which point `phys.rs`'s
    `from_raw_value` gains its own `#[verus_spec]` and this `assume_specification` is
    removed — the same "superseded when the address layer is verified" rationale used for
    `FrameAddress::from_raw_value`/`into_raw_value` above.
