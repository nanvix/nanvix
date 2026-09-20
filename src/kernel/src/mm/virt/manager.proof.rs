// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
// Executable-to-abstract input/result adapters and abstract specification checks.
// None of these lemmas proves an executable API or assumes runtime/ghost correspondence.
use super::vmem::{
    PageView,
    VmCapacityView,
    VmLifecycleView,
    VmMappingDemand,
    VmResourceView,
};

verus! {

/// Runtime errno classes are not injective: ResourceBusy also denotes an absent PTE, and
/// OutOfMemory also denotes refcount saturation. The TOP relation disambiguates by state.
pub closed spec fn vm_error_refines(error: Error, failure: VmFailure) -> bool {
    match failure {
        VmFailure::InvalidBuffer => error.code == ErrorCode::InvalidArgument,
        VmFailure::InvalidRange => error.code == ErrorCode::BadAddress || error.code
            == ErrorCode::InvalidArgument,
        VmFailure::AlreadyMapped => error.code == ErrorCode::EntryExists || error.code
            == ErrorCode::ResourceBusy,
        VmFailure::NotMapped => error.code == ErrorCode::NoSuchEntry || error.code
            == ErrorCode::ResourceBusy,
        VmFailure::NoMemory | VmFailure::SharingLimit => error.code == ErrorCode::OutOfMemory,
    }
}

/// Preserve the actual success value; an unexpected errno has no abstract outcome.
pub closed spec fn vm_result_refines<T>(
    actual: Result<T, Error>,
    outcome: Result<T, VmFailure>,
) -> bool {
    match (actual, outcome) {
        (Ok(value), Ok(expected)) => value == expected,
        (Err(error), Err(failure)) => vm_error_refines(error, failure),
        _ => false,
    }
}

/// Read the real permission's write component, without interpreting unenforced read/execute bits.
pub closed spec fn vm_page_access(access: AccessPermission) -> PageAccess {
    if access.spec_writable() {
        PageAccess::ReadWrite
    } else {
        PageAccess::ReadOnly
    }
}

/// Decode the actual typed CPU input rather than letting the caller choose abstract fault flags.
pub closed spec fn vm_fault_input(
    address: usize,
    code: ::arch::cpu::excp::ErrorCode,
) -> WriteFault {
    WriteFault {
        address: address as int,
        present: code.spec_raw() & 1u32 != 0,
        write: code.spec_raw() & 2u32 != 0,
        user: code.spec_raw() & 4u32 != 0,
    }
}

/// Proof debt (uninterp): the real Vec allocation capacity, not its sequence length. Replace
/// with a capacity-aware Vec specification and prove it at caller with_capacity/reserve sites.
/// Current vstd does not specify Vec::capacity. Used by alloc_upages' validation postcondition.
pub uninterp spec fn vm_scratch_capacity<T>(buffer: &Vec<T>) -> nat;

/// The executable adapter must not change a returned Boolean or disguise an unexpected error.
proof fn check_result_adapter(error: Error)
    requires
        error.code == ErrorCode::TryAgain,
{
    assert(vm_result_refines::<bool>(Ok(true), Ok(true)));
    assert(!vm_result_refines::<bool>(Ok(true), Ok(false)));
    assert(!vm_result_refines::<bool>(Ok(false), Ok(true)));
    assert forall|failure: VmFailure| !vm_result_refines::<bool>(Err(error), Err(failure)) by {}
}

/// The fault address is the actual executable argument, not an unconstrained ghost input.
proof fn check_fault_adapter(address: usize, code: ::arch::cpu::excp::ErrorCode)
    ensures
        vm_fault_input(address, code).address == address as int,
        vm_fault_input(address, code).present == (code.spec_raw() & 1u32 != 0),
        vm_fault_input(address, code).write == (code.spec_raw() & 2u32 != 0),
        vm_fault_input(address, code).user == (code.spec_raw() & 4u32 != 0),
{
}

/// Small model fixtures are proof machinery, not part of the public API vocabulary.
closed spec fn empty_fixture(region: UserRegion) -> VmState {
    VmState {
        user_region: region,
        spaces: Map::empty().insert(0nat, VmemView::empty()).insert(1nat, VmemView::empty()),
        backings: Map::empty(),
        retained: Set::empty(),
    }
}

closed spec fn page_fixture(
    region: UserRegion,
    addr: int,
    access: PageAccess,
    deferred: bool,
) -> VmState {
    VmState {
        spaces: empty_fixture(region).spaces.insert(
            0nat,
            VmemView {
                pages: Map::empty().insert(
                    addr,
                    PageView { backing: 0nat, access, needs_write_resolution: deferred },
                ),
            },
        ),
        backings: Map::empty().insert(0nat, Seq::new(spec_page_size() as nat, |_offset: int| 0u8)),
        ..empty_fixture(region)
    }
}

proof fn lemma_page_fixture(region: UserRegion, addr: int, access: PageAccess, deferred: bool)
    requires
        region.wf(),
        user_page(region, addr),
        deferred ==> access == PageAccess::ReadWrite,
    ensures
        empty_fixture(region).wf(),
        page_fixture(region, addr, access, deferred).wf(),
        page_fixture(region, addr, access, deferred).exclusive(0nat, addr),
{
    reveal(empty_fixture);
    reveal(page_fixture);
    assert(page_fixture(region, addr, access, deferred).owns(0nat, addr, 0nat));
}

/// Positive witnesses for allocation, permission control, and last-owner reclamation.
proof fn check_single_page_lifecycle(region: UserRegion, addr: int)
    requires
        region.wf(),
        user_page(region, addr),
{
    reveal(empty_fixture);
    reveal(page_fixture);
    lemma_page_fixture(region, addr, PageAccess::ReadWrite, false);
    lemma_page_fixture(region, addr, PageAccess::ReadOnly, false);
    let empty = empty_fixture(region);
    let allocated = page_fixture(region, addr, PageAccess::ReadWrite, false);
    let protected = page_fixture(region, addr, PageAccess::ReadOnly, false);
    assert(page_range(addr, 1) =~= Set::empty().insert(addr));
    assert(allocated.spaces[0nat].pages.dom() =~= empty.spaces[0nat].pages.dom().union(
        page_range(addr, 1),
    ));
    assert(spec_alloc_upages(empty, allocated, 0nat, addr, 1, PageAccess::ReadWrite, true, Ok(())));
    assert(!spec_alloc_upages(empty, empty, 0nat, addr, 1, PageAccess::ReadWrite, true, Ok(())));
    assert(protected =~~= allocated.with_space(
        0nat,
        allocated.spaces[0nat].with_access(addr, PageAccess::ReadOnly),
    ));
    assert(spec_ctrl_upage(allocated, protected, 0nat, addr, PageAccess::ReadOnly, Ok(())));
    assert(!empty.mapped_backing(0nat));
    assert(protected.spaces[0nat].unmapped(addr) =~~= VmemView::empty());
    assert(empty =~~= protected.unmapped(0nat, addr));
    assert(spec_try_unmap_upage(protected, empty, 0nat, addr, Ok(true)));
    assert(spec_try_unmap_upage(empty, empty, 0nat, addr, Ok(false)));
}

/// A last-owner deferred page resolves in place and cannot fail for lack of a new backing.
proof fn check_last_owner_resolution(region: UserRegion, addr: int)
    requires
        region.wf(),
        user_page(region, addr),
{
    reveal(empty_fixture);
    reveal(page_fixture);
    lemma_page_fixture(region, addr, PageAccess::ReadWrite, true);
    lemma_page_fixture(region, addr, PageAccess::ReadWrite, false);
    let before = page_fixture(region, addr, PageAccess::ReadWrite, true);
    let after = page_fixture(region, addr, PageAccess::ReadWrite, false);
    let fault = WriteFault { address: addr + 1, user: true, write: true, present: true };
    assert((addr + 1) % spec_page_size() == 1) by (nonlinear_arith)
        requires
            addr % spec_page_size() == 0,
            spec_page_size() > 1,
    ;
    assert(fault.page() == addr);
    assert(after.spaces =~~= before.spaces.insert(
        0nat,
        VmemView { pages: before.spaces[0nat].pages.insert(addr, after.spaces[0nat].pages[addr]) },
    ));
    assert(spec_try_resolve_cow_fault(before, after, 0nat, fault, Ok(true)));
    assert(!spec_try_resolve_cow_fault(before, before, 0nat, fault, Ok(false)));
    assert(!spec_try_resolve_cow_fault(before, before, 0nat, fault, Err(VmFailure::NoMemory)));
}

/// Shared resolution copies bytes, keeps the other owner's backing, and rejects a no-op.
proof fn check_shared_resolution(region: UserRegion, addr: int)
    requires
        region.wf(),
        user_page(region, addr),
{
    reveal(empty_fixture);
    reveal(page_fixture);
    lemma_page_fixture(region, addr, PageAccess::ReadWrite, false);
    let parent = page_fixture(region, addr, PageAccess::ReadWrite, false);
    let shared = parent.spaces[0nat].shared();
    let linked = parent.with_space(0nat, shared).with_space(1nat, shared);
    assert(linked.owns(0nat, addr, 0nat));
    assert(linked.owns(1nat, addr, 0nat));
    assert(linked.wf());
    assert(parent.spaces[1nat].pages.union_prefer_right(shared.pages) =~= shared.pages);
    assert(spec_link_user_pages(parent, linked, 0nat, 1nat, Ok(())));
    let private = PageView {
        backing: 1nat,
        access: PageAccess::ReadWrite,
        needs_write_resolution: false,
    };
    let resolved = VmState {
        spaces: linked.spaces.insert(1nat, VmemView { pages: Map::empty().insert(addr, private) }),
        backings: linked.backings.insert(1nat, linked.backings[0nat]),
        ..linked
    };
    assert(resolved.owns(0nat, addr, 0nat));
    assert(resolved.owns(1nat, addr, 1nat));
    assert(resolved.wf());
    assert(resolved.spaces =~~= linked.spaces.insert(
        1nat,
        VmemView { pages: linked.spaces[1nat].pages.insert(addr, private) },
    ));
    let fault = WriteFault { address: addr, user: true, write: true, present: true };
    assert(spec_try_resolve_cow_fault(linked, resolved, 1nat, fault, Ok(true)));
    assert(!spec_try_resolve_cow_fault(linked, linked, 1nat, fault, Ok(true)));
    assert(resolved.contents(0nat, addr) == linked.contents(0nat, addr));
    let child_removed = linked.with_space(1nat, VmemView::empty());
    assert(child_removed.owns(0nat, addr, 0nat));
    assert(child_removed.wf());
    assert(linked.spaces[1nat].unmapped(addr) =~~= VmemView::empty());
    assert(child_removed =~~= linked.unmapped(1nat, addr));
    assert(spec_try_unmap_upage(linked, child_removed, 1nat, addr, Ok(true)));
}

/// Detached owners also prevent reclamation; mapping count alone is not an ownership count.
proof fn check_retained_owner(region: UserRegion, addr: int)
    requires
        region.wf(),
        user_page(region, addr),
{
    reveal(empty_fixture);
    reveal(page_fixture);
    lemma_page_fixture(region, addr, PageAccess::ReadOnly, false);
    let before = VmState {
        retained: Set::empty().insert(0nat),
        ..page_fixture(region, addr, PageAccess::ReadOnly, false)
    };
    let after = before.with_space(0nat, VmemView::empty());
    assert(before.wf());
    assert(after.wf());
    assert(before.spaces[0nat].unmapped(addr) =~~= VmemView::empty());
    assert(after =~~= before.unmapped(0nat, addr));
    assert(spec_try_unmap_upage(before, after, 0nat, addr, Ok(true)));
    assert(after.backings.contains_key(0nat));
}

/// Exercise the positive witnesses with a demonstrably inhabited, nonempty address interval.
proof fn check_inhabited_lifecycle() {
    let region = UserRegion { start: spec_page_size(), end: 3 * spec_page_size() };
    check_single_page_lifecycle(region, region.start);
    check_last_owner_resolution(region, region.start);
    check_shared_resolution(region, region.start);
    check_retained_owner(region, region.start);
}

/// A concrete abstract witness rules out an inconsistent common precondition.
proof fn lemma_empty_model(region: UserRegion, space: SpaceId)
    requires
        region.wf(),
    ensures
        vm_requires(
            VmState {
                user_region: region,
                spaces: Map::empty().insert(space, VmemView::empty()),
                backings: Map::empty(),
                retained: Set::empty(),
            },
            space,
        ),
{
}

/// Creating a new space really admits a successful outcome, not only resource failures.
proof fn lemma_new_space_witness(region: UserRegion, source: SpaceId, created: SpaceId)
    requires
        region.wf(),
        source != created,
    ensures
        ({
            let before = VmState {
                user_region: region,
                spaces: Map::empty().insert(source, VmemView::empty()),
                backings: Map::empty(),
                retained: Set::empty(),
            };
            spec_new_vmem(
                before,
                before.with_space(created, VmemView::empty()),
                source,
                Ok(created),
            )
        }),
{
    lemma_empty_model(region, source);
}

/// Zero-length allocation is rejected, never admitted as an unconstrained success.
proof fn lemma_zero_length_allocation(before: VmState, space: SpaceId, addr: int)
    requires
        vm_requires(before, space),
    ensures
        spec_alloc_upages(
            before,
            before,
            space,
            addr,
            0,
            PageAccess::ReadWrite,
            true,
            Err(VmFailure::InvalidRange),
        ),
        !spec_alloc_upages(before, before, space, addr, 0, PageAccess::ReadWrite, true, Ok(())),
{
}

/// Returning true without removing a present mapping is rejected by the unmap target.
proof fn lemma_unmap_rejects_noop(before: VmState, space: SpaceId, addr: int)
    requires
        vm_requires(before, space),
        user_page(before.user_region, addr),
        before.spaces[space].mapped(addr),
    ensures
        !spec_try_unmap_upage(before, before, space, addr, Ok(true)),
        !spec_try_unmap_upage(before, before, space, addr, Ok(false)),
{
    assert(!before.spaces[space].unmapped(addr).mapped(addr));
}

/// Absent pages are successful no-ops, including repeated unmapping.
proof fn lemma_absent_unmap(before: VmState, space: SpaceId, addr: int)
    requires
        vm_requires(before, space),
        user_page(before.user_region, addr),
        !before.spaces[space].mapped(addr),
    ensures
        spec_try_unmap_upage(before, before, space, addr, Ok(false)),
{
}

/// A successful permission revocation cannot retain a deferred grant of write permission.
proof fn lemma_control_revokes_deferred_write(
    before: VmState,
    after: VmState,
    space: SpaceId,
    addr: int,
)
    requires
        spec_ctrl_upage(before, after, space, addr, PageAccess::ReadOnly, Ok(())),
    ensures
        after.spaces[space].pages[addr].access == PageAccess::ReadOnly,
        !after.spaces[space].pages[addr].needs_write_resolution,
        after.contents(space, addr) == before.contents(space, addr),
{
}

/// Linking cannot turn truly read-only storage into a writable child page.
proof fn lemma_link_preserves_readonly(
    before: VmState,
    after: VmState,
    parent: SpaceId,
    child: SpaceId,
    addr: int,
)
    requires
        spec_link_user_pages(before, after, parent, child, Ok(())),
        before.spaces[parent].mapped(addr),
        before.spaces[parent].pages[addr].access == PageAccess::ReadOnly,
    ensures
        after.spaces[child].mapped(addr),
        after.spaces[child].pages[addr].access == PageAccess::ReadOnly,
        !after.spaces[child].pages[addr].needs_write_resolution,
        after.contents(child, addr) == before.contents(parent, addr),
{
}

/// A resolved fault cannot report success without preserving contents and establishing isolation.
proof fn lemma_resolution_preserves_contents(
    before: VmState,
    after: VmState,
    space: SpaceId,
    fault: WriteFault,
)
    requires
        spec_try_resolve_cow_fault(before, after, space, fault, Ok(true)),
    ensures
        after.contents(space, fault.page()) == before.contents(space, fault.page()),
        after.exclusive(space, fault.page()),
        !after.spaces[space].pages[fault.page()].needs_write_resolution,
{
}

/// Eligible faults cannot be hidden as false, even after all other owners have gone away.
proof fn lemma_eligible_fault_not_ignored(before: VmState, space: SpaceId, fault: WriteFault)
    requires
        vm_requires(before, space),
        fault.eligible(before, space),
    ensures
        !spec_try_resolve_cow_fault(before, before, space, fault, Ok(false)),
{
}

/// A non-user fault is forwarded without a change, rather than privatizing a page.
proof fn lemma_non_user_fault(before: VmState, space: SpaceId, addr: int)
    requires
        vm_requires(before, space),
    ensures
        spec_try_resolve_cow_fault(
            before,
            before,
            space,
            WriteFault { address: addr, user: false, write: true, present: true },
            Ok(false),
        ),
{
}

/// Inhabited complete resource state: two live roots and six free frames, with a reserve of two.
closed spec fn resource_fixture() -> VmWorldView {
    let size = spec_page_size();
    VmWorldView {
        memory: empty_fixture(UserRegion { start: size, end: 3 * size }),
        lifecycle: VmLifecycleView {
            active_spaces: Set::empty(),
            resources: VmResourceView {
                owners: Map::empty().insert(0nat, Set::empty().insert(0nat)).insert(
                    1nat,
                    Set::empty().insert(1nat),
                ),
                retained: Set::empty(),
                anchors: Map::empty().insert(0nat, 0nat).insert(1nat, 1nat),
                inherited: Set::empty(),
                user_support: Set::empty(),
                frame_backed: Set::empty().insert(0nat).insert(1nat),
            },
        },
        capacity: VmCapacityView {
            total_frames: 8,
            free_frames: 6,
            kernel_reserve: 2,
            creation_metadata: Set::empty().insert(0nat).insert(1nat),
            mapping_metadata: Set::empty().insert(
                VmMappingDemand { space: 0, pages: page_range(size, 1), user_frames: 1 },
            ),
            share_room: Map::empty(),
            reference_limit: 255,
        },
    }
}

/// A positive creation witness includes a fresh root and exact frame conservation.
#[verifier::spinoff_prover]
proof fn check_resource_lifecycle() {
    reveal(resource_fixture);
    reveal(empty_fixture);
    let before = resource_fixture();
    assert(before.lifecycle.resources.anchors.dom() =~= before.memory.spaces.dom());
    assert(before.wf());
    let boot = VmWorldView {
        lifecycle: VmLifecycleView {
            active_spaces: Set::empty().insert(0nat),
            resources: VmResourceView {
                retained: Set::empty().insert(0nat),
                frame_backed: before.lifecycle.resources.frame_backed.remove(0nat),
                ..before.lifecycle.resources
            },
        },
        ..before
    };
    assert(boot.wf());
    let created = VmWorldView {
        memory: before.memory.with_space(2nat, VmemView::empty()),
        lifecycle: VmLifecycleView {
            resources: VmResourceView {
                owners: before.lifecycle.resources.owners.insert(2nat, Set::empty().insert(2nat)),
                anchors: before.lifecycle.resources.anchors.insert(2nat, 2nat),
                frame_backed: before.lifecycle.resources.frame_backed.insert(2nat),
                ..before.lifecycle.resources
            },
            ..before.lifecycle
        },
        capacity: VmCapacityView { free_frames: 5, ..before.capacity },
    };
    assert(created.lifecycle.resources.anchors.dom() =~= created.memory.spaces.dom());
    assert(created.lifecycle.resources.frame_backed.intersect(
        before.lifecycle.resources.owners.dom(),
    ) =~= before.lifecycle.resources.frame_backed);
    assert(created.wf());
    assert(before.conserves(created));
    assert(before.lifecycle.resources.creates(created.lifecycle.resources, 0nat, 2nat));
    assert(spec_new_vmem(before.memory, created.memory, 0nat, Ok(2nat)));
    assert(spec_new_vmem_resources(before, created, 0nat, Ok(2nat)));
    assert(!spec_new_vmem_resources(before, before, 0nat, Err(VmFailure::NoMemory)));
    let lost_frame = VmWorldView {
        capacity: VmCapacityView { free_frames: 4, ..created.capacity },
        ..created
    };
    assert(!before.conserves(lost_frame));
}

/// Real exhaustion and the watermark are allowed; bogus OOM and metadata leaks are rejected.
proof fn check_resource_failures() {
    reveal(resource_fixture);
    reveal(empty_fixture);
    let ready = resource_fixture();
    let size = spec_page_size();
    assert(ready.lifecycle.resources.anchors.dom() =~= ready.memory.spaces.dom());
    assert(ready.wf());
    assert(ready.capacity.admits_mapping(0nat, page_range(size, 1), 1));
    assert(!spec_alloc_upages_request(
        ready,
        ready,
        0nat,
        size,
        1,
        PageAccess::ReadWrite,
        true,
        0,
        1,
        Err(VmFailure::NoMemory),
    ));
    let reserved = VmWorldView {
        capacity: VmCapacityView { free_frames: 2, ..ready.capacity },
        ..ready
    };
    assert(reserved.wf());
    assert(!reserved.capacity.admits_user_frames(1));
    assert(spec_alloc_upages_request(
        reserved,
        reserved,
        0nat,
        size,
        1,
        PageAccess::ReadWrite,
        true,
        0,
        1,
        Err(VmFailure::NoMemory),
    ));
    let exhausted = VmWorldView {
        capacity: VmCapacityView { free_frames: 0, ..ready.capacity },
        ..ready
    };
    assert(exhausted.wf());
    assert(spec_new_vmem(exhausted.memory, exhausted.memory, 0nat, Err(VmFailure::NoMemory)));
    assert(spec_new_vmem_resources(exhausted, exhausted, 0nat, Err(VmFailure::NoMemory)));
    let leaked = VmWorldView {
        lifecycle: VmLifecycleView {
            resources: VmResourceView {
                owners: reserved.lifecycle.resources.owners.insert(9nat, Set::empty().insert(0nat)),
                ..reserved.lifecycle.resources
            },
            ..reserved.lifecycle
        },
        ..reserved
    };
    assert(!reserved.resources_unchanged(leaked));
    assert(!spec_alloc_upages_request(
        reserved,
        leaked,
        0nat,
        size,
        1,
        PageAccess::ReadWrite,
        true,
        0,
        1,
        Err(VmFailure::NoMemory),
    ));
    assert(spec_alloc_upages_request(
        ready,
        ready,
        0nat,
        size,
        1,
        PageAccess::ReadWrite,
        true,
        1,
        1,
        Err(VmFailure::InvalidBuffer),
    ));
    assert(spec_alloc_upages_request(
        ready,
        ready,
        0nat,
        size,
        1,
        PageAccess::ReadWrite,
        true,
        0,
        0,
        Err(VmFailure::InvalidBuffer),
    ));
    assert(!spec_alloc_upages_request(
        ready,
        ready,
        0nat,
        size,
        1,
        PageAccess::ReadWrite,
        true,
        1,
        1,
        Err(VmFailure::NoMemory),
    ));
}

proof fn lemma_two_space_references(state: VmState, backing: BackingId)
    requires
        state.spaces.dom() == Set::empty().insert(0nat).insert(1nat),
    ensures
        state.mapping_references(backing) == state.references_from(0nat, backing)
            + state.references_from(1nat, backing),
{
    let count = |total: nat, space: SpaceId| total + state.references_from(space, backing);
    let empty = vstd::iset::ISet::<SpaceId>::empty();
    assert(vstd::iset::fold::is_fun_commutative(count));
    vstd::iset::fold::lemma_fold_empty(0nat, count);
    vstd::iset::fold::lemma_fold_insert(empty, 0nat, count, 0nat);
    vstd::iset::fold::lemma_fold_insert(empty.insert(0nat), 0nat, count, 1nat);
    assert(state.spaces.dom().to_iset() =~= empty.insert(0nat).insert(1nat));
}

/// Allocation can satisfy the complete contract, charging both the user and metadata frames.
proof fn check_resource_allocation() {
    reveal(resource_fixture);
    reveal(empty_fixture);
    reveal(page_fixture);
    let before = resource_fixture();
    let size = spec_page_size();
    let memory = page_fixture(before.memory.user_region, size, PageAccess::ReadWrite, false);
    lemma_page_fixture(before.memory.user_region, size, PageAccess::ReadWrite, false);
    assert(memory.spaces.dom() =~= Set::empty().insert(0nat).insert(1nat));
    lemma_two_space_references(memory, 0nat);
    assert(memory.spaces[0nat].pages.dom().filter(
        |addr: int| memory.spaces[0nat].pages[addr].backing == 0nat,
    ) =~= Set::empty().insert(size));
    assert(memory.references_from(0nat, 0nat) == 1);
    assert(memory.spaces[1nat].pages.dom().filter(
        |addr: int| memory.spaces[1nat].pages[addr].backing == 0nat,
    ) =~= Set::empty());
    assert(memory.references_from(1nat, 0nat) == 0);
    let after = VmWorldView {
        memory,
        lifecycle: VmLifecycleView {
            resources: VmResourceView {
                owners: before.lifecycle.resources.owners.insert(2nat, Set::empty().insert(0nat)),
                frame_backed: before.lifecycle.resources.frame_backed.insert(2nat),
                user_support: before.lifecycle.resources.user_support.insert(2nat),
                ..before.lifecycle.resources
            },
            ..before.lifecycle
        },
        capacity: VmCapacityView {
            free_frames: 4,
            share_room: Map::empty().insert(0nat, 254nat),
            ..before.capacity
        },
    };
    assert(before.lifecycle.resources.anchors.dom() =~= before.memory.spaces.dom());
    assert(after.lifecycle.resources.anchors.dom() =~= after.memory.spaces.dom());
    assert(before.wf());
    assert(after.wf());
    assert(page_range(size, 1) =~= Set::empty().insert(size));
    assert(after.memory.spaces[0nat].pages.dom() =~= before.memory.spaces[0nat].pages.dom().union(
        page_range(size, 1),
    ));
    assert(before.conserves(after));
    assert(after.lifecycle.resources.user_support =~= before.lifecycle.resources.user_support.union(
        after.lifecycle.resources.owners.dom().difference(before.lifecycle.resources.owners.dom()),
    ));
    assert(after.lifecycle.resources.owners.restrict(before.lifecycle.resources.owners.dom())
        =~= before.lifecycle.resources.owners);
    assert(after.lifecycle.resources.frame_backed.intersect(before.lifecycle.resources.owners.dom())
        =~= before.lifecycle.resources.frame_backed);
    assert(spec_alloc_upages_request(
        before,
        after,
        0nat,
        size,
        1,
        PageAccess::ReadWrite,
        true,
        0,
        1,
        Ok(()),
    ));
}

/// Sharing capacity counts aliases individually; spare metadata does not excuse refcount OOM.
proof fn check_sharing_capacity() {
    reveal(resource_fixture);
    reveal(empty_fixture);
    reveal(page_fixture);
    let size = spec_page_size();
    let region = UserRegion { start: size, end: 3 * size };
    let parent = page_fixture(region, size, PageAccess::ReadOnly, false);
    let aliases = parent.with_space(
        0nat,
        VmemView {
            pages: parent.spaces[0nat].pages.insert(2 * size, parent.spaces[0nat].pages[size]),
        },
    );
    assert(aliases.references_from(0nat, 0nat) == 2) by {
        assert(aliases.spaces[0nat].pages.dom().filter(
            |addr: int| aliases.spaces[0nat].pages[addr].backing == 0nat,
        ) =~= Set::empty().insert(size).insert(2 * size));
    }
    let limited = VmCapacityView {
        share_room: Map::empty().insert(0nat, 1nat),
        ..resource_fixture().capacity
    };
    assert(aliases.backings.contains_key(0nat));
    assert(limited.share_room[0nat] == 1);
    assert(!limited.admits_sharing(aliases, 0nat));
    let sufficient = VmCapacityView { share_room: Map::empty().insert(0nat, 2nat), ..limited };
    assert(sufficient.admits_sharing(aliases, 0nat));
}

/// Shared permission grants remain deferred, while revocation makes write faults ineligible.
proof fn check_shared_permission_control() {
    reveal(empty_fixture);
    reveal(page_fixture);
    let size = spec_page_size();
    let region = UserRegion { start: size, end: 3 * size };
    lemma_page_fixture(region, size, PageAccess::ReadOnly, false);
    let parent = page_fixture(region, size, PageAccess::ReadOnly, false);
    let before = parent.with_space(1nat, parent.spaces[0nat]);
    assert(before.owns(0nat, size, 0nat));
    assert(before.owns(1nat, size, 0nat));
    assert(before.wf());
    let granted = before.controlled(1nat, size, PageAccess::ReadWrite);
    assert(granted.owns(1nat, size, 0nat));
    assert(granted.wf());
    assert(spec_ctrl_upage(before, granted, 1nat, size, PageAccess::ReadWrite, Ok(())));
    assert(granted.spaces[1nat].pages[size].needs_write_resolution);
    assert(granted.write_isolation());
    let direct = before.with_space(
        1nat,
        before.spaces[1nat].with_access(size, PageAccess::ReadWrite),
    );
    assert(direct.owns(0nat, size, 0nat));
    assert(direct.owns(1nat, size, 0nat));
    assert(!direct.exclusive(1nat, size));
    assert(!spec_ctrl_upage(before, direct, 1nat, size, PageAccess::ReadWrite, Ok(())));
    assert(!direct.write_isolation());
    let revoked = granted.controlled(1nat, size, PageAccess::ReadOnly);
    assert(revoked =~~= before);
    assert(spec_ctrl_upage(granted, revoked, 1nat, size, PageAccess::ReadOnly, Ok(())));
    let fault = WriteFault { address: size, user: true, write: true, present: true };
    assert(!fault.eligible(revoked, 1nat));
}

} // verus!
