// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
verus! {

/// Linear proof context passed only through verus_spec(with ...), never a runtime argument.
///
/// Paging objects already own the environment's PDE/PTE tokens; this context must not duplicate
/// or mint them. User-byte/allocator ownership, container refinement, and initialization remain
/// unfinished. Changing a ghost snapshot alone cannot establish concrete correspondence.
pub tracked struct VmProofState {
    ghost model: VmState,
    ghost lifecycle_model: VmLifecycleView,
    ghost capacity_model: VmCapacityView,
    ghost roots: Map<SpaceId, int>,
    ghost frames: Map<BackingId, int>,
}

impl View for VmProofState {
    type V = VmState;

    closed spec fn view(&self) -> VmState {
        self.model
    }
}

impl VmProofState {
    pub closed spec fn snapshot(&self) -> VmWorldView {
        VmWorldView {
            memory: self.model,
            lifecycle: self.lifecycle_model,
            capacity: self.capacity_model,
        }
    }

    /// Concrete ownership/activation correspondence belongs to internal_inv, not this projection.
    pub closed spec fn lifecycle(&self) -> VmLifecycleView {
        self.lifecycle_model
    }

    /// Proof debt (uninterp): define using owned page-table/storage permissions, frame allocator
    /// reference counts, detached handles, configured user bounds, and serialized access. The
    /// lifecycle model must account for all VM-owned storage and all active execution references,
    /// including implicit field-drop effects; private metadata cannot disappear from accounting.
    /// user_support classifies ALL disposable user-mapping support, not a selectable subset;
    /// lifetime and inherited kernel storage cannot be relabeled as disposable during cleanup.
    /// Capacity must refine the authoritative FrameAllocView and heap/slab permissions:
    /// free_frames is exactly the number of covered zero-refcount frames, kernel_reserve is the
    /// configured watermark, and share_room is the actual reference limit minus each refcount.
    /// Metadata admission must be an IFF with feasible joint reservations after reserving the
    /// request's user/root frames, including existing support and heap size classes. Neither an
    /// empty underapproximation nor a result-dependent admission set satisfies this obligation.
    /// Identity-map initialization must cover all allocator frames without runtime BSS allocation.
    /// Ordinary kernel writes must respect deferred writes; explicit shared-writable memory is
    /// outside this private-write model. Prove provider observations complete at initialization
    /// and after every operation; these fields are not independently assignable availability flags.
    /// Required and preserved by all eight TOP contracts. No constructor or lemma assumes it.
    pub uninterp spec fn internal_inv(&self) -> bool;
}

/// An inhabited cleanup witness distinguishes shared, detached, and last-owner user backings.
#[verifier::spinoff_prover]
proof fn check_clear_user_space() {
    let size = spec_page_size();
    let region = UserRegion { start: size, end: 4 * size };
    let shared = PageView {
        backing: 0nat,
        access: PageAccess::ReadOnly,
        needs_write_resolution: false,
    };
    let detached = PageView { backing: 1nat, ..shared };
    let private = PageView { backing: 2nat, ..shared };
    let before = VmState {
        user_region: region,
        spaces: Map::empty().insert(
            0nat,
            VmemView {
                pages: Map::empty().insert(size, shared).insert(2 * size, detached).insert(
                    3 * size,
                    private,
                ),
            },
        ).insert(1nat, VmemView { pages: Map::empty().insert(size, shared) }),
        backings: Map::empty().insert(0nat, Seq::new(size as nat, |_offset: int| 1u8)).insert(
            1nat,
            Seq::new(size as nat, |_offset: int| 2u8),
        ).insert(2nat, Seq::new(size as nat, |_offset: int| 3u8)),
        retained: Set::empty().insert(1nat),
    };
    let lifecycle = VmLifecycleView {
        active_spaces: Set::empty().insert(1nat),
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
    };
    assert(before.owns(0nat, size, 0nat));
    assert(before.owns(0nat, 2 * size, 1nat));
    assert(before.owns(0nat, 3 * size, 2nat));
    assert(before.wf());
    assert(lifecycle.resources.anchors.dom() =~= before.spaces.dom());
    assert(lifecycle.wf(before));
    let after = VmState {
        backings: before.backings.remove(2nat),
        ..before.with_space(0nat, VmemView::empty())
    };
    let empty = before.with_space(0nat, VmemView::empty());
    assert(empty.owns(1nat, size, 0nat));
    assert(after.owns(1nat, size, 0nat));
    assert(after.wf());
    assert(lifecycle.resources.anchors.dom() =~= after.spaces.dom());
    assert(lifecycle.wf(after));
    assert(after =~~= before.cleared(0nat));
    assert(lifecycle.resources =~~= lifecycle.resources.released(0nat, Set::empty()));
    assert(lifecycle.resources.owned_by(0nat) =~= Set::empty().insert(0nat));
    assert(lifecycle.resources.owned_by(0nat).intersect(lifecycle.resources.user_support)
        =~= Set::empty());
    assert(spec_clear_user_space(before, after, lifecycle, lifecycle, 0nat, true));
    assert(after.has_space(0nat));
    assert(after.spaces[0nat].pages.dom().is_empty());
    assert(after.contents(1nat, size) == before.contents(1nat, size));
    assert(after.backings[1nat] == before.backings[1nat]);
    assert(!after.backings.contains_key(2nat));
    assert(!spec_clear_user_space(before, before, lifecycle, lifecycle, 0nat, true));
    assert(!spec_clear_user_space(before, after, lifecycle, lifecycle, 0nat, false));
    let active = VmLifecycleView { active_spaces: Set::empty().insert(0nat), ..lifecycle };
    assert(!spec_clear_user_space(before, after, active, lifecycle, 0nat, true));
}

/// Clearing reclaims disposable mapping support, not the root or inherited kernel resources.
proof fn check_clear_mapping_support() {
    let size = spec_page_size();
    let before = VmState {
        user_region: UserRegion { start: size, end: 2 * size },
        spaces: Map::empty().insert(
            0nat,
            VmemView {
                pages: Map::empty().insert(
                    size,
                    PageView {
                        backing: 0nat,
                        access: PageAccess::ReadOnly,
                        needs_write_resolution: false,
                    },
                ),
            },
        ),
        backings: Map::empty().insert(0nat, Seq::new(size as nat, |_offset: int| 0u8)),
        retained: Set::empty(),
    };
    let lifecycle = VmLifecycleView {
        active_spaces: Set::empty(),
        resources: VmResourceView {
            owners: Map::empty().insert(0nat, Set::empty().insert(0nat)).insert(
                1nat,
                Set::empty().insert(0nat),
            ).insert(2nat, Set::empty().insert(0nat)),
            anchors: Map::empty().insert(0nat, 0nat),
            retained: Set::empty().insert(2nat),
            inherited: Set::empty().insert(2nat),
            user_support: Set::empty().insert(1nat),
            frame_backed: Set::empty().insert(0nat).insert(1nat),
        },
    };
    let after = VmState {
        spaces: before.spaces.insert(0nat, VmemView::empty()),
        backings: Map::empty(),
        ..before
    };
    let released = VmLifecycleView {
        resources: lifecycle.resources.released(0nat, Set::empty().insert(1nat)),
        ..lifecycle
    };
    assert(before.owns(0nat, size, 0nat));
    assert(lifecycle.resources.anchors.dom() =~= before.spaces.dom());
    assert(released.resources.anchors.dom() =~= after.spaces.dom());
    assert(after =~~= before.cleared(0nat));
    assert(lifecycle.resources.owned_by(0nat).intersect(lifecycle.resources.user_support)
        =~= Set::empty().insert(1nat));
    assert(spec_clear_user_space(before, after, lifecycle, released, 0nat, true));
    assert(!released.resources.owners.contains_key(1nat));
    assert(released.resources.owners[0nat].contains(0nat));
    assert(released.resources.owners[2nat].contains(0nat));
    assert(!spec_clear_user_space(before, after, lifecycle, lifecycle, 0nat, true));
}

/// Destruction releases private metadata but preserves other spaces and detached storage.
proof fn check_destroy_vmem() {
    let size = spec_page_size();
    let before = VmState {
        user_region: UserRegion { start: size, end: 2 * size },
        spaces: Map::empty().insert(0nat, VmemView::empty()).insert(1nat, VmemView::empty()),
        backings: Map::empty(),
        retained: Set::empty(),
    };
    let before_lifecycle = VmLifecycleView {
        active_spaces: Set::empty().insert(1nat),
        resources: VmResourceView {
            owners: Map::empty().insert(0nat, Set::empty().insert(0nat)).insert(
                1nat,
                Set::empty().insert(0nat).insert(1nat),
            ).insert(2nat, Set::empty().insert(0nat)).insert(3nat, Set::empty().insert(1nat)),
            retained: Set::empty().insert(2nat),
            anchors: Map::empty().insert(0nat, 0nat).insert(1nat, 3nat),
            inherited: Set::empty().insert(1nat),
            user_support: Set::empty(),
            frame_backed: Set::empty().insert(0nat).insert(3nat),
        },
    };
    let after = VmState { spaces: before.spaces.remove(0nat), ..before };
    let after_lifecycle = VmLifecycleView {
        resources: VmResourceView {
            owners: Map::empty().insert(1nat, Set::empty().insert(1nat)).insert(
                2nat,
                Set::empty(),
            ).insert(3nat, Set::empty().insert(1nat)),
            anchors: Map::empty().insert(1nat, 3nat),
            frame_backed: Set::empty().insert(3nat),
            ..before_lifecycle.resources
        },
        ..before_lifecycle
    };
    assert(after_lifecycle.resources =~~= before_lifecycle.resources.released(
        0nat,
        before_lifecycle.resources.owned_by(0nat),
    ));
    assert(after_lifecycle.resources.anchors.dom() =~= after.spaces.dom());
    assert(spec_destroy_vmem(before, after, before_lifecycle, after_lifecycle, 0nat));
    assert(!after.has_space(0nat));
    assert(after.spaces[1nat] == before.spaces[1nat]);
    assert(!after_lifecycle.resources.owners.contains_key(0nat));
    assert(after_lifecycle.resources.owners[1nat] =~= Set::empty().insert(1nat));
    assert(after_lifecycle.resources.owners.contains_key(2nat));
    assert(after_lifecycle.resources.retained.contains(2nat));
    assert(!spec_destroy_vmem(before, before, before_lifecycle, before_lifecycle, 0nat));
    assert(!spec_destroy_vmem(before, after, before_lifecycle, before_lifecycle, 0nat));
    let active = VmLifecycleView { active_spaces: Set::empty().insert(0nat), ..before_lifecycle };
    assert(!spec_destroy_vmem(before, after, active, after_lifecycle, 0nat));
}

} // verus!
include!("vmem_binding.proof.rs");
