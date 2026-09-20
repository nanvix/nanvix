// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
// Public vocabulary for user-page and address-space lifecycles on x86 MicroVM.
//
// These are specification values, not a second executable memory manager. There is deliberately
// no assumed runtime correspondence: coupling the explicit tracked environment to Vmem and
// physical storage is a separate proof obligation in vmem.proof.rs.
use crate::hal::mem::spec_page_size;

verus! {

impl VmProofState {
    /// Coherent user-memory state with exclusive authority over its concrete representation.
    ///
    /// The internal part must establish actual memory ownership, configured bounds, and
    /// serialized access. It is not permission to supply an arbitrary mathematical snapshot.
    pub open spec fn inv(&self) -> bool {
        self.snapshot().wf() && self.bindings_inv() && self.internal_inv()
    }

    /// Bind an actual executable address space to its identity in this environment snapshot.
    pub open spec fn owns(&self, vmem: &Vmem, space: SpaceId) -> bool {
        self@.has_space(space) && self.matches_vmem(vmem, space)
    }
}

/// Abstract address-space identity, independent of a process identifier or paging-root address.
pub type SpaceId = nat;

/// Identity of live user-page storage, independent of a physical frame address.
pub type BackingId = nat;

/// Identity of VM-owned storage other than the user backings in VmState.
///
/// This includes paging structures and shared kernel storage, without exposing their layout.
pub type VmResourceId = nat;

/// Ownership of live VM-management and shared kernel storage.
#[verifier::ext_equal]
pub ghost struct VmResourceView {
    /// Address spaces retaining each resource. Multiple concrete references by one space are
    /// one abstract claim; releasing that claim must release all of its concrete references.
    pub owners: Map<VmResourceId, Set<SpaceId>>,
    /// Resources also retained outside the modeled address spaces, including permanent storage.
    pub retained: Set<VmResourceId>,
    /// Per-space lifetime anchors; unlike disposable mapping metadata, these survive clearing.
    pub anchors: Map<SpaceId, VmResourceId>,
    /// Kernel resources inherited by a new space. Private roots/metadata are never inherited.
    pub inherited: Set<VmResourceId>,
    /// Private, disposable storage supporting user mappings, as opposed to lifetime storage.
    pub user_support: Set<VmResourceId>,
    /// Resources backed by one frame from the same allocator as user backings.
    /// Heap objects and statically reserved BSS storage are not counted here.
    pub frame_backed: Set<VmResourceId>,
}

impl VmResourceView {
    /// Every resource has an owner, and every address-space owner is still live.
    pub open spec fn wf(self, live: Set<SpaceId>) -> bool {
        &&& self.retained.subset_of(self.owners.dom())
        &&& self.inherited.subset_of(self.owners.dom())
        &&& self.user_support.subset_of(self.owners.dom())
        &&& self.user_support.disjoint(self.inherited)
        &&& self.user_support.disjoint(self.retained)
        &&& self.frame_backed.subset_of(self.owners.dom())
        &&& self.anchors.dom() == live
        &&& forall|space: SpaceId|
            #![trigger self.anchors[space]]
            self.anchors.contains_key(space) ==> {
                let anchor = self.anchors[space];
                &&& self.owners.contains_key(anchor)
                &&& self.owners[anchor] == Set::empty().insert(space)
                &&& !self.inherited.contains(anchor)
                &&& (self.frame_backed.contains(anchor) ==> !self.retained.contains(anchor))
                &&& !self.user_support.contains(anchor)
            }
        &&& forall|resource: VmResourceId|
            #![trigger self.owners[resource]]
            self.owners.contains_key(resource) ==> {
                &&& self.owners[resource].subset_of(live)
                &&& (!self.owners[resource].is_empty() || self.retained.contains(resource))
                &&& (self.user_support.contains(resource) ==> self.owners[resource].len() == 1)
            }
    }

    /// All storage claims held by an address space, independent of resource representation.
    pub open spec fn owned_by(self, space: SpaceId) -> Set<VmResourceId> {
        self.owners.dom().filter(|resource: VmResourceId| self.owners[resource].contains(space))
    }

    /// Release selected claims, reclaiming a resource exactly when no owner remains.
    ///
    /// Other spaces' claims and detached owners are unchanged. No new storage is allocated.
    pub open spec fn released(self, space: SpaceId, resources: Set<VmResourceId>) -> Self
        recommends
            resources.subset_of(self.owned_by(space)),
    {
        let claims = Map::new(
            self.owners.dom(),
            |resource: VmResourceId|
                if resources.contains(resource) {
                    self.owners[resource].remove(space)
                } else {
                    self.owners[resource]
                },
        );
        let live = claims.dom().filter(
            |resource: VmResourceId|
                !claims[resource].is_empty() || self.retained.contains(resource),
        );
        Self {
            owners: Map::new(live, |resource: VmResourceId| claims[resource]),
            anchors: if self.anchors.contains_key(space) && resources.contains(
                self.anchors[space],
            ) {
                self.anchors.remove(space)
            } else {
                self.anchors
            },
            inherited: self.inherited.intersect(live),
            user_support: self.user_support.intersect(live),
            frame_backed: self.frame_backed.intersect(live),
            ..self
        }
    }

    /// Allocate only private support for the target, without changing any existing claim.
    pub open spec fn grows_for(self, after: Self, space: SpaceId) -> bool {
        &&& self.owners.dom().subset_of(after.owners.dom())
        &&& after.owners.restrict(self.owners.dom()) == self.owners
        &&& after.retained == self.retained
        &&& after.inherited == self.inherited
        &&& after.anchors == self.anchors
        &&& after.user_support == self.user_support.union(
            after.owners.dom().difference(self.owners.dom()),
        )
        &&& after.frame_backed.intersect(self.owners.dom()) == self.frame_backed
        &&& forall|resource: VmResourceId| #[trigger]
            after.owners.contains_key(resource) && !self.owners.contains_key(resource)
                ==> after.owners[resource] == Set::empty().insert(space)
    }

    /// Clone kernel claims, not user metadata, and allocate a fresh lifetime anchor.
    pub open spec fn creates(self, after: Self, source: SpaceId, created: SpaceId) -> bool {
        &&& self.owners.dom().subset_of(after.owners.dom())
        &&& after.retained == self.retained
        &&& after.inherited == self.inherited
        &&& after.user_support == self.user_support
        &&& after.frame_backed.intersect(self.owners.dom()) == self.frame_backed
        &&& after.anchors == self.anchors.insert(created, after.anchors[created])
        &&& !self.owners.contains_key(after.anchors[created])
        &&& after.frame_backed.contains(after.anchors[created])
        &&& forall|resource: VmResourceId| #[trigger]
            after.owners.contains_key(resource) ==> {
                after.owners[resource] == if self.owners.contains_key(resource) {
                    if self.inherited.contains(resource) && self.owners[resource].contains(source) {
                        self.owners[resource].insert(created)
                    } else {
                        self.owners[resource]
                    }
                } else {
                    Set::empty().insert(created)
                }
            }
    }
}

/// Lifetime observations needed for safe whole-space reclamation.
#[verifier::ext_equal]
pub ghost struct VmLifecycleView {
    /// Spaces still in use by the execution environment and therefore not safe to reclaim.
    /// This abstracts activation/pins, not CR3 encoding or TLB semantics.
    pub active_spaces: Set<SpaceId>,
    /// VM-owned paging and shared kernel storage, separate from user-page contents.
    pub resources: VmResourceView,
}

impl VmLifecycleView {
    /// Active spaces and resource owners must refer to live address spaces.
    pub open spec fn wf(self, state: VmState) -> bool {
        &&& self.active_spaces.subset_of(state.spaces.dom())
        &&& self.resources.wf(state.spaces.dom())
        &&& forall|space: SpaceId|
            #![trigger state.spaces[space]]
            state.has_space(space) && state.spaces[space].pages.dom().is_empty()
                ==> self.resources.owned_by(space).disjoint(self.resources.user_support)
    }

    /// The caller has retired execution references before reclaiming this space.
    pub open spec fn inactive(self, space: SpaceId) -> bool {
        !self.active_spaces.contains(space)
    }
}

/// A complete metadata reservation, not a particular page-table layout or allocation sequence.
///
/// Mapping support includes any additional paging frames and heap records, *after* reserving
/// user_frames from the shared frame pool. This prevents counting those frames twice.
#[verifier::ext_equal]
pub ghost struct VmMappingDemand {
    pub space: SpaceId,
    pub pages: Set<int>,
    pub user_frames: nat,
}

/// Exact resource-provider observations at a serialized operation boundary.
///
/// Metadata admission sets contain ALL and ONLY jointly satisfiable reservations, accounting for
/// reusable existing support, heap size classes, and the shared frame pool. They are not caller
/// preferences, an underapproximation, a forecast of the API's result, or arbitrary "enough" bits.
/// Their soundness AND completeness must be derived from allocator/heap ownership in proof.rs.
#[verifier::ext_equal]
pub ghost struct VmCapacityView {
    pub total_frames: nat,
    pub free_frames: nat,
    pub kernel_reserve: nat,
    /// Sources whose empty-space clone metadata is available after reserving its root frame.
    pub creation_metadata: Set<SpaceId>,
    pub mapping_metadata: Set<VmMappingDemand>,
    /// Representable additional references, including references retained outside VM mappings.
    pub share_room: Map<BackingId, nat>,
    pub reference_limit: nat,
}

impl VmCapacityView {
    pub open spec fn wf(self, state: VmState, lifecycle: VmLifecycleView) -> bool {
        &&& self.reference_limit > 0
        &&& self.kernel_reserve <= self.total_frames
        &&& self.total_frames * spec_page_size() <= usize::MAX as int + 1
        &&& self.free_frames + state.backings.len() + lifecycle.resources.frame_backed.len()
            <= self.total_frames
        &&& self.creation_metadata.subset_of(state.spaces.dom())
        &&& self.share_room.dom() == state.backings.dom()
        &&& forall|backing: BackingId| #[trigger]
            self.share_room.contains_key(backing) ==> {
                &&& self.share_room[backing] < self.reference_limit
                &&& if state.retained.contains(backing) {
                    self.share_room[backing] + state.mapping_references(backing)
                        < self.reference_limit
                } else {
                    self.share_room[backing] + state.mapping_references(backing)
                        == self.reference_limit
                }
            }
    }

    /// User allocations preserve the kernel reserve; root/paging allocations may use it.
    pub open spec fn admits_user_frames(self, count: nat) -> bool {
        count == 0 || self.free_frames >= self.kernel_reserve + count
    }

    pub open spec fn admits_creation(self, source: SpaceId) -> bool {
        self.free_frames >= 1 && self.creation_metadata.contains(source)
    }

    pub open spec fn admits_mapping(self, space: SpaceId, pages: Set<int>, count: nat) -> bool {
        &&& self.admits_user_frames(count)
        &&& (pages.is_empty() || self.mapping_metadata.contains(
            VmMappingDemand { space, pages, user_frames: count },
        ))
    }

    /// Count every alias in the parent, not merely each distinct backing once.
    pub open spec fn admits_sharing(self, state: VmState, parent: SpaceId) -> bool {
        forall|backing: BackingId| #[trigger]
            state.backings.contains_key(backing) ==> self.share_room[backing]
                >= state.references_from(parent, backing)
    }
}

/// Complete public boundary snapshot, with no page-table representation details.
#[verifier::ext_equal]
pub ghost struct VmWorldView {
    pub memory: VmState,
    pub lifecycle: VmLifecycleView,
    pub capacity: VmCapacityView,
}

impl VmWorldView {
    pub open spec fn wf(self) -> bool {
        &&& self.memory.wf()
        &&& self.memory.write_isolation()
        &&& self.lifecycle.wf(self.memory)
        &&& self.capacity.wf(self.memory, self.lifecycle)
    }

    /// No manufactured/lost frames, policy changes, detached references, or activation changes.
    /// Metadata admission is re-observed from the resulting providers, not copied optimistically.
    pub open spec fn conserves(self, after: Self) -> bool {
        &&& self.wf() && after.wf()
        &&& after.lifecycle.active_spaces == self.lifecycle.active_spaces
        &&& after.capacity.total_frames == self.capacity.total_frames
        &&& after.capacity.kernel_reserve == self.capacity.kernel_reserve
        &&& after.capacity.reference_limit == self.capacity.reference_limit
        &&& self.capacity.free_frames + self.memory.backings.len()
            + self.lifecycle.resources.frame_backed.len() == after.capacity.free_frames
            + after.memory.backings.len() + after.lifecycle.resources.frame_backed.len()
        &&& forall|backing: BackingId| #[trigger]
            self.memory.backings.contains_key(backing)
                && #[trigger] after.memory.backings.contains_key(backing) ==> {
                self.capacity.share_room[backing] + self.memory.mapping_references(backing)
                    == after.capacity.share_room[backing] + after.memory.mapping_references(backing)
            }
    }

    /// Even a failed request must return its metadata, frames, and reference reservations.
    pub open spec fn resources_unchanged(self, after: Self) -> bool {
        after.lifecycle == self.lifecycle && after.capacity == self.capacity
    }

    /// Reclamation releases only the selected space's claims; live anchors cannot be removed.
    pub open spec fn releases_from(self, after: Self, space: SpaceId) -> bool {
        exists|released: Set<VmResourceId>|
            released.subset_of(
                self.lifecycle.resources.owned_by(space).intersect(
                    self.lifecycle.resources.user_support,
                ),
            ) && after.lifecycle.resources == #[trigger] self.lifecycle.resources.released(
                space,
                released,
            )
    }
}

/// Effective user-page permissions supported by the x86 paging implementation.
///
/// Read/execute distinctions are not enforced by these APIs; only write permission is modeled.
/// This must not be interpreted as a specification of all AccessPermission bits.
pub enum PageAccess {
    /// User writes are not permitted.
    ReadOnly,
    /// User writes are permitted, possibly after fault resolution.
    ReadWrite,
}

/// One user-page mapping. Contents live in VmState::backings so aliases cannot disagree.
#[verifier::ext_equal]
pub ghost struct PageView {
    /// Shared storage identity.
    pub backing: BackingId,
    /// Logical permission, including writes that first require CoW resolution.
    pub access: PageAccess,
    /// Whether a logical write first needs resolution rather than proceeding directly.
    ///
    /// This distinction remains meaningful for the last owner of a formerly shared page.
    /// It describes fault behavior, not the encoding of a software or hardware flag.
    pub needs_write_resolution: bool,
}

impl PageView {
    /// Logical read-only pages must not acquire write permission through fault resolution.
    pub open spec fn wf(self) -> bool {
        self.needs_write_resolution ==> self.access == PageAccess::ReadWrite
    }

    /// Sharing defers writes to a logically writable page, without changing logical permission.
    pub open spec fn shared(self) -> Self {
        Self { needs_write_resolution: self.access == PageAccess::ReadWrite, ..self }
    }
}

/// A user virtual address space; kernel mappings and paging structures are not exposed.
#[verifier::ext_equal]
pub ghost struct VmemView {
    /// Mappings keyed by page-aligned virtual addresses.
    pub pages: Map<int, PageView>,
}

/// Immutable user address interval, supplied by the platform configuration.
///
/// Relating these bounds to Nanvix's configured USER_BASE/USER_END is a proof obligation;
/// an unsupported external constant is not replaced with an assumed specification.
#[verifier::ext_equal]
pub ghost struct UserRegion {
    /// Inclusive lower bound.
    pub start: int,
    /// Exclusive upper bound.
    pub end: int,
}

impl UserRegion {
    /// A nonempty page-aligned interval within the machine address space.
    pub open spec fn wf(self) -> bool {
        &&& 0 <= self.start < self.end <= usize::MAX as int + 1
        &&& self.start % spec_page_size() == 0
        &&& self.end % spec_page_size() == 0
    }
}

/// Whether an integer denotes a user virtual address in the configured Nanvix layout.
pub open spec fn user_address(region: UserRegion, addr: int) -> bool {
    region.start <= addr < region.end
}

/// Whether an integer denotes the start of a complete user page.
pub open spec fn user_page(region: UserRegion, addr: int) -> bool {
    &&& user_address(region, addr)
    &&& addr % spec_page_size() == 0
    &&& addr + spec_page_size() <= region.end
}

/// A nonempty page-aligned range fully contained in user space.
///
/// Arithmetic is mathematical: the upper bound also excludes machine-address wraparound.
pub open spec fn user_page_range(region: UserRegion, start: int, count: nat) -> bool {
    &&& count > 0
    &&& user_page(region, start)
    &&& start + count * spec_page_size() <= region.end
}

/// The page bases in a range. An empty count denotes the empty set.
pub open spec fn page_range(start: int, count: nat) -> Set<int> {
    Set::range(start, start + count * spec_page_size()).filter(
        |addr: int| (addr - start) % spec_page_size() == 0,
    )
}

impl VmemView {
    /// Finite, aligned user mappings with meaningful logical permissions.
    pub open spec fn wf(self, region: UserRegion) -> bool {
        forall|addr: int| #[trigger]
            self.pages.contains_key(addr) ==> {
                &&& user_page(region, addr)
                &&& self.pages[addr].wf()
            }
    }

    /// A newly created address space has no user mappings.
    pub open spec fn empty() -> Self {
        Self { pages: Map::empty() }
    }

    /// Whether a page base is mapped.
    pub open spec fn mapped(self, addr: int) -> bool {
        self.pages.contains_key(addr)
    }

    /// An exact single-page removal, preserving all other mappings.
    pub open spec fn unmapped(self, addr: int) -> Self {
        Self { pages: self.pages.remove(addr) }
    }

    /// An exact permission update, preserving storage identity.
    ///
    /// Permission control completes the requested change, rather than leaving an old deferred
    /// write capable of restoring a permission that has just been revoked.
    pub open spec fn with_access(self, addr: int, access: PageAccess) -> Self
        recommends
            self.mapped(addr),
    {
        Self {
            pages: self.pages.insert(
                addr,
                PageView { access, needs_write_resolution: false, ..self.pages[addr] },
            ),
        }
    }

    /// The same logical pages prepared for sharing.
    pub open spec fn shared(self) -> Self {
        Self { pages: Map::new(self.pages.dom(), |addr: int| self.pages[addr].shared()) }
    }

    /// Failure may defer some formerly direct writes, but cannot change any logical page.
    ///
    /// This captures link_user_pages rollback without promising that every newly installed
    /// write fault is undone. Already deferred writes must remain deferred.
    pub open spec fn same_pages_with_deferred_writes(self, after: Self) -> bool {
        &&& after.pages.dom() == self.pages.dom()
        &&& forall|addr: int| #[trigger]
            self.pages.contains_key(addr) ==> {
                &&& after.pages[addr].backing == self.pages[addr].backing
                &&& after.pages[addr].access == self.pages[addr].access
                &&& (self.pages[addr].needs_write_resolution
                    ==> after.pages[addr].needs_write_resolution)
            }
    }
}

/// User-memory pre/post state for the TOP operations.
///
/// This contains only user mappings and user-page storage. It does not model kernel-page-table
/// allocation, allocator capacity, kernel mappings, CPU registers, or transient RAII handles.
/// VmLifecycleView separately records reclamation safety and ownership of VM-management storage.
/// Detached user backing owners are represented by retained; their exact reference counts are
/// an implementation detail. The caller must provide a coherent snapshot, not an arbitrary ghost.
#[verifier::ext_equal]
pub ghost struct VmState {
    /// Fixed platform user-address bounds.
    pub user_region: UserRegion,
    /// All live address spaces, including nonparticipants needed to express isolation.
    pub spaces: Map<SpaceId, VmemView>,
    /// One page of bytes for each live user backing.
    pub backings: Map<BackingId, Seq<u8>>,
    /// Backings retained by owners outside the modeled address-space mappings.
    pub retained: Set<BackingId>,
}

impl VmState {
    /// Number of mapping references held by one space (aliases count separately).
    pub open spec fn references_from(self, space: SpaceId, backing: BackingId) -> nat {
        self.spaces[space].pages.dom().filter(
            |addr: int| self.spaces[space].pages[addr].backing == backing,
        ).len()
    }

    /// Mathematical sum of mapping references; retained handles are counted by share_room.
    pub open spec fn mapping_references(self, backing: BackingId) -> nat {
        self.spaces.dom().fold(
            0nat,
            |count: nat, space: SpaceId| count + self.references_from(space, backing),
        )
    }

    /// Private-write semantics: sharing must never leave a directly writable alias.
    pub open spec fn write_isolation(self) -> bool {
        forall|space: SpaceId, addr: int|
            self.has_space(space) && #[trigger] self.spaces[space].mapped(addr)
                && self.spaces[space].pages[addr].access == PageAccess::ReadWrite
                && !self.spaces[space].pages[addr].needs_write_resolution ==> self.exclusive(
                space,
                addr,
            )
    }

    /// Granting writes to shared storage defers them; revocation cancels deferred grants.
    pub open spec fn controlled(self, space: SpaceId, addr: int, access: PageAccess) -> Self {
        let pages = self.spaces[space].pages;
        self.with_space(
            space,
            VmemView {
                pages: pages.insert(
                    addr,
                    PageView {
                        access,
                        needs_write_resolution: access == PageAccess::ReadWrite && !self.exclusive(
                            space,
                            addr,
                        ),
                        ..pages[addr]
                    },
                ),
            },
        )
    }

    /// Whether an address space exists.
    pub open spec fn has_space(self, space: SpaceId) -> bool {
        self.spaces.contains_key(space)
    }

    /// Whether a mapping owns this backing.
    pub open spec fn owns(self, space: SpaceId, addr: int, backing: BackingId) -> bool {
        &&& self.has_space(space)
        &&& self.spaces[space].mapped(addr)
        &&& self.spaces[space].pages[addr].backing == backing
    }

    /// Whether at least one address-space mapping owns this backing.
    pub open spec fn mapped_backing(self, backing: BackingId) -> bool {
        exists|space: SpaceId, addr: int| #[trigger] self.owns(space, addr, backing)
    }

    /// Whether this mapping is the only owner, including detached owners.
    pub open spec fn exclusive(self, space: SpaceId, addr: int) -> bool
        recommends
            self.has_space(space),
            self.spaces[space].mapped(addr),
    {
        let backing = self.spaces[space].pages[addr].backing;
        &&& !self.retained.contains(backing)
        &&& forall|other: SpaceId, page: int| #[trigger]
            self.owns(other, page, backing) ==> other == space && page == addr
    }

    /// Logical consistency and absence of dangling or unowned user backings.
    ///
    /// This is not a representation invariant or a no-leak theorem about physical allocators.
    pub open spec fn wf(self) -> bool {
        &&& self.user_region.wf()
        &&& self.retained.subset_of(self.backings.dom())
        &&& forall|space: SpaceId| #[trigger]
            self.has_space(space) ==> {
                &&& self.spaces[space].wf(self.user_region)
                &&& forall|addr: int| #[trigger]
                    self.spaces[space].mapped(addr) ==> self.backings.contains_key(
                        self.spaces[space].pages[addr].backing,
                    )
            }
        &&& forall|backing: BackingId| #[trigger]
            self.backings.contains_key(backing) ==> {
                &&& self.backings[backing].len() == spec_page_size()
                &&& (self.retained.contains(backing) || self.mapped_backing(backing))
            }
    }

    /// Contents observed through a particular virtual page.
    pub open spec fn contents(self, space: SpaceId, addr: int) -> Seq<u8>
        recommends
            self.has_space(space),
            self.spaces[space].mapped(addr),
    {
        self.backings[self.spaces[space].pages[addr].backing]
    }

    /// Update one address space and preserve every other field.
    pub open spec fn with_space(self, space: SpaceId, value: VmemView) -> Self {
        Self { spaces: self.spaces.insert(space, value), ..self }
    }

    /// Remove a mapping, releasing its backing only when no owner remains.
    pub open spec fn unmapped(self, space: SpaceId, addr: int) -> Self
        recommends
            self.has_space(space),
    {
        let after = self.with_space(space, self.spaces[space].unmapped(addr));
        if self.spaces[space].mapped(addr) {
            let backing = self.spaces[space].pages[addr].backing;
            if !after.mapped_backing(backing) && !after.retained.contains(backing) {
                Self { backings: after.backings.remove(backing), ..after }
            } else {
                after
            }
        } else {
            self
        }
    }

    /// Clear all user mappings, retaining exactly the backings still owned elsewhere.
    pub open spec fn cleared(self, space: SpaceId) -> Self
        recommends
            self.has_space(space),
    {
        let empty = self.with_space(space, VmemView::empty());
        let live = self.backings.dom().filter(
            |backing: BackingId| empty.retained.contains(backing) || empty.mapped_backing(backing),
        );
        Self { backings: Map::new(live, |backing: BackingId| self.backings[backing]), ..empty }
    }
}

/// TOP target for Vmem::clear_user_space.
///
/// Under coherent ownership and an inactive space, cleanup must succeed: internal scan/unmap
/// errors must be proved unreachable, not treated as successful cleanup or atomic rollback.
/// All user mappings disappear, but the space remains live and reusable. Shared/detached user
/// backings survive with unchanged contents. All of this space's disposable user-mapping
/// support is released; lifetime anchors, kernel claims, other owners, and activation are preserved.
pub open spec fn spec_clear_user_space(
    before: VmState,
    after: VmState,
    before_lifecycle: VmLifecycleView,
    after_lifecycle: VmLifecycleView,
    space: SpaceId,
    succeeded: bool,
) -> bool {
    &&& before.wf() && before.has_space(space)
    &&& before_lifecycle.wf(before) && before_lifecycle.inactive(space)
    &&& succeeded
    &&& after.wf() && after_lifecycle.wf(after)
    &&& after == before.cleared(space)
    &&& after_lifecycle.active_spaces == before_lifecycle.active_spaces
    &&& after_lifecycle.resources == before_lifecycle.resources.released(
        space,
        before_lifecycle.resources.owned_by(space).intersect(
            before_lifecycle.resources.user_support,
        ),
    )
}

/// TOP target for the consuming Vmem::destroy entry, after the entire Rust drop operation.
///
/// The space must already be inactive and empty. Retire its identity and release every remaining
/// VM-storage claim, including those released by automatic field destructors after Drop::drop.
/// Reclaim only last-owner storage; preserve other spaces, all user bytes, and detached owners.
pub open spec fn spec_destroy_vmem(
    before: VmState,
    after: VmState,
    before_lifecycle: VmLifecycleView,
    after_lifecycle: VmLifecycleView,
    space: SpaceId,
) -> bool {
    &&& before.wf() && before.has_space(space)
    &&& before.spaces[space].pages.dom().is_empty()
    &&& before_lifecycle.wf(before) && before_lifecycle.inactive(space)
    &&& after.wf() && after_lifecycle.wf(after)
    &&& after == VmState { spaces: before.spaces.remove(space), ..before }
    &&& after_lifecycle == VmLifecycleView {
        resources: before_lifecycle.resources.released(
            space,
            before_lifecycle.resources.owned_by(space),
        ),
        ..before_lifecycle
    }
}

} // verus!
