// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(clippy::expect_used)]

use super::*;

/// Page-table storage deliberately offset from a page boundary.
#[repr(C, align(4096))]
struct UnalignedDirectory {
    padding: PteWord,
    entries: [PteWord; PAGE_TABLE_LENGTH],
}

/// Used only by sequential in-kernel tests, with no overlapping borrows.
static mut DIRECTORY: UnalignedDirectory = UnalignedDirectory {
    padding: 0,
    entries: [0; PAGE_TABLE_LENGTH],
};

/// Checks that a failed CR3 address conversion cannot publish a kernel mapping.
fn check_bad_directory(existing_table: bool) -> bool {
    // SAFETY: in-kernel tests run sequentially with interrupts disabled. This Vmem is dropped
    // before the next test reuses the storage, and its unaligned directory is never loaded.
    let entries: &'static mut [PteWord; PAGE_TABLE_LENGTH] =
        unsafe { &mut *::core::ptr::addr_of_mut!(DIRECTORY.entries) };
    let mut vmem: Vmem = Vmem {
        pgdir: PageDirectory::new(PageDirectoryStorage::Bss(entries)),
        kernel_page_tables: LinkedList::new(),
        kernel_pages: LinkedList::new(),
        user_page_tables: LinkedList::new(),
        #[cfg(target_arch = "x86_64")]
        hw_pml4: 0,
    };
    let vaddr: PageAligned<VirtualAddress> =
        PageAligned::from_raw_value(0).expect("invalid test page address");
    let pt_vaddr: PageTableAddress = PageTableAddress::new(
        PageTableAligned::from_raw_value(0).expect("invalid test page table address"),
    );

    if existing_table {
        let page_table: PageTable<PageTableStorage> =
            Vmem::allocate_kernel_page_table().expect("failed to allocate test page table");
        vmem.pgdir
            .map(
                pt_vaddr,
                page_table
                    .physical_address()
                    .expect("invalid test page table physical address"),
                false,
                AccessPermission::RDWR,
            )
            .expect("failed to install test page table");
        vmem.kernel_page_tables
            .push_back(Rc::new(RefCell::new((pt_vaddr, page_table))));
    }

    let pde_before: PteWord = vmem
        .pgdir
        .read_pde(pt_vaddr)
        .expect("failed to read test PDE")
        .into_raw_value();
    let table_count: usize = vmem.kernel_page_tables.len();
    // SAFETY: the memory manager is initialized and these tests run without concurrent access.
    let kpage: KernelPage = unsafe { VirtMemoryManager::get_mut() }
        .alloc_kpage(true)
        .expect("failed to allocate test kernel page");

    let error: Error = vmem
        .map_kpage(kpage, vaddr)
        .expect_err("mapping with an unaligned page directory must fail");
    assert_eq!(error.code, ErrorCode::BadAddress, "unexpected mapping error");
    assert_eq!(
        vmem.pgdir
            .read_pde(pt_vaddr)
            .expect("failed to read test PDE after mapping")
            .into_raw_value(),
        pde_before,
        "failed mapping changed the page directory"
    );
    assert_eq!(
        vmem.kernel_page_tables.len(),
        table_count,
        "failed mapping retained a new page table"
    );
    assert!(vmem.kernel_pages.is_empty(), "failed mapping retained the kernel page");
    if let Some(entry) = vmem.kernel_page_tables.front() {
        let page_table: &PageTable<PageTableStorage> = &entry.borrow().1;
        assert_eq!(page_table.nmapped(), 0, "failed mapping changed the page count");
        assert!(
            !page_table
                .is_page_present(PageAddress::new(vaddr))
                .expect("failed to read test PTE"),
            "failed mapping published a page table entry"
        );
    }

    true
}

/// Rejects an invalid directory before publishing a newly allocated page table.
fn test_map_kpage_bad_directory_new_table() -> bool {
    check_bad_directory(false)
}

/// Rejects an invalid directory before modifying an existing shared page table.
fn test_map_kpage_bad_directory_existing_table() -> bool {
    check_bad_directory(true)
}

/// Exercises new and existing table mappings and preserves them when a duplicate is rejected.
fn test_map_kpage_success(root: &Vmem) -> bool {
    let original_pgdir: FrameAddress = root
        .pgdir
        .physical_address()
        .expect("invalid root page directory address");
    // SAFETY: the memory manager is initialized and these tests run without concurrent access.
    let pgdir_page: KernelPage = unsafe { VirtMemoryManager::get_mut() }
        .alloc_kpage(true)
        .expect("failed to allocate test page directory");
    let mut vmem: Vmem = Vmem::clone(root, pgdir_page).expect("failed to clone root address space");
    let table_count: usize = vmem.kernel_page_tables.len();
    let page_count: usize = vmem.kernel_pages.len();
    let pt_vaddr: PageTableAddress = PageTableAddress::new(
        PageTableAligned::from_raw_value(::config::kernel::MEMORY_SIZE)
            .expect("invalid test page table address"),
    );
    assert!(
        !vmem
            .pgdir
            .read_pde(pt_vaddr)
            .expect("failed to read test PDE")
            .is_present(),
        "test range must not already be mapped"
    );

    for offset in [0, ::arch::mem::PAGE_SIZE] {
        let vaddr: PageAligned<VirtualAddress> =
            PageAligned::from_raw_value(pt_vaddr.into_raw_value() + offset)
                .expect("invalid test page address");
        // SAFETY: the memory manager is initialized and access is exclusive.
        let kpage: KernelPage = unsafe { VirtMemoryManager::get_mut() }
            .alloc_kpage(true)
            .expect("failed to allocate test kernel page");
        let frame: FrameAddress = kpage.frame_address();
        vmem.map_kpage(kpage, vaddr)
            .expect("failed to map test kernel page");

        assert_eq!(vmem.kernel_page_tables.len(), table_count + 1, "unexpected table count");
        assert_eq!(
            vmem.kernel_pages.len(),
            page_count + offset / ::arch::mem::PAGE_SIZE + 1,
            "mapped page is not retained"
        );
        let entry: &Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>> = vmem
            .kernel_page_tables
            .back()
            .expect("missing new page table");
        let page_table: &PageTable<PageTableStorage> = &entry.borrow().1;
        assert_eq!(
            page_table
                .lookup(PageAddress::new(vaddr))
                .expect("missing mapped page"),
            frame,
            "mapped page has the wrong frame"
        );
        assert_eq!(
            vmem.pgdir
                .read_pde(pt_vaddr)
                .expect("missing test PDE")
                .frame_address(),
            page_table
                .physical_address()
                .expect("invalid test table address")
                .into_raw_value(),
            "directory does not point to the new page table"
        );
    }

    let vaddr: PageAligned<VirtualAddress> =
        PageAligned::from_raw_value(pt_vaddr.into_raw_value()).expect("invalid test page address");
    let original_frame: FrameAddress = vmem
        .kernel_page_tables
        .back()
        .expect("missing test table")
        .borrow()
        .1
        .lookup(PageAddress::new(vaddr))
        .expect("missing original mapping");
    // SAFETY: the memory manager is initialized and access is exclusive.
    let duplicate: KernelPage = unsafe { VirtMemoryManager::get_mut() }
        .alloc_kpage(true)
        .expect("failed to allocate duplicate kernel page");
    let error: Error = vmem
        .map_kpage(duplicate, vaddr)
        .expect_err("duplicate mapping must fail");
    assert_eq!(error.code, ErrorCode::ResourceBusy, "unexpected duplicate mapping error");
    assert_eq!(vmem.kernel_page_tables.len(), table_count + 1, "duplicate changed table count");
    assert_eq!(vmem.kernel_pages.len(), page_count + 2, "duplicate retained an extra page");
    assert_eq!(
        vmem.kernel_page_tables
            .back()
            .expect("missing test table")
            .borrow()
            .1
            .nmapped(),
        2,
        "duplicate changed the mapped page count"
    );
    assert_eq!(
        vmem.kernel_page_tables
            .back()
            .expect("missing test table")
            .borrow()
            .1
            .lookup(PageAddress::new(vaddr))
            .expect("missing original mapping after duplicate"),
        original_frame,
        "duplicate changed the original mapping"
    );

    // SAFETY: restore the live root directory before dropping the temporary address space
    // whose page directory map_kpage() loaded on x86.
    unsafe { mmu::load_page_directory(original_pgdir.into_raw_value()) };
    info!("passed: test_map_kpage_success");
    true
}

/// Checks user mappings across new and existing tables, including duplicate-frame ownership.
fn test_map_user_pages(root: &Vmem) -> bool {
    // SAFETY: the memory manager is initialized and these tests run without concurrent access.
    let pgdir_page: KernelPage = unsafe { VirtMemoryManager::get_mut() }
        .alloc_kpage(true)
        .expect("failed to allocate test page directory");
    let mut vmem: Vmem = Vmem::clone(root, pgdir_page).expect("failed to clone root address space");
    let base: usize = config::memory_layout::USER_BASE.into_raw_value();

    for (offset, table_count, page_count, access) in [
        (0, 1, 1, AccessPermission::RDONLY),
        (mem::PAGE_SIZE, 1, 2, AccessPermission::RDWR),
        (mem::PGTAB_SIZE, 2, 1, AccessPermission::RDWR),
    ] {
        let vaddr: PageAligned<VirtualAddress> =
            PageAligned::from_raw_value(base + offset).expect("invalid test user address");
        // SAFETY: the physical memory manager is initialized and access is exclusive.
        let owner: UserFrame = unsafe { PhysMemoryManager::get_mut() }
            .alloc_user_frame()
            .expect("failed to allocate test user frame");
        vmem.map(owner.share().expect("failed to share test frame"), vaddr, access)
            .expect("failed to map test user frame");
        assert_eq!(owner.refcount().expect("failed to read frame refcount"), 2);
        assert_eq!(vmem.user_page_tables.len(), table_count, "unexpected user table count");

        let (pt_vaddr, page_table): &(PageTableAddress, PageTable<PageTableStorage>) = vmem
            .user_page_tables
            .front()
            .expect("missing test user page table");
        let pte: PageTableEntry = page_table
            .read_pte_at(PageAddress::new(vaddr))
            .expect("missing test user PTE");
        assert_eq!(page_table.nmapped(), page_count, "unexpected mapped page count");
        assert_eq!(
            page_table
                .lookup(PageAddress::new(vaddr))
                .expect("missing test user frame"),
            owner.address(),
            "user mapping points to the wrong frame"
        );
        assert!(pte.flags().is_user(), "user mapping has supervisor permissions");
        assert_eq!(pte.flags().is_writable(), access.is_writable(), "wrong user permissions");
        assert_eq!(
            vmem.pgdir
                .read_pde(*pt_vaddr)
                .expect("missing test user PDE")
                .frame_address(),
            page_table
                .physical_address()
                .expect("invalid test user table address")
                .into_raw_value(),
            "directory does not point to the retained user table"
        );
    }

    let vaddr: PageAligned<VirtualAddress> =
        PageAligned::from_raw_value(base).expect("invalid duplicate test address");
    // SAFETY: the physical memory manager is initialized and access is exclusive.
    let rejected: UserFrame = unsafe { PhysMemoryManager::get_mut() }
        .alloc_user_frame()
        .expect("failed to allocate duplicate user frame");
    let original: FrameAddress = vmem
        .try_find_user_frame(vaddr)
        .expect("failed to find original user frame")
        .expect("original user page is not mapped");
    let error: Error = vmem
        .map(
            rejected.share().expect("failed to share duplicate frame"),
            vaddr,
            AccessPermission::RDWR,
        )
        .expect_err("duplicate user mapping must fail");
    assert_eq!(error.code, ErrorCode::ResourceBusy, "unexpected duplicate mapping error");
    assert_eq!(
        rejected
            .refcount()
            .expect("failed to read rejected frame refcount"),
        1,
        "failed mapping retained the rejected frame"
    );
    assert_eq!(vmem.user_page_tables.len(), 2, "duplicate retained an extra user table");
    let page_table: &mut PageTable<PageTableStorage> = vmem
        .lookup_user_page_table(PageTableAddress::new(
            PageTableAligned::from_raw_value(base).expect("invalid test table address"),
        ))
        .expect("missing original user table");
    assert_eq!(page_table.nmapped(), 2, "duplicate changed the mapped page count");
    assert_eq!(
        page_table
            .lookup(PageAddress::new(vaddr))
            .expect("missing original mapping after duplicate"),
        original,
        "duplicate replaced the original user frame"
    );
    assert!(
        !page_table
            .read_pte_at(PageAddress::new(vaddr))
            .expect("missing original user PTE")
            .flags()
            .is_writable(),
        "duplicate changed the original permissions"
    );

    vmem.clear_user_space()
        .expect("failed to reclaim test user mappings");
    assert!(vmem.user_page_tables.is_empty(), "unmapping retained empty user tables");
    info!("passed: test_map_user_pages");
    true
}

/// Runs the virtual address space mapping regression tests.
pub fn test(root: &Vmem) -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_map_kpage_bad_directory_new_table);
    passed &= run_test!(test_map_kpage_bad_directory_existing_table);
    passed &= test_map_kpage_success(root);
    passed &= test_map_user_pages(root);
    passed
}
