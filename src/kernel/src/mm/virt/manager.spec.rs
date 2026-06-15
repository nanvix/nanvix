// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// VirtMemoryManager — Specifications (spec phase)
//
// `VirtMemoryManager` is a zero-sized, stateless singleton façade: every
// observable effect lands on the `Vmem` argument(s) (modeled by the inherited
// `VmemView` from `mm::virt::vmem`), on returned owned RAII values
// (`KernelPage`, `Vec<KernelFrame>`, `Vec<UserFrame>`, a new `Vmem`), or on the
// global physical-frame pool (deliberately outside any View). This file
// therefore contributes (a) a unit manager View, and (b) the manager-level
// vocabulary (pure architectural predicates and range/fork transitions over
// `VmemView`) that the manager's `#[verus_spec]` contracts are written against.
//
// See `view_design.md` for the full rationale.

// Inherited abstractions from the `mm::virt::vmem` module (only available under
// `verus_keep_ghost`, which is exactly when this file is included).
use crate::mm::virt::vmem::{
    is_page_aligned,
    page_base,
    page_size,
    spec_is_physical_region,
    spec_is_user_addr,
    AddrNat,
    PagePerms,
    PermView,
    UserPageView,
    VmemView,
};

verus! {

//==================================================================================================
// External (opaque) dependency types used only by the manager's contracts
//
// These types are not modeled by `mm::virt::vmem`'s spec file. They live in
// lower-level / sibling crates (the x86 exception error code, the kernel-frame
// RAII handle, the ELF file header) and are declared `external_body` so Verus
// treats them as opaque. The trust obligation is discharged when those modules
// are verified.
//==================================================================================================

/// The x86 page-fault error code (`::arch::cpu::excp::ErrorCode`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExExcpErrorCode(::arch::cpu::excp::ErrorCode);

/// RAII handle for a single kernel frame (`mm::phys::KernelFrame`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExKernelFrame(KernelFrame);

/// ELF32 file header (`mm::elf::Elf32Fhdr`).
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExElf32Fhdr(Elf32Fhdr);

//==================================================================================================
// Projection of the opaque page-fault error code into the abstract bit triple
//
// `is_present` / `is_write` / `is_user` are pure `const fn`s decoding hardware
// error-code bits. Their abstract counterparts are uninterpreted projections of
// the opaque external `ErrorCode` (a mechanical consequence of that type being
// `external_body`).
//==================================================================================================

pub trait ErrorCodeBits {
    spec fn ec_present(&self) -> bool;

    spec fn ec_write(&self) -> bool;

    spec fn ec_user(&self) -> bool;
}

impl ErrorCodeBits for ::arch::cpu::excp::ErrorCode {
    uninterp spec fn ec_present(&self) -> bool;

    uninterp spec fn ec_write(&self) -> bool;

    uninterp spec fn ec_user(&self) -> bool;
}

//==================================================================================================
// Pure architectural predicates (no `&self`, instance-independent)
//==================================================================================================

/// A page fault qualifies as a copy-on-write write fault: a user-mode write to a
/// present page. Pure decode of the x86 page-fault error-code bits. Whether the
/// *page* is actually CoW is read from `vmem@` separately.
pub open spec fn spec_is_cow_write_fault(ec: ::arch::cpu::excp::ErrorCode) -> bool {
    ec.ec_present() && ec.ec_write() && ec.ec_user()
}

/// Membership test for the page run `base, base + PAGE_SIZE, …, base +
/// (n-1)*PAGE_SIZE`: `v` is in the run iff it is a page-aligned offset from
/// `base` strictly below `base + n*PAGE_SIZE`.
pub open spec fn in_page_run(base: nat, n: nat, v: nat) -> bool {
    &&& base <= v
    &&& v < base + n * page_size()
    &&& (v - base) as int % page_size() as int == 0
}

/// A mapping is *logically writable* (CoW-eligible at fork time): writable in
/// hardware OR already CoW (left read-only with the CoW bit set by an earlier
/// fork). Mirrors the docstring of `link_user_pages`.
pub open spec fn logically_writable(p: UserPageView) -> bool {
    p.perms.write || p.cow
}

//==================================================================================================
// Range / fork shapes over `VmemView`
//==================================================================================================

/// Success shape of `alloc_upages`: exactly the run `[base, base+n*PAGE_SIZE)`
/// was added, each page mapped with `perms` and not CoW; everything outside the
/// run is bit-for-bit preserved; kernel half and page directory are untouched.
/// The backing frame of each new page is an allocator detail and stays
/// existential (not pinned here).
pub open spec fn maps_user_run_with(
    old: VmemView,
    new: VmemView,
    base: nat,
    n: nat,
    perms: PagePerms,
) -> bool {
    // The user domain grew by exactly the run: a vaddr is mapped afterwards iff
    // it was mapped before or it lies in the freshly-allocated run.
    &&& forall|v: nat| #[trigger] new.user.contains_key(v)
            <==> (old.user.contains_key(v) || in_page_run(base, n, v))
    // Pages outside the run are bit-for-bit preserved.
    &&& forall|v: nat| #[trigger] old.user.contains_key(v) ==> new.user.contains_key(v)
            && new.user[v] == old.user[v]
    // Each new page has the requested perms and is private (not CoW).
    &&& forall|v: nat| in_page_run(base, n, v) ==> #[trigger] new.user.contains_key(v)
            && new.user[v].perms == perms && !new.user[v].cow
    // Kernel half and page directory are unchanged.
    &&& new.kernel == old.kernel
    &&& new.pgdir == old.pgdir
}

/// Caller precondition for `link_user_pages`: the child has no user mappings
/// overlapping the parent's (relied on by the rollback heuristic).
pub open spec fn link_user_pages_pre(parent: VmemView, child: VmemView) -> bool {
    forall|v: nat| #[trigger] parent.user.contains_key(v) ==> !child.user.contains_key(v)
}

/// Success shape of `link_user_pages` on the `(parent, child)` pair. Every
/// present user page of the entry parent is shared into the child at the same
/// vaddr and frame; logically-writable pages become CoW in BOTH parent and
/// child; genuinely read-only pages are shared read-only (not CoW). The child
/// gains exactly the parent's user domain (it had none overlapping on entry —
/// see `link_user_pages_pre`). Parent user frames/domain are preserved (only its
/// CoW marks may flip); kernel halves and page directories are unchanged.
pub open spec fn links_child_cow(
    p_old: VmemView,
    p_new: VmemView,
    c_old: VmemView,
    c_new: VmemView,
) -> bool {
    // Parent domain & frames preserved; perms unchanged except CoW marking.
    &&& p_new.user.dom() == p_old.user.dom()
    &&& forall|v: nat| #[trigger] p_old.user.contains_key(v) ==> {
            &&& p_new.user.contains_key(v)
            &&& p_new.user[v].frame == p_old.user[v].frame
            &&& (logically_writable(p_old.user[v]) ==> p_new.user[v].cow)
            &&& (!logically_writable(p_old.user[v]) ==> p_new.user[v] == p_old.user[v])
        }
    // Child gains exactly the parent's user pages: same frames, CoW iff writable.
    &&& c_new.user.dom() == c_old.user.dom().union(p_old.user.dom())
    &&& forall|v: nat| #[trigger] p_old.user.contains_key(v) ==> {
            &&& c_new.user.contains_key(v)
            &&& c_new.user[v].frame == p_old.user[v].frame
            &&& c_new.user[v].cow == logically_writable(p_old.user[v])
        }
    // Pre-existing, non-overlapping child pages are preserved.
    &&& forall|v: nat| #[trigger] c_old.user.contains_key(v) && !p_old.user.contains_key(v)
            ==> c_new.user.contains_key(v) && c_new.user[v] == c_old.user[v]
    // Kernel halves and page directories unchanged on both.
    &&& p_new.kernel == p_old.kernel
    &&& p_new.pgdir == p_old.pgdir
    &&& c_new.kernel == c_old.kernel
    &&& c_new.pgdir == c_old.pgdir
}

//==================================================================================================
// VirtMemoryManagerView — unit View of the stateless manager singleton
//==================================================================================================

/// Abstract state of the (stateless) virtual-memory manager singleton.
///
/// `VirtMemoryManager` is zero-sized: it owns no mappings, no frames, and no
/// page tables. All abstract state a caller reasons about lives in the `Vmem`
/// argument(s) (`VmemView`) and in the returned owned values. This marker View
/// therefore carries no fields — it exists only so the manager satisfies the
/// module's View/`inv()` convention.
pub struct VirtMemoryManagerView;

impl View for VirtMemoryManager {
    type V = VirtMemoryManagerView;

    closed spec fn view(&self) -> VirtMemoryManagerView {
        VirtMemoryManagerView
    }
}

impl VirtMemoryManager {
    /// Internal consistency. The manager has no internal exec fields, so there is
    /// no implementation-consistency obligation to encode.
    pub open spec fn internal_inv(&self) -> bool {
        true
    }

    /// Top-level invariant. There is no caller-visible manager-level
    /// well-formedness; per-`Vmem` well-formedness is asserted on the `Vmem`
    /// arguments of each method.
    pub open spec fn inv(&self) -> bool {
        self.internal_inv()
    }
}

} // verus!
