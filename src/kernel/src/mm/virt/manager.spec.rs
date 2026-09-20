// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
// TOP behavior relations used by the six executable VirtMemoryManager contracts.
//
// Each executable contract threads a tracked VmProofState and binds the actual Vmem arguments,
// addresses, permissions, fault input, and result to these relations. The contracts are proof
// obligations, not assumed guarantees. Representation/ownership correspondence remains proof debt
// in vmem.proof.rs; no executable body is trusted to establish these relations.
//
// Scope: x86 MicroVM, serialized kernel operations, initialized managers, valid caller-owned
// scratch storage. Logical effects and resource properties below are conjoined at the real TOPs.
// Resource observations must be complete, result-independent refinements of the real providers;
// no operation may manufacture its own "unavailable" observation to justify an error.
//
// Known proof gaps:
// - map/unmap have fallible paths after mutation; range rollback is best effort.
// - link rollback may leave deferred writes, and its collection errors bypass rollback.
// - uctrl leaves the CoW bit intact: revocation on a deferred-write page does not implement
//   spec_ctrl_upage as written. This target must not be silently weakened to match that behavior.
// - physical contents, detached ownership, and hardware visibility have no proved coupling yet.
use super::vmem::{
    page_range,
    user_page,
    user_page_range,
    BackingId,
    PageAccess,
    SpaceId,
    UserRegion,
    VmProofState,
    VmState,
    VmWorldView,
    VmemView,
};
use crate::hal::mem::spec_page_size;

verus! {

/// Semantic failure classes, to be related to the real ErrorCode in proof.rs.
///
/// Resource conditions are specified by the companion resource relations at each real TOP.
pub enum VmFailure {
    /// The caller's scratch vector is nonempty or too small; it must be preserved.
    InvalidBuffer,
    /// The requested address or range is not a complete user-page range.
    InvalidRange,
    /// At least one requested address is already mapped.
    AlreadyMapped,
    /// The requested page is absent.
    NotMapped,
    /// A backing or paging-metadata allocation could not be satisfied.
    NoMemory,
    /// An additional shared ownership reference could not be represented.
    SharingLimit,
}

/// Meaning of the page-fault input; no register layout or raw error-code encoding is exposed.
pub ghost struct WriteFault {
    /// Faulting byte address, not necessarily page aligned.
    pub address: int,
    /// The processor reported a user access.
    pub user: bool,
    /// The processor reported a write.
    pub write: bool,
    /// The processor reported a protection fault on a present page.
    pub present: bool,
}

impl WriteFault {
    /// Page containing the faulting byte.
    pub open spec fn page(self) -> int {
        self.address - self.address % spec_page_size()
    }

    /// Whether this is a deferred user write eligible for CoW resolution.
    pub open spec fn eligible(self, state: VmState, space: SpaceId) -> bool {
        &&& self.user && self.write && self.present
        &&& state.has_space(space)
        &&& user_page(state.user_region, self.page())
        &&& state.spaces[space].mapped(self.page())
        &&& state.spaces[space].pages[self.page()].needs_write_resolution
    }
}

/// Common abstract precondition; it does not exclude invalid operation arguments.
///
/// Establishing this from real initialization, mutation, and destruction is a proof obligation.
pub open spec fn vm_requires(before: VmState, space: SpaceId) -> bool {
    before.wf() && before.has_space(space)
}

/// TOP target for VirtMemoryManager::new_vmem.
///
/// Success adds exactly one fresh empty user space. Every existing space and backing is preserved.
/// Failure leaves user state unchanged; the companion resource relation specifies inheritance.
pub open spec fn spec_new_vmem(
    before: VmState,
    after: VmState,
    source: SpaceId,
    result: Result<SpaceId, VmFailure>,
) -> bool {
    &&& vm_requires(before, source)
    &&& after.wf()
    &&& match result {
        Ok(created) => {
            &&& !before.has_space(created)
            &&& after == before.with_space(created, VmemView::empty())
        },
        Err(error) => error == VmFailure::NoMemory && after == before,
    }
}

/// Exact successful range allocation, including its frame over all nonparticipating spaces.
///
/// New backing identities are fresh and pairwise distinct. Contents are unconstrained only when
/// clear is false; even then each backing contains exactly one page of bytes.
pub open spec fn allocated_range(
    before: VmState,
    after: VmState,
    space: SpaceId,
    start: int,
    count: nat,
    access: PageAccess,
    clear: bool,
) -> bool {
    let range = page_range(start, count);
    &&& after.has_space(space)
    &&& after.user_region == before.user_region
    &&& after.spaces[space].pages.dom() == before.spaces[space].pages.dom().union(range)
    &&& after.spaces == before.spaces.insert(space, after.spaces[space])
    &&& after.retained == before.retained
    &&& forall|addr: int| #[trigger]
        before.spaces[space].mapped(addr) ==> after.spaces[space].pages[addr]
            == before.spaces[space].pages[addr]
    &&& forall|addr: int| #[trigger]
        range.contains(addr) ==> {
            let page = after.spaces[space].pages[addr];
            &&& page.access == access
            &&& !page.needs_write_resolution
            &&& !before.backings.contains_key(page.backing)
            &&& (clear ==> after.backings[page.backing] == Seq::new(
                spec_page_size() as nat,
                |_offset: int| 0u8,
            ))
        }
    &&& forall|a: int, b: int| #[trigger]
        range.contains(a) && #[trigger] range.contains(b) && a != b
            ==> after.spaces[space].pages[a].backing != after.spaces[space].pages[b].backing
    &&& before.backings.dom().subset_of(after.backings.dom())
    &&& forall|backing: BackingId| #[trigger]
        before.backings.contains_key(backing) ==> after.backings[backing]
            == before.backings[backing]
    &&& forall|backing: BackingId| #[trigger]
        after.backings.contains_key(backing) ==> before.backings.contains_key(backing) || exists|
            addr: int,
        | #[trigger]
            after.owns(space, addr, backing) && range.contains(addr)
}

/// TOP target for VirtMemoryManager::alloc_upages.
///
/// Invalid ranges and overlaps are rejected without mutation. Success adds the entire range.
/// Resource failure rolls back the entire logical operation, not merely its last iteration.
/// spec_alloc_upages_request handles scratch validation before applying this logical effect.
pub open spec fn spec_alloc_upages(
    before: VmState,
    after: VmState,
    space: SpaceId,
    start: int,
    count: nat,
    access: PageAccess,
    clear: bool,
    result: Result<(), VmFailure>,
) -> bool {
    let valid = user_page_range(before.user_region, start, count);
    let vacant = before.spaces[space].pages.dom().disjoint(page_range(start, count));
    &&& vm_requires(before, space)
    &&& after.wf()
    &&& match result {
        Ok(()) => valid && vacant && allocated_range(
            before,
            after,
            space,
            start,
            count,
            access,
            clear,
        ),
        Err(error) => {
            &&& after == before
            &&& if !valid {
                error == VmFailure::InvalidRange
            } else if !vacant {
                error == VmFailure::AlreadyMapped
            } else {
                error == VmFailure::NoMemory
            }
        },
    }
}

/// TOP target for VirtMemoryManager::try_unmap_upage.
///
/// On valid addresses, the Boolean exactly reports prior presence; absent pages are a no-op.
/// A removed backing is reclaimed only when its last owner disappears. No other storage changes.
/// Under the common precondition, internal lookup/cleanup errors must be proved unreachable.
pub open spec fn spec_try_unmap_upage(
    before: VmState,
    after: VmState,
    space: SpaceId,
    addr: int,
    result: Result<bool, VmFailure>,
) -> bool {
    &&& vm_requires(before, space)
    &&& after.wf()
    &&& match result {
        Ok(removed) => {
            &&& user_page(before.user_region, addr)
            &&& removed == before.spaces[space].mapped(addr)
            &&& after == before.unmapped(space, addr)
        },
        Err(error) => {
            &&& !user_page(before.user_region, addr)
            &&& error == VmFailure::InvalidRange
            &&& after == before
        },
    }
}

/// TOP target for VirtMemoryManager::ctrl_upage.
///
/// Changes effective write permission without changing contents or backing identity. Revocation
/// also prevents a subsequent CoW fault from granting write access again. Granting writes to
/// shared storage defers them until CoW resolution; permission control itself does not allocate.
pub open spec fn spec_ctrl_upage(
    before: VmState,
    after: VmState,
    space: SpaceId,
    addr: int,
    access: PageAccess,
    result: Result<(), VmFailure>,
) -> bool {
    let valid = user_page(before.user_region, addr);
    let mapped = before.spaces[space].mapped(addr);
    &&& vm_requires(before, space)
    &&& after.wf()
    &&& match result {
        Ok(()) => {
            &&& valid && mapped
            &&& after == before.controlled(space, addr, access)
        },
        Err(error) => {
            &&& after == before
            &&& if !valid {
                error == VmFailure::InvalidRange
            } else {
                !mapped && error == VmFailure::NotMapped
            }
        },
    }
}

/// TOP target for VirtMemoryManager::link_user_pages.
///
/// Parent and child share the same bytes and logical permissions. Logically writable copies
/// defer writes; genuinely read-only copies cannot be resolved into writable pages.
/// Existing nonoverlapping child mappings, all other spaces, and all backing contents are stable.
///
/// A resource failure must restore the child and ownership. The parent may acquire extra deferred
/// writes, but its logical permissions, storage identities, and contents must remain unchanged.
pub open spec fn spec_link_user_pages(
    before: VmState,
    after: VmState,
    parent: SpaceId,
    child: SpaceId,
    result: Result<(), VmFailure>,
) -> bool {
    let disjoint = before.spaces[parent].pages.dom().disjoint(before.spaces[child].pages.dom());
    let shared = before.spaces[parent].shared();
    &&& vm_requires(before, parent)
    &&& before.has_space(child)
    &&& parent != child
    &&& after.wf()
    &&& match result {
        Ok(()) => {
            &&& disjoint
            &&& after == before.with_space(parent, shared).with_space(
                child,
                VmemView { pages: before.spaces[child].pages.union_prefer_right(shared.pages) },
            )
        },
        Err(error) => {
            if !disjoint {
                error == VmFailure::AlreadyMapped && after == before
            } else {
                &&& (error == VmFailure::NoMemory || error == VmFailure::SharingLimit)
                &&& !before.spaces[parent].pages.dom().is_empty()
                &&& after.has_space(parent)
                &&& before.spaces[parent].same_pages_with_deferred_writes(after.spaces[parent])
                &&& after == before.with_space(parent, after.spaces[parent])
            }
        },
    }
}

/// Successful resolution preserves bytes and logical permission and makes this mapping exclusive.
///
/// The last owner reuses its backing. Otherwise the new backing is fresh, and the old backing
/// remains available to every other owner, including owners outside these address spaces.
pub open spec fn resolved_write(
    before: VmState,
    after: VmState,
    space: SpaceId,
    addr: int,
) -> bool {
    let old_page = before.spaces[space].pages[addr];
    let page = after.spaces[space].pages[addr];
    &&& after.user_region == before.user_region
    &&& page.access == old_page.access
    &&& !page.needs_write_resolution
    &&& after.exclusive(space, addr)
    &&& after.spaces == before.spaces.insert(
        space,
        VmemView { pages: before.spaces[space].pages.insert(addr, page) },
    )
    &&& after.retained == before.retained
    &&& if before.exclusive(space, addr) {
        page.backing == old_page.backing && after.backings == before.backings
    } else {
        &&& !before.backings.contains_key(page.backing)
        &&& after.backings == before.backings.insert(
            page.backing,
            before.backings[old_page.backing],
        )
    }
}

/// TOP target for VirtMemoryManager::try_resolve_cow_fault.
///
/// False means precisely that this fault is not eligible; it cannot hide a failed CoW resolution.
/// The last owner still resolves successfully, without allocating. A shared eligible page may
/// fail for lack of resources, in which case all modeled state must remain unchanged.
pub open spec fn spec_try_resolve_cow_fault(
    before: VmState,
    after: VmState,
    space: SpaceId,
    fault: WriteFault,
    result: Result<bool, VmFailure>,
) -> bool {
    let eligible = fault.eligible(before, space);
    &&& vm_requires(before, space)
    &&& after.wf()
    &&& match result {
        Ok(true) => eligible && resolved_write(before, after, space, fault.page()),
        Ok(false) => !eligible && after == before,
        Err(error) => {
            &&& eligible
            &&& !before.exclusive(space, fault.page())
            &&& error == VmFailure::NoMemory
            &&& after == before
        },
    }
}

/// Full resource frame for creation. Sufficient root and metadata supply excludes OOM.
pub open spec fn spec_new_vmem_resources(
    before: VmWorldView,
    after: VmWorldView,
    source: SpaceId,
    result: Result<SpaceId, VmFailure>,
) -> bool {
    &&& before.conserves(after)
    &&& match result {
        Ok(created) => before.capacity.admits_creation(source)
            && before.lifecycle.resources.creates(after.lifecycle.resources, source, created),
        Err(_) => !before.capacity.admits_creation(source) && before.resources_unchanged(after),
    }
}

/// Complete allocation boundary, including invalid scratch arguments and error precedence.
///
/// Scratch validation precedes range validation, which precedes overlap and resource checks.
/// A valid request must succeed whenever both its user frames and joint metadata reservation
/// are available. On failure neither user state nor any resource reservation may leak.
pub open spec fn spec_alloc_upages_request(
    before: VmWorldView,
    after: VmWorldView,
    space: SpaceId,
    start: int,
    count: nat,
    access: PageAccess,
    clear: bool,
    scratch_len: nat,
    scratch_capacity: nat,
    result: Result<(), VmFailure>,
) -> bool {
    &&& before.conserves(after)
    &&& if scratch_len != 0 || scratch_capacity < count {
        result == Err(VmFailure::InvalidBuffer) && after == before
    } else {
        &&& spec_alloc_upages(
            before.memory,
            after.memory,
            space,
            start,
            count,
            access,
            clear,
            result,
        )
        &&& match result {
            Ok(()) => {
                &&& before.capacity.admits_mapping(space, page_range(start, count), count)
                &&& before.lifecycle.resources.grows_for(after.lifecycle.resources, space)
            },
            Err(error) => {
                &&& before.resources_unchanged(after)
                &&& (error == VmFailure::NoMemory ==> !before.capacity.admits_mapping(
                    space,
                    page_range(start, count),
                    count,
                ))
            },
        }
    }
}

/// Linking consumes reference headroom, not new user frames; all reservations roll back on error.
/// If both shortages exist, either resource error is allowed: allocation order is not a TOP detail.
pub open spec fn spec_link_user_pages_resources(
    before: VmWorldView,
    after: VmWorldView,
    parent: SpaceId,
    child: SpaceId,
    result: Result<(), VmFailure>,
) -> bool {
    let metadata = before.capacity.admits_mapping(
        child,
        before.memory.spaces[parent].pages.dom(),
        0,
    );
    let sharing = before.capacity.admits_sharing(before.memory, parent);
    &&& before.conserves(after)
    &&& match result {
        Ok(()) => {
            &&& metadata && sharing
            &&& before.lifecycle.resources.grows_for(after.lifecycle.resources, child)
            &&& (before.memory.spaces[parent].pages.dom().is_empty() ==> before.resources_unchanged(
                after,
            ))
        },
        Err(error) => {
            &&& before.resources_unchanged(after)
            &&& (error == VmFailure::NoMemory ==> !metadata)
            &&& (error == VmFailure::SharingLimit ==> !sharing)
        },
    }
}

/// An eligible shared fault needs exactly one policy-admissible user frame, no new mapping support.
/// Initialized identity mapping and a coherent existing mapping cannot invent another OOM cause.
pub open spec fn spec_cow_resources(
    before: VmWorldView,
    after: VmWorldView,
    space: SpaceId,
    fault: WriteFault,
    result: Result<bool, VmFailure>,
) -> bool {
    &&& before.conserves(after)
    &&& after.lifecycle.resources == before.lifecycle.resources
    &&& match result {
        Ok(true) => if before.memory.exclusive(space, fault.page()) {
            before.resources_unchanged(after)
        } else {
            before.capacity.admits_user_frames(1)
        },
        Ok(false) => before.resources_unchanged(after),
        Err(_) => !before.capacity.admits_user_frames(1) && before.resources_unchanged(after),
    }
}

/// No-op/error unmaps cannot mutate metadata; successful removal may release only target claims.
pub open spec fn spec_unmap_resources(
    before: VmWorldView,
    after: VmWorldView,
    space: SpaceId,
    result: Result<bool, VmFailure>,
) -> bool {
    &&& before.conserves(after)
    &&& match result {
        Ok(true) => before.releases_from(after, space),
        _ => before.resources_unchanged(after),
    }
}

} // verus!
