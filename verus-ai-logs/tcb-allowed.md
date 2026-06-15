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

- `src/kernel/src/mm/phys/mod.spec.rs::ExLinkedList` — `external_type_specification` (with the
  mandatory `external_body`) registering the foreign `alloc::collections::LinkedList<T, A>` as a
  Verus-visible opaque type. Verus ships no `LinkedList` model and the orphan rule forbids a
  downstream crate from implementing vstd's `View` / `ForLoopGhostIterator` for the foreign type,
  so the type can only be declared (no abstract state). This is the verus-constraints-sanctioned
  way to name an unparseable foreign type in spec signatures ("use `external_type_specification`
  in spec.rs to declare it — do not duplicate the definition"). See
  `nanvix-phys-phys-mod/bugs.md`.
- `src/kernel/src/mm/phys/mod.rs::book_physical_memory_regions` — iterates an
  `alloc::collections::LinkedList` in a `for` loop. Verus has no `LinkedList` model and the
  orphan rule blocks providing one from the kernel crate (see
  `nanvix-phys-phys-mod/bugs.md`). Body cannot be verified; `ensures` states that, on `Ok`,
  every frame in `phys_regions_frame_set(&physical_memory_regions)` becomes reserved.
- `src/kernel/src/mm/phys/mod.rs::book_mmio_regions` — same `LinkedList` limitation.
  `ensures` states that, on `Ok`, every *covered* frame in `mmio_regions_frame_set(mmio_regions)`
  becomes reserved (uncovered MMIO frames are skipped, matching the `frame::is_covered` gate).

## Cross-module dependencies trusted until their module is verified (`external_body` / `assume_specification`)

- `src/kernel/src/mm/phys/frame.rs::init` — also listed under skip; callable from verified `init`.
- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::init` — no specs yet; opaque callee.
- `src/kernel/src/mm/phys/upool.rs::Upool` (struct) and `Upool::new` — no specs yet; opaque
  type/callee needed so verified `init` can construct the user page pool.
- `src/kernel/src/mm/phys/upool.rs::Upool::alloc` — pool allocation primitive the manager's user
  paths call. `ensures` describes the free→allocated transition (`alloc_one`) and the empty-pool
  `Err` arm (`free_count() == 0`). Verified when `upool` is.
- `src/kernel/src/mm/phys/kframe.rs::KernelFrame::map_frame` — exec-only helper holding the
  identity-mapping side effect extracted from `KernelFrame::new`. Declared in
  `kframe.spec.rs` as `pub assume_specification[ KernelFrame::map_frame ](base: FrameAddress)
  -> Result<(), Error>;` with an **empty** contract (no `requires`, no abstract `ensures`).
  Its sole effect is calling `mm::virt::identity_map_page`, whose precondition
  `identity_map_view().inv()` is a global invariant of the not-yet-verified `mm::virt` module.
  That invariant cannot be discharged from `mm::phys`: `identity_map_view` is an `uninterp spec
  fn` in the PRIVATE `mod identity_map` and is not re-exported, so it cannot even be NAMED here
  (verified: `grep identity_map_view` in `mm/virt/mod.rs` shows only `identity_map_page`,
  `memcpy`, `sync_kernel_pdes` are re-exported). This trusts strictly LESS than the previous
  `external_body` on `new`: the owned-frame identity (`kf@ == base@`) and well-formedness
  (`kf.inv()`) postconditions of `new` are now machine-verified; only the cross-module
  page-table side effect remains trusted, exactly at the `mm::virt` boundary. Removed when
  `mm::virt` is verified, at which point `new` can call `identity_map_page` directly.
- `src/kernel/src/mm/phys/frame.rs::alloc` — singleton wrapper around `Inner::alloc`;
  `ensures Ok(frame) => frame.inv()`.
- `src/kernel/src/mm/phys/frame.rs::alloc_contiguous` — singleton wrapper around
  `Inner::alloc_contiguous`; `requires count > 0`, `ensures` page-aligned base plus the
  address-space range bound the manager's index arithmetic relies upon.
- `src/kernel/src/mm/phys/frame.rs::free_count` — reports the free-partition size
  (`== phys_view().frames.free_count()`).
- `src/kernel/src/mm/phys/frame.rs::free` — best-effort frame release used by manager error
  cleanup; no precondition, no abstract postcondition (callers ignore the outcome).
- `src/kernel/src/mm/phys/frame.rs::book` — singleton wrapper around `Inner::book`. On `Ok`
  the frame becomes reserved in `phys_view().frames`; on `Err` it was not free. Like
  `frame::alloc`, the post-mutation `reserved(phys_addr@)` references the *new* singleton state,
  which `instance()` does not pin (it pins only the pre-call `(*r)@`). The free→reserved
  transition therefore lives in the verified `Inner::book` and is bridged to `phys_view().frames`
  by the §8 ghost token in the proving phase; `external_body` until the free-function layer is
  verified. Sibling of `frame::alloc`/`free`.
- `src/kernel/src/mm/phys/frame.rs::alloc_range` — singleton wrapper around `Inner::alloc_range`.
  On `Ok` every frame of the region is reserved (`all_reserved(...)`); on `Err` not all were free.
  Same post-mutation `phys_view().frames` reference as `frame::book`/`alloc`, so the region-level
  free→reserved transition (verified in `Inner::alloc_range`) is bridged by the §8 ghost token in
  the proving phase; `external_body` until the free-function layer is verified.
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

## `assume_specification` retained due to a genuine Verus limitation (`sys::VirtualAddress::into_raw_value`)

- `src/kernel/src/hal/mem/types/address/phys.spec.rs::<::sys::mm::VirtualAddress as
  ::sys::mm::Address>::into_raw_value` — `ensures result as int == addr@`. Trusted contract for a
  not-yet-verifiable `sys` callee, consumed by the verified `PhysicalAddress::into_frame_number`
  (`phys.rs`, `let raw_addr = self.0.into_raw_value();`).

  **Why it cannot be eliminated (unlike its sibling `VirtualAddress::new`):** `into_raw_value` is a
  **trait-impl** method (`<VirtualAddress as Address>::into_raw_value`). Verus requires an *entire*
  trait impl to be verified as a unit:

  > error: In order to verify any items of this trait impl, the entire impl must be verified.
  > Try wrapping the entire impl in the `verus!` macro.

  But the same `impl Address for VirtualAddress` block (`src/libs/sys/src/sys/mm/address/virt.rs`)
  contains `as_ptr` / `as_mut_ptr`, whose `usize as *const u8` / `usize as *mut u8` int-to-pointer
  casts Verus does not support:

  > error: Verus does not support this cast: `usize` to `*const u8`
  > error: Verus does not support this cast: `usize` to `*mut u8`

  Empirically confirmed against the committed history: commit `d54fd253d` verified `sys` (PASS, 6
  verified, 0 errors) with the block **un-annotated**; a later commit adding `#[verus_verify]` to
  the block regressed `make verify-sys` to a compilation/setup error (`HEAD` = `c7a556350`). That
  regression has been reverted (block left un-annotated) so `make verify-sys` PASSES again.

  Verifying `into_raw_value` would require `external_body` on `as_ptr` / `as_mut_ptr`, i.e. moving
  two int-to-pointer casts into the trusted base — strictly *expanding* the TCB to remove a single
  trivial assumption whose body (`self.0`) plainly satisfies `result as int == self@`. The
  `assume_specification` is therefore the smaller, more honest trust boundary.

  **Isolated reproducers** (minimal standalone Verus snippets, each reproducing one of the two
  errors above verbatim): `verus-ai-logs/nanvix-phys-hal-phys-address/specification/whole_impl_rule.rs`
  (whole-impl-must-verify rule) and `.../specification/ptr_cast.rs` (`usize`→`*const u8` cast).

  Superseded only if/when `sys` gains a Verus-supported pointer-materialization path for `as_ptr` /
  `as_mut_ptr` (e.g. `vstd::raw_ptr` exposed-provenance), after which the whole block can be
  verified and this placeholder removed.
