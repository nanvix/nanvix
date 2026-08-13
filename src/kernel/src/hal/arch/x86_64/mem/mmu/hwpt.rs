// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Hardware Page Table Manager for x86_64
//!
//! Manages the 4-level page table hierarchy (PML4 → PDPT → PD → PT)
//! that the CPU uses for virtual-to-physical address translation.
//!
//! The VMM sets up initial identity-mapped page tables with 2 MiB pages
//! covering 0–2 GiB. This module extends those tables by:
//! - Splitting 2 MiB PD entries into 4 KiB PT entries on demand.
//! - Adding new PDPT/PD/PT entries for unmapped regions (e.g., user stack).
//! - Providing `map` and `unmap` for individual 4 KiB pages.

include!("hwpt.spec.rs");

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of page-table pages that can be allocated from the static pool.
const MAX_PT_PAGES: usize = 1024;

/// Number of entries per page table level (PML4, PDPT, PD, PT).
#[verus_verify]
pub const ENTRIES_PER_TABLE: usize = 512;

/// Page table entry flag: Present.
#[verus_verify]
pub const PTE_PRESENT: u64 = 1 << 0;

/// Page table entry flag: Writable.
#[verus_verify]
pub const PTE_WRITABLE: u64 = 1 << 1;

/// Page table entry flag: User-accessible.
#[verus_verify]
pub const PTE_USER: u64 = 1 << 2;

/// Page directory entry flag: Page Size (2 MiB page).
#[verus_verify]
pub const PDE_PS: u64 = 1 << 7;

/// Mask for extracting the physical address from a 4 KiB page table entry.
#[verus_verify]
pub const ADDR_MASK_4K: u64 = 0x000F_FFFF_FFFF_F000;

/// Mask for extracting the physical address from a 2 MiB page directory entry.
#[verus_verify]
pub const ADDR_MASK_2M: u64 = 0x000F_FFFF_FFE0_0000;

//==================================================================================================
// Global State
//==================================================================================================

/// Static pool of page-table pages (each 4 KiB, 512 × u64 entries).
#[repr(C, align(4096))]
struct PtPage([u64; ENTRIES_PER_TABLE]);

/// Pool of page-table pages allocated from BSS.
static mut PT_POOL: [PtPage; MAX_PT_PAGES] = {
    const ZERO_PAGE: PtPage = PtPage([0u64; ENTRIES_PER_TABLE]);
    [ZERO_PAGE; MAX_PT_PAGES]
};

/// Next free index into `PT_POOL`.
static mut PT_POOL_NEXT: usize = 0;

/// Free list of page-table page physical addresses returned by [`free_pt_page`]. Reusing freed
/// pages keeps the static pool from being exhausted by process churn (fork/exec/exit).
static mut PT_FREELIST: [u64; MAX_PT_PAGES] = [0; MAX_PT_PAGES];

/// Number of valid entries in [`PT_FREELIST`].
static mut PT_FREELIST_LEN: usize = 0;

/// Physical address of the PML4 (read from CR3 during `init()`).
static mut PML4_PADDR: usize = 0;

/// Whether the hardware page table manager has been initialized.
static mut INITIALIZED: bool = false;

//==================================================================================================
// Private Helpers
//==================================================================================================

/// Checks that the hardware page-table manager completed runtime initialization.
#[verus_verify(external_body)]
fn assert_initialized() {
    assert!(unsafe { INITIALIZED }, "hwpt: not initialized");
}

/// Returns the boot PML4 physical address discovered during initialization.
#[verus_verify(external_body)]
fn boot_pml4_paddr() -> u64 {
    unsafe { PML4_PADDR as u64 }
}

/// Returns the boot `PD0` physical address discovered during initialization.
#[verus_verify(external_body)]
fn boot_pd0_paddr() -> u64 {
    unsafe { BOOT_PD0_PADDR }
}

/// Allocates a zeroed page-table page from the free list or the static pool. Returns its physical
/// address.
///
/// # Panics
///
/// Panics if the pool is exhausted.
unsafe fn alloc_pt_page() -> u64 {
    // Reuse a freed page if one is available.
    if PT_FREELIST_LEN > 0 {
        PT_FREELIST_LEN -= 1;
        let paddr: u64 = PT_FREELIST[PT_FREELIST_LEN];
        // Zero the reused page before handing it out.
        let ptr: *mut u64 = paddr as *mut u64;
        for i in 0..ENTRIES_PER_TABLE {
            // core::ptr::write_volatile(ptr.add(i), 0);
            unsafe {
                env_interaction_zero_hardware_page_table_entry(ptr.add(i));
            }
        }
        return paddr;
    }

    let idx: usize = PT_POOL_NEXT;
    if idx >= MAX_PT_PAGES {
        error!("hwpt: page-table pool exhausted (used={}, max={})", idx, MAX_PT_PAGES);
        panic!("hwpt: page-table pool exhausted");
    }
    PT_POOL_NEXT += 1;
    // The pool lives in identity-mapped kernel BSS, so virtual address == physical address.
    let ptr: *const PtPage = &PT_POOL[idx];
    ptr as u64
}

/// Returns a page-table page to the free list for later reuse.
///
/// # Safety
///
/// `paddr` must be a page previously obtained from [`alloc_pt_page`] and no longer referenced by
/// any page table.
#[verus_verify(external_body)]
#[verus_spec(
    with
        Tracked(available_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
        Tracked(page):
            Tracked<NanvixHwPageToken>,
    requires
        !old(available_pages).dom().contains(paddr),
        page.physical_base() == paddr,
        page.ready_for_mmu(),
    ensures
        final(available_pages).dom()
            == old(available_pages).dom().insert(paddr),
        final(available_pages)[paddr] == page,
)]
unsafe fn free_pt_page(paddr: u64) {
    if PT_FREELIST_LEN < MAX_PT_PAGES {
        PT_FREELIST[PT_FREELIST_LEN] = paddr;
        PT_FREELIST_LEN += 1;
    }
}

/// Reads a 64-bit entry from a page table at `table_paddr[index]`.
///
/// # Safety
///
/// `table_paddr` must be a valid, identity-mapped physical address of a page table.
#[inline]
#[verus_verify(external_body)]
#[verus_spec(result =>
    with
        Tracked(page): Tracked<&NanvixHwPageToken>,
    requires
        page.ready_for_mmu(),
        table_paddr == page.physical_base(),
        0 <= index < ENTRIES_PER_TABLE,
    ensures
        page.entry(index as nat).admits(page.level(), result),
)]
unsafe fn read_entry(table_paddr: u64, index: usize) -> u64 {
    let ptr: *const u64 = (table_paddr as usize + index * 8) as *const u64;
    // core::ptr::read_volatile(ptr)
    unsafe { env_interaction_read_hardware_page_table_entry(ptr) }
}

/// Writes a 64-bit entry to a page table at `table_paddr[index]`.
///
/// # Safety
///
/// `table_paddr` must be a valid, identity-mapped physical address of a page table.
#[inline]
#[verus_verify(external_body)]
#[verus_spec(
    with
        Tracked(page):
            Tracked<&mut NanvixHwPageToken>,
        Tracked(child):
            Tracked<Option<&NanvixHwPageToken>>,
    requires
        old(page).ready_for_mmu(),
        table_paddr == old(page).physical_base(),
        0 <= index < ENTRIES_PER_TABLE,
        valid_hw_entry(old(page).level(), value),
        valid_hw_entry_target(old(page).level(), value, child),
    ensures
        final(page).ready_for_mmu(),
        final(page).physical_base() == old(page).physical_base(),
        final(page).level() == old(page).level(),
        final(page).entry(index as nat).ptr()
            == old(page).entry(index as nat).ptr(),
        final(page).entry(index as nat).is_init(),
        final(page).entry(index as nat).expected() == value,
        forall|i: nat|
            0 <= i < ENTRIES_PER_TABLE && i != index as nat
                ==> final(page).entry(i) == old(page).entry(i),
)]
unsafe fn write_entry(table_paddr: u64, index: usize, value: u64) {
    let ptr: *mut u64 = (table_paddr as usize + index * 8) as *mut u64;
    // core::ptr::write_volatile(ptr, value);
    unsafe {
        env_interaction_write_hardware_page_table_entry(ptr, value);
    }
}

/// Reads an entry using the unique page token stored by its executable owner.
#[inline]
#[verus_spec(result =>
        with
            Tracked(pages):
                Tracked<&Map<u64, NanvixHwPageToken>>,
        requires
            owned_hw_pages_wf(*pages),
            pages.dom().contains(table_paddr),
            0 <= index < ENTRIES_PER_TABLE,
        ensures
            pages[table_paddr].entry(index as nat).admits(
                pages[table_paddr].level(),
                result,
            ),
    )]
unsafe fn read_owned_entry(table_paddr: u64, index: usize) -> u64 {
    proof_decl! {
        let tracked page: &NanvixHwPageToken;
    }
    proof! {
        page = pages.tracked_borrow(table_paddr);
    }
    unsafe {
        proof_with! {
            Tracked(page)
        };
        read_entry(table_paddr, index)
    }
}

/// Writes an entry using the unique page token stored by its executable owner.
#[inline]
#[verus_spec(
        with
            Tracked(pages):
                Tracked<&mut Map<u64, NanvixHwPageToken>>,
            Ghost(child_paddr):
                Ghost<Option<u64>>,
        requires
            owned_hw_pages_inv(*old(pages)),
            old(pages).dom().contains(table_paddr),
            0 <= index < ENTRIES_PER_TABLE,
            valid_hw_entry(old(pages)[table_paddr].level(), value),
            hw_entry_nonleaf(old(pages)[table_paddr].level(), value)
                == child_paddr.is_some(),
            child_paddr.is_some() ==> {
                let child_base = child_paddr.unwrap();
                &&& child_base != table_paddr
                &&& old(pages).dom().contains(child_base)
                &&& old(pages)[child_base].ready_for_mmu()
                &&& old(pages)[child_base].level()
                    == next_hw_level(old(pages)[table_paddr].level())
                &&& old(pages)[child_base].physical_base()
                    == hw_entry_target_address(value)
            },
        ensures
            owned_hw_pages_inv(*final(pages)),
            final(pages).dom() == old(pages).dom(),
            final(pages)[table_paddr].ready_for_mmu(),
            final(pages)[table_paddr].entry(index as nat).expected() == value,
            forall|base: u64| old(pages).dom().contains(base)
                && base != table_paddr
                    ==> final(pages)[base] == old(pages)[base],
    )]
unsafe fn write_owned_entry(table_paddr: u64, index: usize, value: u64) {
    proof_decl! {
        let tracked mut page: NanvixHwPageToken;
        let tracked child: Option<&NanvixHwPageToken>;
    }
    proof! {
        page = pages.tracked_remove(table_paddr);
        child = if child_paddr.is_some() {
            Some(pages.tracked_borrow(child_paddr.unwrap()))
        } else {
            None
        };
    }
    unsafe {
        proof_with! {
            Tracked(&mut page),
            Tracked(child)
        };
        write_entry(table_paddr, index, value);
    }
    proof! {
        pages.tracked_insert(table_paddr, page);
    }
}

/// Transfers one zeroed page token from manager ownership to an executable owner.
#[inline]
#[verus_verify(external_body)]
#[verus_spec(result =>
        with
            Tracked(available_pages):
                Tracked<&mut Map<u64, NanvixHwPageToken>>,
            Tracked(owner_pages):
                Tracked<&mut Map<u64, NanvixHwPageToken>>,
            Ghost(level):
                Ghost<HwPagingLevel>,
        requires
            !old(available_pages).dom().is_empty(),
            old(available_pages).dom().disjoint(old(owner_pages).dom()),
            owned_hw_pages_inv(*old(owner_pages)),
        ensures
            old(available_pages).dom().contains(result),
            final(available_pages).dom()
                == old(available_pages).dom().remove(result),
            owned_hw_pages_inv(*final(owner_pages)),
            final(owner_pages).dom() == old(owner_pages).dom().insert(result),
            final(owner_pages)[result].physical_base() == result,
            final(owner_pages)[result].level() == level,
            final(owner_pages)[result].is_zeroed(),
    )]
unsafe fn alloc_owned_pt_page() -> u64 {
    unsafe { alloc_pt_page() }
}

/// Returns one detached page token from its executable owner to manager ownership.
#[inline]
#[verus_spec(
        with
            Tracked(available_pages):
                Tracked<&mut Map<u64, NanvixHwPageToken>>,
            Tracked(owner_pages):
                Tracked<&mut Map<u64, NanvixHwPageToken>>,
        requires
                owned_hw_pages_inv(*old(owner_pages)),
                old(owner_pages).dom().contains(paddr),
                !owned_hw_page_is_referenced(*old(owner_pages), paddr),
                !old(available_pages).dom().contains(paddr),
                old(owner_pages)[paddr].ready_for_mmu(),
            ensures
                owned_hw_pages_inv(*final(owner_pages)),
                final(owner_pages).dom() == old(owner_pages).dom().remove(paddr),
                final(available_pages).dom()
                    == old(available_pages).dom().insert(paddr),
    )]
unsafe fn free_owned_pt_page(paddr: u64) {
    proof_decl! {
        let tracked page: NanvixHwPageToken;
    }
    proof! {
        page = owner_pages.tracked_remove(paddr);
    }
    unsafe {
        proof_with! {
            Tracked(available_pages),
            Tracked(page)
        };
        free_pt_page(paddr);
    }
}

/// Returns one detached owner page through the shared manager.
#[inline]
#[verus_spec(
    with
        Tracked(manager):
            Tracked<&HwptManagerHandle>,
        Tracked(owner_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
    requires
        owned_hw_pages_inv(*old(owner_pages)),
        old(owner_pages).dom().contains(paddr),
        !owned_hw_page_is_referenced(*old(owner_pages), paddr),
    ensures
        owned_hw_pages_inv(*final(owner_pages)),
        final(owner_pages).dom() == old(owner_pages).dom().remove(paddr),
)]
unsafe fn return_owned_pt_page(paddr: u64) {
    proof_decl! {
        let tracked manager_inv:
            &::vstd::invariant::LocalInvariant<
                (),
                HwptManagerState,
                HwptManagerInvariant,
            > = manager.borrow();
    }
    open_local_invariant!(manager_inv => manager_state => {
        unsafe {
            proof_with! {
                Tracked(&mut manager_state.available_pages),
                Tracked(owner_pages)
            };
            free_owned_pt_page(paddr);
        }
    });
}

/// Ensures an intermediate page table entry (PML4/PDPT/PD) exists and has the required flags.
/// If the entry is not present, allocates a new zeroed page and installs it.
/// If the entry exists but lacks the User bit and `user` is true, the User bit is added.
///
/// # Safety
///
/// `table_paddr` and `index` must refer to a valid page table.
#[verus_spec(result =>
    with
        Tracked(available_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
        Tracked(owner_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
        Ghost(level):
            Ghost<HwPagingLevel>,
    requires
        owned_hw_pages_inv(*old(owner_pages)),
        old(owner_pages).dom().contains(table_paddr),
        old(owner_pages)[table_paddr].level() == level,
        0 <= index < ENTRIES_PER_TABLE,
    ensures
        owned_hw_pages_inv(*final(owner_pages)),
        final(owner_pages).dom().contains(result),
        final(owner_pages)[result].physical_base() == result,
        final(owner_pages)[result].level() == next_hw_level(level),
        final(owner_pages)[result].ready_for_mmu(),
)]
unsafe fn ensure_table(table_paddr: u64, index: usize, user: bool) -> u64 {
    proof_with! {
        Tracked(&*owner_pages)
    };
    let entry: u64 = read_owned_entry(table_paddr, index);
    if entry & PTE_PRESENT != 0 {
        // Entry exists. If user access is required but the entry lacks PTE_USER, upgrade it.
        // The U/S bit must be set at every level of the page table hierarchy for user-mode
        // access to succeed.
        if user && (entry & PTE_USER == 0) {
            proof_with! {
                Tracked(owner_pages),
                Ghost(Some(entry & ADDR_MASK_4K))
            };
            write_owned_entry(table_paddr, index, entry | PTE_USER);
        }
        entry & ADDR_MASK_4K
    } else {
        // Allocate and install a new table.
        proof_with! {
            Tracked(available_pages),
            Tracked(owner_pages),
            Ghost(next_hw_level(level))
        };
        let new_table: u64 = alloc_owned_pt_page();
        let mut flags: u64 = PTE_PRESENT | PTE_WRITABLE;
        if user {
            flags |= PTE_USER;
        }
        proof_with! {
            Tracked(owner_pages),
            Ghost(Some(new_table))
        };
        write_owned_entry(table_paddr, index, new_table | flags);
        new_table
    }
}

/// Splits a 2 MiB PD entry into 512 × 4 KiB PT entries, preserving the identity mapping.
///
/// # Safety
///
/// `pd_paddr` and `pd_index` must point to a valid 2 MiB PD entry.
#[verus_spec(result =>
    with
        Tracked(available_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
        Tracked(owner_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
    requires
        owned_hw_pages_inv(*old(owner_pages)),
        old(owner_pages).dom().contains(pd_paddr),
        old(owner_pages)[pd_paddr].level() == HwPagingLevel::Pd,
        0 <= pd_index < ENTRIES_PER_TABLE,
    ensures
        owned_hw_pages_inv(*final(owner_pages)),
        final(owner_pages).dom().contains(result),
        final(owner_pages)[result].physical_base() == result,
        final(owner_pages)[result].level() == HwPagingLevel::Pt,
        final(owner_pages)[result].ready_for_mmu(),
)]
unsafe fn split_2m_entry(pd_paddr: u64, pd_index: usize) -> u64 {
    proof_with! {
        Tracked(&*owner_pages)
    };
    let pd_entry: u64 = read_owned_entry(pd_paddr, pd_index);
    let base_2m: u64 = pd_entry & ADDR_MASK_2M;
    let flags_4k: u64 = pd_entry & 0x67; // Present, Writable, User, Accessed, Dirty — drop PS.

    proof_with! {
        Tracked(available_pages),
        Tracked(owner_pages),
        Ghost(HwPagingLevel::Pt)
    };
    let pt_page: u64 = alloc_owned_pt_page();
    for i in 0..ENTRIES_PER_TABLE {
        let pte: u64 = (base_2m + (i as u64 * 4096)) | flags_4k;
        proof_with! {
            Tracked(owner_pages),
            Ghost(None)
        };
        write_owned_entry(pt_page, i, pte);
    }

    // Replace PD entry: point to new PT, drop PS flag.
    let new_pd_entry: u64 = pt_page | (pd_entry & 0x67);
    proof_with! {
        Tracked(owner_pages),
        Ghost(Some(pt_page))
    };
    write_owned_entry(pd_paddr, pd_index, new_pd_entry);

    pt_page
}

/// Flushes the TLB entry for `vaddr`.
#[inline]
#[verus_verify(external_body)]
unsafe fn invlpg(vaddr: usize) {
    // core::arch::asm!("invlpg [{}]", in(reg) vaddr, options(nostack, preserves_flags));
    unsafe {
        env_interaction_invalidate_tlb_page(vaddr);
    }
}

//==================================================================================================
// Public API
//==================================================================================================

/// Initializes the hardware page table manager by reading the current CR3.
///
/// Must be called once during kernel init, after the VMM page tables are in use.
///
/// # Safety
///
/// Must be called from kernel mode with identity mapping active.
#[verus_verify(external_body)]
pub unsafe fn init() {
    // let cr3: u64;
    // core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, nomem));
    let cr3: u64 = unsafe { env_interaction_read_cr3() };
    PML4_PADDR = (cr3 & ADDR_MASK_4K) as usize;

    // Discover boot PD0 address: PML4[0] → PDPT, PDPT[0] → PD0.
    let pml4_entry: u64 = read_entry(PML4_PADDR as u64, 0);
    let pdpt: u64 = pml4_entry & ADDR_MASK_4K;
    let pdpt_entry0: u64 = read_entry(pdpt, 0);
    BOOT_PD0_PADDR = pdpt_entry0 & ADDR_MASK_4K;

    INITIALIZED = true;
}

/// Maps a single 4 KiB page: `vaddr` → `paddr`.
///
/// # Parameters
///
/// - `vaddr`: Virtual address (must be 4 KiB aligned).
/// - `paddr`: Physical frame address.
/// - `user`: If true, the page is accessible from user mode (Ring 3).
/// - `writable`: If true, the page is writable.
///
/// # Safety
///
/// Allocates a fresh per-process 4-level page table (PML4 + PDPT) for a user address space.
///
/// The new PML4 shares the kernel's low-memory mapping by pointing `PDPT[0]` at the boot `PD0`
/// (which identity-maps `0..1 GiB`, covering all of kernel space — code, data, stacks, the IDT/GDT,
/// and the microvm control/pvclock pages). User space (`1 GiB..`) lives in `PDPT[1..]`, whose page
/// directories are created on demand by [`map_user`]. The Local APIC MMIO page (above `1 GiB`) is
/// also mapped, supervisor-only, so the kernel can issue EOIs while running on this address space
/// after an interrupt taken from user mode.
///
/// Returns the physical address of the new PML4 (the value to load into `CR3`).
///
/// # Safety
///
/// Must be called after [`init`].
#[verus_spec(result =>
    with
        Tracked(manager):
            Tracked<&HwptManagerHandle>,
        Tracked(private_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
    requires
        old(private_pages).dom().is_empty(),
    ensures
        owned_hw_pages_inv(*final(private_pages)),
        final(private_pages).dom().contains(result),
        final(private_pages)[result].physical_base() == result,
        final(private_pages)[result].level() == HwPagingLevel::Pml4,
)]
pub unsafe fn create_user_pml4() -> u64 {
    assert_initialized();

    proof_decl! {
        let tracked manager_inv:
            &::vstd::invariant::LocalInvariant<
                (),
                HwptManagerState,
                HwptManagerInvariant,
            > = manager.borrow();
    }
    let new_pml4: u64;
    let new_pdpt: u64;
    open_local_invariant!(manager_inv => manager_state => {
        proof_with! {
            Tracked(&mut manager_state.available_pages),
            Tracked(private_pages),
            Ghost(HwPagingLevel::Pml4)
        };
        new_pml4 = unsafe { alloc_owned_pt_page() };
        proof_with! {
            Tracked(&mut manager_state.available_pages),
            Tracked(private_pages),
            Ghost(HwPagingLevel::Pdpt)
        };
        new_pdpt = unsafe { alloc_owned_pt_page() };

        // PDPT[0] → boot PD0 (shared kernel mapping). PTE_USER is set on this intermediate so that
        // user-accessible low pages (e.g. the pvclock page) remain reachable from Ring 3; the actual
        // U/S permission is still gated by the leaf entries inside the shared PD.
        let boot_pd0: u64 = boot_pd0_paddr();
        proof_decl! {
            let tracked boot_pd0_page: &NanvixHwPageToken =
                manager_state.boot_pages.tracked_borrow(boot_pd0);
            let tracked mut pdpt_page: NanvixHwPageToken =
                private_pages.tracked_remove(new_pdpt);
        }
        unsafe {
            proof_with! {
                Tracked(&mut pdpt_page),
                Tracked(Some(boot_pd0_page))
            };
            write_entry(
                new_pdpt,
                0,
                boot_pd0 | PTE_PRESENT | PTE_WRITABLE | PTE_USER,
            );
        }
        proof! {
            private_pages.tracked_insert(new_pdpt, pdpt_page);
        }

        // PML4[0] → new PDPT.
        unsafe {
            proof_with! {
                Tracked(private_pages),
                Ghost(Some(new_pdpt))
            };
            write_owned_entry(
                new_pml4,
                0,
                new_pdpt | PTE_PRESENT | PTE_WRITABLE | PTE_USER,
            );
        }
    });

    // Map the Local APIC MMIO page (supervisor-only) so interrupt EOIs issued while this address
    // space is active do not fault. This is only needed on the WHP backend, where the kernel drives
    // the LAPIC directly for timer delivery and EOI. On the KVM backend the kernel uses the legacy
    // 8259 PIC together with the PIT periodic timer (mirroring the 32-bit x86 path), so it never
    // touches the LAPIC MMIO page and the mapping is omitted.
    #[cfg(feature = "whp")]
    {
        let lapic: usize = ::config::microvm::DEFAULT_LAPIC_BASE;
        proof_decl! {
            let tracked manager_inv:
                &::vstd::invariant::LocalInvariant<
                    (),
                    HwptManagerState,
                    HwptManagerInvariant,
                > = manager.borrow();
        }
        open_local_invariant!(manager_inv => manager_state => {
            unsafe {
                proof_with! {
                    Tracked(&mut manager_state.available_pages),
                    Tracked(private_pages)
                };
                map_in(new_pml4, lapic, lapic, false, true);
            }
        });
    }

    new_pml4
}

/// Maps a single 4 KiB user page `vaddr` → `paddr` (User-accessible) in the given per-process PML4.
///
/// # Safety
///
/// `pml4` must be a valid PML4 physical address from [`create_user_pml4`].
#[verus_spec(
    with
        Tracked(manager):
            Tracked<&HwptManagerHandle>,
        Tracked(private_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
    requires
        owned_hw_pages_inv(*old(private_pages)),
        old(private_pages).dom().contains(pml4),
        old(private_pages)[pml4].level() == HwPagingLevel::Pml4,
        vaddr >= (1usize << 30),
    ensures
        owned_hw_pages_inv(*final(private_pages)),
)]
pub unsafe fn map_user(pml4: u64, vaddr: usize, paddr: usize, writable: bool) {
    proof_decl! {
        let tracked manager_inv:
            &::vstd::invariant::LocalInvariant<
                (),
                HwptManagerState,
                HwptManagerInvariant,
            > = manager.borrow();
    }
    open_local_invariant!(manager_inv => manager_state => {
        unsafe {
            proof_with! {
                Tracked(&mut manager_state.available_pages),
                Tracked(private_pages)
            };
            map_in(pml4, vaddr, paddr, true, writable);
        }
    });
}

/// Unmaps a single 4 KiB user page at `vaddr` in the given per-process PML4.
///
/// # Safety
///
/// `pml4` must be a valid PML4 physical address from [`create_user_pml4`].
#[verus_spec(
    with
        Tracked(private_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
    requires
        owned_hw_pages_inv(*old(private_pages)),
        old(private_pages).dom().contains(pml4),
        old(private_pages)[pml4].level() == HwPagingLevel::Pml4,
        vaddr >= (1usize << 30),
    ensures
        owned_hw_pages_inv(*final(private_pages)),
)]
pub unsafe fn unmap_user(pml4: u64, vaddr: usize) {
    unsafe {
        proof_with! {
            Tracked(private_pages)
        };
        unmap_in(pml4, vaddr);
    }
}

/// Maps (user-accessible) a single 4 KiB page `vaddr` → `paddr` into the boot PML4's shared kernel
/// low-memory page directory (`PDPT[0]` → boot `PD0`). Because every per-process PML4 references
/// that same boot `PD0`, this makes the page visible — with the same permissions — in all address
/// spaces, mirroring how the kernel's shared identity-mapped page tables behave on 32-bit targets.
///
/// This is used by `kctrl()` to expose MMIO windows (e.g. the RAMFS) that live in low physical
/// memory to user processes. Mapping at 4 KiB granularity transparently splits the covering 2 MiB
/// identity page in boot `PD0` while preserving the surrounding supervisor-only kernel mappings.
///
/// # Safety
///
/// Must be called after [`init`]. `vaddr` must lie within the boot `PD0` coverage (the low 1 GiB).
#[verus_spec(
    with
        Tracked(manager):
            Tracked<&HwptManagerHandle>,
)]
pub unsafe fn map_kernel_mmio(vaddr: usize, paddr: usize, writable: bool) {
    assert_initialized();
    let pml4: u64 = boot_pml4_paddr();
    proof_decl! {
        let tracked manager_inv:
            &::vstd::invariant::LocalInvariant<
                (),
                HwptManagerState,
                HwptManagerInvariant,
            > = manager.borrow();
    }
    open_local_invariant!(manager_inv => manager_state => {
        unsafe {
            proof_with! {
                Tracked(&mut manager_state.available_pages),
                Tracked(&mut manager_state.boot_pages)
            };
            map_in(pml4, vaddr, paddr, true, writable);
        }
    });
}

/// Updates the writable permission of an already-mapped 4 KiB user page (used for copy-on-write).
/// If the page is not currently mapped at 4 KiB granularity, this is a no-op.
///
/// # Safety
///
/// `pml4` must be a valid PML4 physical address from [`create_user_pml4`].
#[verus_spec(
    with
        Tracked(private_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
    requires
        owned_hw_pages_inv(*old(private_pages)),
        old(private_pages).dom().contains(pml4),
        old(private_pages)[pml4].level() == HwPagingLevel::Pml4,
        vaddr >= (1usize << 30),
    ensures
        owned_hw_pages_inv(*final(private_pages)),
)]
pub unsafe fn protect_user(pml4: u64, vaddr: usize, writable: bool) {
    let pml4_idx: usize = (vaddr >> 39) & 0x1FF;
    let pdpt_idx: usize = (vaddr >> 30) & 0x1FF;
    let pd_idx: usize = (vaddr >> 21) & 0x1FF;
    let pt_idx: usize = (vaddr >> 12) & 0x1FF;

    proof_with! {
        Tracked(&*private_pages)
    };
    let pml4_entry: u64 = read_owned_entry(pml4, pml4_idx);
    if pml4_entry & PTE_PRESENT == 0 {
        return;
    }
    let pdpt: u64 = pml4_entry & ADDR_MASK_4K;
    proof_with! {
        Tracked(&*private_pages)
    };
    let pdpt_entry: u64 = read_owned_entry(pdpt, pdpt_idx);
    if pdpt_entry & PTE_PRESENT == 0 {
        return;
    }
    let pd: u64 = pdpt_entry & ADDR_MASK_4K;
    proof_with! {
        Tracked(&*private_pages)
    };
    let pd_entry: u64 = read_owned_entry(pd, pd_idx);
    if pd_entry & PTE_PRESENT == 0 || pd_entry & PDE_PS != 0 {
        return;
    }
    let pt: u64 = pd_entry & ADDR_MASK_4K;
    proof_with! {
        Tracked(&*private_pages)
    };
    let pte: u64 = read_owned_entry(pt, pt_idx);
    if pte & PTE_PRESENT == 0 {
        return;
    }
    let new_pte: u64 = if writable {
        pte | PTE_WRITABLE
    } else {
        pte & !PTE_WRITABLE
    };
    proof_with! {
        Tracked(private_pages),
        Ghost(None)
    };
    write_owned_entry(pt, pt_idx, new_pte);
    invlpg(vaddr);
}

/// Tears down a per-process PML4, returning every process-owned page-table page to the free list.
///
/// The shared kernel `PD0` (referenced by `PDPT[0]`) is never freed; only the process-private
/// `PDPT[1..]` subtrees (user space plus the per-process LAPIC tables) and the `PDPT`/`PML4`
/// pages themselves are reclaimed.
///
/// # Safety
///
/// `pml4` must be a valid PML4 physical address from [`create_user_pml4`] that is no longer loaded
/// in any `CR3`.
#[verus_spec(
    with
        Tracked(manager):
            Tracked<&HwptManagerHandle>,
        Tracked(private_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
    requires
        owned_hw_pages_inv(*old(private_pages)),
        old(private_pages).dom().contains(pml4),
        old(private_pages)[pml4].level() == HwPagingLevel::Pml4,
    ensures
        final(private_pages).dom().is_empty(),
)]
pub unsafe fn destroy_user_pml4(pml4: u64) {
    let pml4_entry: u64 = unsafe {
        proof_with! {
            Tracked(&*private_pages)
        };
        read_owned_entry(pml4, 0)
    };
    if pml4_entry & PTE_PRESENT != 0 {
        let pdpt: u64 = pml4_entry & ADDR_MASK_4K;
        // Skip PDPT[0] (shared kernel PD0); free all process-private PDPT[1..] subtrees.
        for pdpt_i in 1..ENTRIES_PER_TABLE {
            let pdpt_entry: u64 = unsafe {
                proof_with! {
                    Tracked(&*private_pages)
                };
                read_owned_entry(pdpt, pdpt_i)
            };
            if pdpt_entry & PTE_PRESENT != 0 && pdpt_entry & PDE_PS == 0 {
                let pd: u64 = pdpt_entry & ADDR_MASK_4K;
                for pd_i in 0..ENTRIES_PER_TABLE {
                    let pd_entry: u64 = unsafe {
                        proof_with! {
                            Tracked(&*private_pages)
                        };
                        read_owned_entry(pd, pd_i)
                    };
                    if pd_entry & PTE_PRESENT != 0 && pd_entry & PDE_PS == 0 {
                        unsafe {
                            proof_with! {
                                Tracked(private_pages),
                                Ghost(None)
                            };
                            write_owned_entry(pd, pd_i, 0);
                        }
                        unsafe {
                            proof_with! {
                                Tracked(manager),
                                Tracked(private_pages)
                            };
                            return_owned_pt_page(pd_entry & ADDR_MASK_4K);
                        }
                    }
                }
                unsafe {
                    proof_with! {
                        Tracked(private_pages),
                        Ghost(None)
                    };
                    write_owned_entry(pdpt, pdpt_i, 0);
                }
                unsafe {
                    proof_with! {
                        Tracked(manager),
                        Tracked(private_pages)
                    };
                    return_owned_pt_page(pd);
                }
            }
        }
        unsafe {
            proof_with! {
                Tracked(private_pages),
                Ghost(None)
            };
            write_owned_entry(pml4, 0, 0);
        }
        unsafe {
            proof_with! {
                Tracked(manager),
                Tracked(private_pages)
            };
            return_owned_pt_page(pdpt);
        }
    }
    unsafe {
        proof_with! {
            Tracked(manager),
            Tracked(private_pages)
        };
        return_owned_pt_page(pml4);
    }
}

/// Unmaps a single 4 KiB page at `vaddr` using the given PML4.
///
/// # Safety
///
/// Caller must ensure `vaddr` is page-aligned, `pml4` is a valid PML4 physical address.
#[verus_spec(
    with
        Tracked(owner_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
    requires
        owned_hw_pages_inv(*old(owner_pages)),
        old(owner_pages).dom().contains(pml4),
        old(owner_pages)[pml4].level() == HwPagingLevel::Pml4,
    ensures
        owned_hw_pages_inv(*final(owner_pages)),
)]
unsafe fn unmap_in(pml4: u64, vaddr: usize) {
    let pml4_idx: usize = (vaddr >> 39) & 0x1FF;
    let pdpt_idx: usize = (vaddr >> 30) & 0x1FF;
    let pd_idx: usize = (vaddr >> 21) & 0x1FF;
    let pt_idx: usize = (vaddr >> 12) & 0x1FF;

    // Walk the hierarchy — if any level is missing, the page was never mapped.
    proof_with! {
        Tracked(&*owner_pages)
    };
    let pml4_entry: u64 = read_owned_entry(pml4, pml4_idx);
    if pml4_entry & PTE_PRESENT == 0 {
        return;
    }
    let pdpt: u64 = pml4_entry & ADDR_MASK_4K;

    proof_with! {
        Tracked(&*owner_pages)
    };
    let pdpt_entry: u64 = read_owned_entry(pdpt, pdpt_idx);
    if pdpt_entry & PTE_PRESENT == 0 {
        return;
    }
    let pd: u64 = pdpt_entry & ADDR_MASK_4K;

    proof_with! {
        Tracked(&*owner_pages)
    };
    let pd_entry: u64 = read_owned_entry(pd, pd_idx);
    if pd_entry & PTE_PRESENT == 0 || pd_entry & PDE_PS != 0 {
        // Not present or still a 2 MiB page — nothing to unmap at 4 KiB granularity.
        return;
    }
    let pt: u64 = pd_entry & ADDR_MASK_4K;

    // Clear the PT entry.
    proof_with! {
        Tracked(owner_pages),
        Ghost(None)
    };
    write_owned_entry(pt, pt_idx, 0);
    invlpg(vaddr);
}

//==================================================================================================
// Per-Process Page Tables
//==================================================================================================

/// Physical address of the boot PD0 (supervisor-only, maps 0–1 GiB kernel space).
/// Discovered from the boot PML4 during `init()`.
static mut BOOT_PD0_PADDR: u64 = 0;

/// Maps a single 4 KiB page in a specific PML4 hierarchy.
#[verus_spec(
    with
        Tracked(available_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
        Tracked(owner_pages):
            Tracked<&mut Map<u64, NanvixHwPageToken>>,
    requires
        owned_hw_pages_inv(*old(owner_pages)),
        old(owner_pages).dom().contains(pml4),
        old(owner_pages)[pml4].level() == HwPagingLevel::Pml4,
    ensures
        owned_hw_pages_inv(*final(owner_pages)),
)]
unsafe fn map_in(pml4: u64, vaddr: usize, paddr: usize, user: bool, writable: bool) {
    let pml4_idx: usize = (vaddr >> 39) & 0x1FF;
    let pdpt_idx: usize = (vaddr >> 30) & 0x1FF;
    let pd_idx: usize = (vaddr >> 21) & 0x1FF;
    let pt_idx: usize = (vaddr >> 12) & 0x1FF;

    // Walk/create PML4 → PDPT.
    proof_with! {
        Tracked(available_pages),
        Tracked(owner_pages),
        Ghost(HwPagingLevel::Pml4)
    };
    let pdpt: u64 = ensure_table(pml4, pml4_idx, user);

    // Walk/create PDPT → PD.
    proof_with! {
        Tracked(available_pages),
        Tracked(owner_pages),
        Ghost(HwPagingLevel::Pdpt)
    };
    let pd: u64 = ensure_table(pdpt, pdpt_idx, user);

    // Check if PD entry is a 2 MiB page (needs splitting).
    proof_with! {
        Tracked(&*owner_pages)
    };
    let pd_entry: u64 = read_owned_entry(pd, pd_idx);
    let pt: u64 = if pd_entry & PTE_PRESENT != 0 && pd_entry & PDE_PS != 0 {
        proof_with! {
            Tracked(available_pages),
            Tracked(owner_pages)
        };
        let pt_addr: u64 = split_2m_entry(pd, pd_idx);
        if user {
            proof_with! {
                Tracked(&*owner_pages)
            };
            let new_pd_entry: u64 = read_owned_entry(pd, pd_idx);
            if new_pd_entry & PTE_USER == 0 {
                proof_with! {
                    Tracked(owner_pages),
                    Ghost(Some(pt_addr))
                };
                write_owned_entry(pd, pd_idx, new_pd_entry | PTE_USER);
            }
        }
        pt_addr
    } else if pd_entry & PTE_PRESENT != 0 {
        if user && (pd_entry & PTE_USER == 0) {
            proof_with! {
                Tracked(owner_pages),
                Ghost(Some(pd_entry & ADDR_MASK_4K))
            };
            write_owned_entry(pd, pd_idx, pd_entry | PTE_USER);
        }
        pd_entry & ADDR_MASK_4K
    } else {
        proof_with! {
            Tracked(available_pages),
            Tracked(owner_pages),
            Ghost(HwPagingLevel::Pd)
        };
        ensure_table(pd, pd_idx, user)
    };

    // Build the PT entry.
    let mut flags: u64 = PTE_PRESENT;
    if writable {
        flags |= PTE_WRITABLE;
    }
    if user {
        flags |= PTE_USER;
    }
    let pte: u64 = (paddr as u64 & ADDR_MASK_4K) | flags;
    proof_with! {
        Tracked(owner_pages),
        Ghost(None)
    };
    write_owned_entry(pt, pt_idx, pte);

    invlpg(vaddr);
}
