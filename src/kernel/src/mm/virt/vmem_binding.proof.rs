// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
use crate::hal::arch::x86::mem::mmu::{
    page_directory::{
        present_pde,
        valid_pde_target,
    },
    page_table::{
        compatible_pte,
        stable_pte_fields,
    },
};

verus! {

/// Frame, CoW, user, write, and present bits; accessed/dirty bits are deliberately excluded.
const VM_PTE_TRANSLATION_MASK: u32 = 0xffff_f207;

const VM_PTE_FRAME_MASK: u32 = 0xffff_f000;

/// Proof debt: the actual entries of the opaque standard-library LinkedList, in iteration order.
/// Each integer must be the stored PageTableAddress's address, and each table the actual stored
/// object with its existing tokens. This is not a caller-selected list of witness tables.
/// A sound container View and mutation contracts must replace this declaration.
pub uninterp spec fn vm_user_tables(
    tables: &LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
) -> Seq<(int, PageTable<PageTableStorage>)>;

closed spec fn vm_pte_translation(value: u32) -> u32 {
    value & VM_PTE_TRANSLATION_MASK
}

closed spec fn vm_pte_matches(page: PageView, frame: int, value: u32) -> bool {
    let bits = vm_pte_translation(value);
    &&& bits & 5u32 == 5u32
    &&& (bits & VM_PTE_FRAME_MASK) as int == frame
    &&& (page.access == PageAccess::ReadWrite <==> bits & 0x202u32 != 0)
    &&& (page.needs_write_resolution <==> bits & 0x200u32 != 0)
    &&& (page.needs_write_resolution ==> bits & 2u32 == 0)
}

impl VmProofState {
    /// Stable identities refer to distinct physical storage; the allocator coupling remains in
    /// internal_inv. BSS roots are allowed and are not charged as allocator frames by this map.
    pub closed spec fn bindings_inv(&self) -> bool {
        &&& self.roots.dom() == self.model.spaces.dom()
        &&& self.frames.dom() == self.model.backings.dom()
        &&& forall|space: SpaceId| #[trigger]
            self.roots.contains_key(space) ==> {
                &&& 0 < self.roots[space] <= VM_PTE_FRAME_MASK as int
                &&& self.roots[space] % spec_page_size() == 0
            }
        &&& forall|backing: BackingId| #[trigger]
            self.frames.contains_key(backing) ==> {
                &&& 0 <= self.frames[backing] <= VM_PTE_FRAME_MASK as int
                &&& self.frames[backing] % spec_page_size() == 0
            }
        &&& forall|a: SpaceId, b: SpaceId| #[trigger]
            self.roots.contains_key(a) && #[trigger] self.roots.contains_key(b) && a != b
                ==> self.roots[a] != self.roots[b]
        &&& forall|a: BackingId, b: BackingId| #[trigger]
            self.frames.contains_key(a) && #[trigger] self.frames.contains_key(b) && a != b
                ==> self.frames[a] != self.frames[b]
        &&& forall|space: SpaceId, backing: BackingId| #[trigger]
            self.roots.contains_key(space) && #[trigger] self.frames.contains_key(backing)
                ==> self.roots[space] != self.frames[backing]
    }

    /// Surviving identities cannot be rebound to different physical storage during another API.
    pub closed spec fn preserves_bindings(&self, after: &Self) -> bool {
        &&& forall|space: SpaceId| #[trigger]
            self.roots.contains_key(space) && #[trigger] after.roots.contains_key(space)
                ==> self.roots[space] == after.roots[space]
        &&& forall|backing: BackingId| #[trigger]
            self.frames.contains_key(backing) && #[trigger] after.frames.contains_key(backing)
                ==> self.frames[backing] == after.frames[backing]
    }

    /// Decode a real table's token baselines, not immutable snapshots of MMU-owned entry bytes.
    closed spec fn matches_user_table(
        &self,
        table: &PageTable<PageTableStorage>,
        space: SpaceId,
        base: int,
    ) -> bool {
        &&& table.ready_for_mmu()
        &&& forall|slot: nat|
            0 <= slot < ::arch::mem::PAGE_TABLE_LENGTH ==> {
                let addr = base + slot * spec_page_size();
                let permission = #[trigger] table.permissions()[slot];
                match permission.expected() {
                    None => false,
                    Some(value) => if self.model.spaces[space].mapped(addr) {
                        let page = self.model.spaces[space].pages[addr];
                        self.frames.contains_key(page.backing) && vm_pte_matches(
                            page,
                            self.frames[page.backing],
                            value,
                        )
                    } else {
                        value & 1u32 == 0
                    },
                }
            }
    }

    /// The actual root and every user PDE/PTE must agree with the public mapping model.
    /// The remaining opaque boundary is container projection, not this correspondence predicate.
    pub closed spec fn matches_vmem(&self, vmem: &Vmem, space: SpaceId) -> bool {
        let tables = vm_user_tables(&vmem.user_page_tables);
        let span = spec_page_size() * ::arch::mem::PAGE_TABLE_LENGTH;
        &&& self.model.has_space(space)
        &&& self.roots.contains_key(space)
        &&& vmem.pgdir.ready_for_mmu()
        &&& vmem.pgdir.physical_base() == self.roots[space]
        &&& forall|i: int|
            0 <= i < tables.len() ==> {
                let entry = #[trigger] tables[i];
                let base = entry.0;
                &&& 0 <= base < usize::MAX as int + 1
                &&& base % span == 0
                &&& base < self.model.user_region.end && self.model.user_region.start < base + span
                &&& self.matches_user_table(&entry.1, space, base)
                &&& match vmem.pgdir.permissions()[(base / span) as nat].expected() {
                    Some(value) => valid_pde_target(value, Some(&entry.1)) && value & 6u32 == 6u32,
                    None => false,
                }
            }
        &&& forall|i: int, j: int|
            0 <= i < tables.len() && 0 <= j < tables.len() && i != j ==> (#[trigger] tables[i]).0
                != (#[trigger] tables[j]).0 && tables[i].1.physical_base()
                != tables[j].1.physical_base()
        &&& forall|addr: int| #[trigger]
            self.model.spaces[space].mapped(addr) ==> exists|i: int|
                0 <= i < tables.len() && (#[trigger] tables[i]).0 == addr - addr % span
        &&& forall|slot: nat|
            0 <= slot < ::arch::mem::PAGE_TABLE_LENGTH && slot * span < self.model.user_region.end
                && self.model.user_region.start < (slot + 1) * span ==> {
                let permission = #[trigger] vmem.pgdir.permissions()[slot];
                match permission.expected() {
                    None => false,
                    Some(value) => present_pde(value) <==> exists|i: int|
                        0 <= i < tables.len() && (#[trigger] tables[i]).0 == slot * span,
                }
            }
    }
}

/// The existing environment contract permits A/D changes without changing TOP-visible fields.
proof fn lemma_vm_pte_translation_stable(expected: u32, observed: u32)
    requires
        compatible_pte(expected, observed),
    ensures
        vm_pte_translation(expected) == vm_pte_translation(observed),
{
    assert(stable_pte_fields(expected) == stable_pte_fields(observed));
    assert(expected & !0x60u32 == observed & !0x60u32) by (bit_vector)
        requires
            stable_pte_fields(expected) == stable_pte_fields(observed),
    ;
    assert(expected & 0xffff_f207u32 == observed & 0xffff_f207u32) by (bit_vector)
        requires
            expected & !0x60u32 == observed & !0x60u32,
    ;
}

/// An admitted observation has exactly the mapping/permission meaning of the token baseline.
proof fn lemma_vm_pte_observation(page: PageView, frame: int, expected: u32, observed: u32)
    requires
        vm_pte_matches(page, frame, expected),
        compatible_pte(expected, observed),
    ensures
        vm_pte_matches(page, frame, observed),
{
    lemma_vm_pte_translation_stable(expected, observed);
}

/// Concrete positive and negative witnesses for the environment-to-TOP interpretation.
proof fn check_vm_pte_interpretation() {
    let shared = PageView {
        backing: 0nat,
        access: PageAccess::ReadWrite,
        needs_write_resolution: true,
    };
    assert(vm_pte_matches(shared, 0x1000, 0x1205)) by (compute);
    assert(compatible_pte(0x1205, 0x1265)) by (bit_vector);
    lemma_vm_pte_observation(shared, 0x1000, 0x1205, 0x1265);
    let readonly = PageView {
        access: PageAccess::ReadOnly,
        needs_write_resolution: false,
        ..shared
    };
    assert(vm_pte_matches(readonly, 0x1000, 0x1005)) by (compute);
    assert(!vm_pte_matches(readonly, 0x1000, 0x1205)) by (compute);
    assert(!vm_pte_matches(shared, 0x1000, 0x1207)) by (compute);
    assert(!compatible_pte(0x1205, 0x2205)) by (bit_vector);
    assert(!compatible_pte(0x1205, 0x1207)) by (bit_vector);
}

} // verus!
