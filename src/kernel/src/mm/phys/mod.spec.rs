// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// PhysMemoryManager - Specifications
//
// This file contains specification functions and view types.

verus! {

use crate::hal::mem::spec_page_size;
use vstd::map::*;

pub uninterp spec fn byte_at_address(ptr: int) -> u8;

/// Abstract view of the frame allocator (`frame::Inner`).
///
/// Models every frame tracked by the allocator as a single map from frame
/// address to reference count:
///
/// - A frame address is a key of `refcounts` iff the allocator covers it.
/// - A covered frame is *free* iff its reference count is `0`.
/// - A covered frame is *allocated* iff its reference count is positive; a
///   count greater than one models shared ownership (e.g. copy-on-write after
///   `share()`).
///
/// Keeping a single map (rather than separate `allocated`/`free` sets that
/// duplicate the same information) makes the specs simpler to write and prove.
#[verifier::ext_equal]
pub struct FrameAllocView
{
    /// Maps each covered frame address to its reference count
    /// (`0` = free, `> 0` = allocated).
    pub refcounts: Map<int, int>,
}

impl FrameAllocView
{
    /// `true` if the allocator covers (tracks) the frame at `addr`.
    pub open spec fn is_covered(&self, addr: int) -> bool
    {
        self.refcounts.contains_key(addr)
    }

    /// `true` if the frame at `addr` is covered and currently allocated.
    pub open spec fn is_allocated(&self, addr: int) -> bool
    {
        self.refcounts.contains_key(addr) && self.refcounts[addr] > 0
    }

    /// `true` if the frame at `addr` is covered and currently free.
    pub open spec fn is_free(&self, addr: int) -> bool
    {
        self.refcounts.contains_key(addr) && self.refcounts[addr] == 0
    }

    /// `true` if no covered frame is free (every covered frame is allocated).
    pub open spec fn no_free_frames(&self) -> bool
    {
        forall|addr: int| #[trigger] self.refcounts.contains_key(addr) ==> self.refcounts[addr] > 0
    }

    /// `true` if every frame address in `frames` is covered and free.
    pub open spec fn all_free(&self, frames: Set<int>) -> bool
    {
        forall|addr: int| #[trigger] frames.contains(addr) ==> self.is_free(addr)
    }

    pub open spec fn wf(&self) -> bool
    {
        // The set of covered frames is finite (it mirrors the finite bitmap),
        // so future proofs can count free/allocated frames via `dom().len()`.
        &&& self.refcounts.dom().finite()
        // Every covered frame address is page-aligned.
        &&& forall|addr: int| #[trigger] self.refcounts.contains_key(addr) ==>
            addr % spec_page_size() == 0
        // Reference counts fit the u8 range (0 = free, up to 255 owners).
        &&& forall|addr: int| #[trigger] self.refcounts.contains_key(addr) ==>
            0 <= self.refcounts[addr] <= 255
    }
}

//==================================================================================================
// LinkedList — external type registration
//==================================================================================================
//
// `vstd` provides no specification for `alloc::collections::LinkedList`, so it must be
// registered as an external type before it can appear in spec signatures. Only the type
// is registered here; no `View`/iterator specification is provided, because the orphan
// rule forbids a downstream crate from implementing vstd's `View` / `ForLoopGhostIterator`
// traits for the foreign `LinkedList` / `linked_list::Iter` types. As a consequence, the
// two helpers that iterate a `LinkedList` in a `for` loop (`book_physical_memory_regions`,
// `book_mmio_regions`) cannot have their bodies verified and are `external_body`. See
// `verus-ai-logs/nanvix-phys-phys-mod/bugs.md`.

use ::core::alloc::Allocator;

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::accept_recursive_types(T)]
#[verifier::reject_recursive_types(A)]
pub struct ExLinkedList<T, A: Allocator>(::alloc::collections::LinkedList<T, A>);

//==================================================================================================
// Physical-memory subsystem view
//==================================================================================================

/// Abstract view of the global physical-memory subsystem managed by `mm::phys`.
///
/// Pure ghost description — names no `MaybeUninit`, `AtomicBool`, bitmap, refcount slice,
/// or any other storage mechanism. It wraps the existing frame-allocator view
/// (`FrameAllocView`) with the two liveness facts the caller depends on.
pub ghost struct PhysModView {
    /// The frame allocator singleton has been initialized (`frame::init` ran
    /// successfully). All `frames`-related guarantees are meaningful only when this holds.
    pub initialized: bool,
    /// Abstract frame-allocator state: which physical frames are allocated (reserved) vs.
    /// free, with per-frame refcounts. This is the existing `FrameAllocView`.
    pub frames: FrameAllocView,
    /// The `PhysMemoryManager` singleton has been initialized with a fresh user page pool.
    pub manager_ready: bool,
}

/// Current abstract state of the global physical-memory subsystem.
///
/// Uninterpreted accessor: the subsystem state lives in module-level singletons
/// (`frame::INSTANCE`/`INSTANCE_INIT` and the `PhysMemoryManager`/`Upool` singletons)
/// whose value is not directly spec-readable. The cross-call transition (`v -> v'`) is
/// realized in the proving phase by a ghost token over those singletons (see
/// `view_design.md` section 8); during the specification phase it is read like `self@`.
pub uninterp spec fn phys_view() -> PhysModView;

impl PhysModView {
    /// Well-formedness invariant of the subsystem.
    pub open spec fn inv(self) -> bool {
        // Once the allocator is up, the frame partition is well formed.
        &&& self.initialized ==> self.frames.wf()
        // The manager layer can only be up if the allocator is up.
        &&& self.manager_ready ==> self.initialized
    }

    /// The subsystem is fully brought up and self-consistent.
    pub open spec fn live(self) -> bool {
        &&& self.initialized
        &&& self.manager_ready
        &&& self.frames.wf()
    }
}

//==================================================================================================
// Frame-set vocabulary (on the existing FrameAllocView)
//==================================================================================================

impl FrameAllocView {
    /// The allocator tracks (covers) the frame at `addr` — it is one of the frames this
    /// allocator knows about, allocated or free. Models `frame::is_covered`.
    pub open spec fn covers(self, addr: int) -> bool {
        self.allocated_frames.contains(addr) || self.free_frames.contains(addr)
    }

    /// The frame at `addr` is reserved: present in the allocated set, hence `alloc()` can
    /// never return it. This is the core caller-visible fact a "booked" frame satisfies.
    pub open spec fn reserved(self, addr: int) -> bool {
        self.allocated_frames.contains(addr)
    }

    /// Every frame address in `set` is reserved.
    pub open spec fn all_reserved(self, set: Set<int>) -> bool {
        forall|a: int| set.contains(a) ==> self.reserved(a)
    }

    /// Every frame address in `set` is currently free (booking precondition: a range can be
    /// booked only if it is entirely free).
    pub open spec fn all_free(self, set: Set<int>) -> bool {
        forall|a: int| #[trigger] set.contains(a) ==> self.free_frames.contains(a)
    }

    /// Reserve every frame in `set` (each assumed free in `self`): move it from
    /// `free_frames` to `allocated_frames` with refcount 1.
    pub open spec fn book_all(self, set: Set<int>) -> FrameAllocView {
        FrameAllocView {
            allocated_frames: self.allocated_frames.union(set),
            free_frames: self.free_frames.difference(set),
            refcounts: self.refcounts.union_prefer_right(
                Map::new(|a: int| set.contains(a), |a: int| 1int),
            ),
        }
    }

    /// Reserve only the *covered* frames of `set`; skip the rest (coverage-gated). Models
    /// the MMIO booking rule.
    pub open spec fn book_covered(self, set: Set<int>) -> FrameAllocView {
        self.book_all(set.filter(|a: int| self.covers(a)))
    }
}

/// Page-aligned physical frame addresses covered by a region `[start, start + size)`.
/// Mirrors the frame-set computation used by `frame::alloc_range`.
pub open spec fn region_frame_addrs(start: int, size: int) -> Set<int> {
    let start_frame_number = start / spec_page_size();
    let end_frame_number = (start + size) / spec_page_size();
    vstd::set_lib::set_int_range(start_frame_number, end_frame_number)
        .map(|i: int| i * spec_page_size())
}

/// The set of physical frame addresses covered by all regions in a physical
/// non-usable-memory list. Uninterpreted: `LinkedList` has no Verus model, so its contents
/// cannot be folded over in spec; this names the abstract union of `region_frame_addrs`
/// over the list, which is all the contract needs.
pub uninterp spec fn phys_regions_frame_set(
    regions: &::alloc::collections::LinkedList<TruncatedMemoryRegion<PhysicalAddress>>,
) -> Set<int>;

/// The set of physical frame addresses covered by all MMIO regions in a list (after
/// GVA->GPA translation). Uninterpreted for the same reason as `phys_regions_frame_set`.
pub uninterp spec fn mmio_regions_frame_set(
    regions: &::alloc::collections::LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Set<int>;

} // end verus!

