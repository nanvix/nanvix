// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Vmem — Specifications (spec phase)
//
// This file defines the abstract `VmemView` of a virtual address space and the
// spec-level vocabulary (address-partition predicates, permission model, and
// state-transition functions) that the `#[verus_spec]` contracts on `Vmem`'s
// exec methods are written against. See `view_design.md` for the rationale.

verus! {

//==================================================================================================
// External (opaque) dependency types
//
// These types live in lower-level, not-yet-verified crates (`sys`, `hal`,
// `mm::phys`). They are declared `external_body` so Verus treats them as opaque;
// the trust obligation is tracked here and is discharged when those modules are
// verified. Their projection into the abstract domain (below) is therefore
// `uninterp` — a mechanical consequence of the type being `external_body`.
//==================================================================================================

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExVirtualAddress(VirtualAddress);

#[verifier::reject_recursive_types(T)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageAligned<T: Address>(PageAligned<T>);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExAccessPermission(AccessPermission);

// --- Address / physical types (concrete) ---

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPhysicalAddress(PhysicalAddress);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFrameAddress(FrameAddress);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageAddress(PageAddress);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageTableAddress(PageTableAddress);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageDirectoryAddress(PageDirectoryAddress);

#[verifier::reject_recursive_types(T)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageTableAligned<T: Address>(PageTableAligned<T>);

// --- MMU paging types (generic over storage) ---

#[verifier::reject_recursive_types(T)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageDirectory<T: core::ops::DerefMut<Target = [PteWord]>>(PageDirectory<T>);

#[verifier::reject_recursive_types(T)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageTable<T: core::ops::DerefMut<Target = [PteWord]>>(PageTable<T>);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageDirectoryEntry(PageDirectoryEntry);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageTableEntry(PageTableEntry);

// --- Physical / kernel page management types ---

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExKernelPage(KernelPage);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExUserFrame(UserFrame);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPhysMemoryManager(PhysMemoryManager);

// --- std container types not modeled by vstd ---

#[verifier::reject_recursive_types(T)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExRefCell<T: ?Sized>(::core::cell::RefCell<T>);

#[verifier::reject_recursive_types(T)]
#[verifier::reject_recursive_types(A)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExLinkedList<T, A: ::core::alloc::Allocator>(::alloc::collections::LinkedList<T, A>);

//==================================================================================================
// Abstract projections of the opaque address / permission types
//==================================================================================================

/// Projection of an address-like value to its raw byte address as a `nat`.
pub trait AddrNat {
    spec fn addr_nat(&self) -> nat;
}

impl AddrNat for VirtualAddress {
    uninterp spec fn addr_nat(&self) -> nat;
}

impl AddrNat for PageAligned<VirtualAddress> {
    uninterp spec fn addr_nat(&self) -> nat;
}

/// Projection of the concrete permission encoding to the abstract triple.
pub trait PermView {
    spec fn perms_view(&self) -> PagePerms;
}

impl PermView for AccessPermission {
    uninterp spec fn perms_view(&self) -> PagePerms;
}

//==================================================================================================
// Architectural constants (mirror `sys::config` / `arch::mem` layout values)
//==================================================================================================

/// Page size in bytes (`arch::mem::PAGE_SIZE`).
pub open spec fn page_size() -> nat {
    4096
}

/// First user-space byte address (`config::memory_layout::USER_BASE_RAW`).
pub open spec fn user_base() -> nat {
    0x4000_0000
}

/// One-past-last user-space byte address (`config::memory_layout::USER_END_RAW`).
pub open spec fn user_end() -> nat {
    0xf000_0000
}

/// Size of valid guest physical memory (`config::kernel::MEMORY_SIZE`).
///
/// Modeled as a literal (mirroring `page_size`/`user_base`/`user_end` above)
/// because the generated `config::kernel::MEMORY_SIZE` const is not reachable
/// from spec context. Kept in sync with the build-time configuration
/// (`src/libs/config/build.rs`, currently 128 MiB).
pub open spec fn phys_mem_size() -> nat {
    0x800_0000
}

//==================================================================================================
// Address-partition helpers (pure, total, instance-independent)
//==================================================================================================

pub open spec fn is_page_aligned(a: nat) -> bool {
    a % page_size() == 0
}

/// Page-aligned base of the page containing `a`.
pub open spec fn page_base(a: nat) -> nat {
    (a - (a % page_size())) as nat
}

/// Intra-page offset of `a`.
pub open spec fn page_offset(a: nat) -> nat {
    a % page_size()
}

/// Whether `a` lies in user space.
pub open spec fn spec_is_user_addr(a: nat) -> bool {
    user_base() <= a < user_end()
}

/// Whether `a` lies in kernel space (the total complement of user space).
pub open spec fn spec_is_kernel_addr(a: nat) -> bool {
    !spec_is_user_addr(a)
}

/// Whether `[start, start+size)` lies entirely in user space.
pub open spec fn spec_is_user_region(start: nat, size: nat) -> bool {
    &&& size > 0
    &&& user_base() <= start
    &&& start + size <= user_end()
}

/// Whether `[start, start+size)` lies entirely in kernel space (below the user
/// window). Matches the kernel half of the address partition.
pub open spec fn spec_is_kernel_region(start: nat, size: nat) -> bool {
    &&& size > 0
    &&& start + size <= user_base()
}

/// Whether `[start, start+size)` lies entirely within valid physical memory.
/// (`hal::platform::is_valid_physical_region`: rejects empty ranges.)
pub open spec fn spec_is_physical_region(start: nat, size: nat) -> bool {
    &&& size > 0
    &&& start + size <= phys_mem_size()
}

//==================================================================================================
// PagePerms — abstract access permissions
//==================================================================================================

/// Abstract access permissions of a mapped page.
pub struct PagePerms {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

//==================================================================================================
// UserPageView / KernelPageView — single-mapping abstractions
//==================================================================================================

/// Abstract state of a single user-space mapping.
pub struct UserPageView {
    /// Physical base address of the backing user frame.
    pub frame: nat,
    /// Access permissions currently in effect for the page.
    pub perms: PagePerms,
    /// Whether the page is marked copy-on-write.
    pub cow: bool,
}

/// Abstract state of a single kernel-space mapping.
pub struct KernelPageView {
    /// Physical base address of the backing kernel page/frame.
    pub frame: nat,
    /// Access permissions currently in effect for the page.
    pub perms: PagePerms,
}

//==================================================================================================
// VmemView — abstract state of an entire address space
//==================================================================================================

/// Abstract view of a virtual memory space.
pub struct VmemView {
    /// User half: page-aligned user vaddr -> mapping.
    pub user: Map<nat, UserPageView>,
    /// Kernel half: page-aligned kernel vaddr -> mapping.
    pub kernel: Map<nat, KernelPageView>,
    /// Physical base address of the page directory (the value loaded into CR3).
    pub pgdir: nat,
}

impl VmemView {
    //==============================================================================================
    // Well-formedness invariant
    //==============================================================================================

    pub open spec fn inv(self) -> bool {
        // User keys are page-aligned user addresses.
        &&& forall|v: nat| #[trigger] self.user.contains_key(v)
                ==> spec_is_user_addr(v) && is_page_aligned(v)
        // Kernel keys are page-aligned kernel addresses.
        &&& forall|v: nat| #[trigger] self.kernel.contains_key(v)
                ==> spec_is_kernel_addr(v) && is_page_aligned(v)
        // User frames are valid, page-aligned, and CoW implies read-only.
        &&& forall|v: nat| #[trigger] self.user.contains_key(v) ==> {
                &&& is_page_aligned(self.user[v].frame)
                &&& spec_is_physical_region(self.user[v].frame, page_size())
                &&& (self.user[v].cow ==> !self.user[v].perms.write)
            }
        // Kernel frames are valid and page-aligned.
        &&& forall|v: nat| #[trigger] self.kernel.contains_key(v) ==> {
                &&& is_page_aligned(self.kernel[v].frame)
                &&& spec_is_physical_region(self.kernel[v].frame, page_size())
            }
        // The page directory has a valid, page-aligned physical base.
        &&& is_page_aligned(self.pgdir)
        &&& spec_is_physical_region(self.pgdir, page_size())
    }

    //==============================================================================================
    // Observers
    //==============================================================================================

    /// Whether user virtual page `v` is mapped.
    pub open spec fn user_mapped(self, v: nat) -> bool {
        self.user.contains_key(v)
    }

    /// Whether kernel virtual page `v` is mapped.
    pub open spec fn kernel_mapped(self, v: nat) -> bool {
        self.kernel.contains_key(v)
    }

    /// Every CoW user page overlapping `[start, start+size)` is privatized.
    pub open spec fn region_cow_resolved(self, start: nat, size: nat) -> bool {
        forall|v: nat| #[trigger] self.user.contains_key(v)
            && page_base(start) <= v < start + size
            ==> !self.user[v].cow
    }

    //==============================================================================================
    // State transitions (mutators preserve unchanged fields via `..self`)
    //==============================================================================================

    pub open spec fn spec_map(self, v: nat, frame: nat, perms: PagePerms) -> VmemView {
        VmemView { user: self.user.insert(v, UserPageView { frame, perms, cow: false }), ..self }
    }

    pub open spec fn spec_unmap(self, v: nat) -> VmemView {
        VmemView { user: self.user.remove(v), ..self }
    }

    pub open spec fn spec_map_kpage(self, v: nat, frame: nat, perms: PagePerms) -> VmemView {
        VmemView { kernel: self.kernel.insert(v, KernelPageView { frame, perms }), ..self }
    }

    pub open spec fn spec_mark_cow(self, v: nat) -> VmemView {
        VmemView {
            user: self.user.insert(
                v,
                UserPageView {
                    perms: PagePerms { write: false, ..self.user[v].perms },
                    cow: true,
                    ..self.user[v]
                },
            ),
            ..self
        }
    }

    pub open spec fn spec_unmark_cow(self, v: nat) -> VmemView {
        VmemView {
            user: self.user.insert(
                v,
                UserPageView {
                    perms: PagePerms { write: true, ..self.user[v].perms },
                    cow: false,
                    ..self.user[v]
                },
            ),
            ..self
        }
    }

    /// `new_frame` is the freshly allocated private frame supplied existentially
    /// by the caller's ensures clause.
    pub open spec fn spec_resolve_cow(self, v: nat, new_frame: nat) -> VmemView {
        VmemView {
            user: self.user.insert(
                v,
                UserPageView {
                    frame: new_frame,
                    perms: PagePerms { write: true, ..self.user[v].perms },
                    cow: false,
                },
            ),
            ..self
        }
    }

    pub open spec fn spec_uctrl(self, v: nat, perms: PagePerms) -> VmemView {
        VmemView {
            user: self.user.insert(v, UserPageView { perms, ..self.user[v] }),
            ..self
        }
    }

    pub open spec fn spec_kctrl(self, v: nat, perms: PagePerms) -> VmemView {
        VmemView {
            kernel: self.kernel.insert(v, KernelPageView { perms, ..self.kernel[v] }),
            ..self
        }
    }
}

//==================================================================================================
// Vmem — View and invariant
//==================================================================================================

impl View for Vmem {
    type V = VmemView;

    closed spec fn view(&self) -> VmemView {
        self.vmem_view@
    }
}

impl Vmem {
    /// Top-level invariant: the abstract view is well-formed.
    pub open spec fn inv(&self) -> bool {
        &&& self@.inv()
        &&& self.internal_inv()
    }

    /// Internal consistency between the concrete page-table representation and
    /// the abstract view. Refined during the proving phase.
    pub open spec fn internal_inv(&self) -> bool {
        true
    }
}

} // verus!
