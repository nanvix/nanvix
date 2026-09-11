# Virtual Memory Interactions

This inventory treats an MMU interaction as an explicit access to paging structures,
translation-control registers, TLB state, or payload memory deliberately reached through those
translations. Counting ordinary instruction fetches and stack accesses would make virtually every
line an interaction.

Both x86 and x86_64 paths are included. A crucial distinction is that Nanvix's
`PageDirectory`/`PageTable` structures are directly hardware-walked on x86, but primarily logical
bookkeeping on x86_64; x86_64 hardware walks the separate hierarchy in `hwpt.rs`.

## MMU-Relevant Data Structures

- `Vmem` - `src/kernel/src/mm/virt/vmem.rs:90-106`
  - `pgdir`: root of the two-level logical/x86 hardware address space.
  - `kernel_page_tables`: owns shared kernel page-table pages and their virtual ranges.
  - `kernel_pages`: keeps mapped kernel frames alive.
  - `user_page_tables`: owns per-address-space user page tables.
  - `hw_pml4`: physical address of the actual x86_64 hardware PML4. Zero denotes the kernel
    address space.
- `PageDirectory<T>` -
  `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs:31-34`
  - Contains the raw PDE array potentially read and modified by the MMU.
- `PageTable<T>` - `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs:37-42`
  - `entries`: raw PTE array.
  - `nmapped`: software-only count controlling when a table can be disconnected and reclaimed.
- `PageDirectoryStorage` and `PageTableStorage` - `src/kernel/src/mm/virt/mod.rs:37-99`
  - Either BSS-backed arrays or page-sized physical frames.
  - Their `Deref`/`DerefMut` implementations expose those frames as raw paging-entry arrays.
- `KernelPage` and `KernelFrame` - `src/kernel/src/mm/virt/kpage.rs:16-19`,
  `src/kernel/src/mm/phys/kframe.rs:33-36`
  - Own the physical frames used as paging structures.
- `PAGE_TABLE_STORAGE` and `PAGE_TABLE_ALLOCATOR` -
  `src/kernel/src/mm/virt/page_table_allocator.rs:46-87`
  - Page-aligned BSS memory from which active paging structures are allocated.
- `PageDirectoryEntry` and `PageDirectoryEntryFlags` -
  `src/libs/arch/src/x86/mem/paging/pde.rs:32-53,256-263`
  - Frame address plus present, writable, user/supervisor, caching, accessed, dirty, and page-size
    fields.
- `PageTableEntry` and `PageTableEntryFlags` -
  `src/libs/arch/src/x86/mem/paging/pte.rs:35-56,257-264`
  - Frame address plus present, writable, user/supervisor, caching, accessed, dirty, and software
    copy-on-write fields.
- Paging flag encodings - `src/libs/arch/src/x86/mem/paging/flags.rs:20-136`
  - `PresentFlag`, `ReadWriteFlag`, `UserSupervisorFlag`, `PageWriteThroughFlag`,
    `PageCacheDisableFlag`, `AccessedFlag`, `DirtyFlag`, `PageSizeFlag`, and `CopyOnWriteFlag`.
  - Accessed and dirty bits are especially important because the MMU may write them.
  - Copy-on-write is OS-defined and ignored by the MMU, but shares the same MMU-visible entry word.
- `FrameNumber` and raw `PteWord`
  - Store physical targets encoded into paging entries.
- `Table<E>` - `src/libs/arch/src/x86/mem/paging/table.rs:83-91`
  - Non-owning pointer to a physical page-table page, used for volatile entry access.
- `Cr3Register` - `src/libs/arch/src/x86/cpu/cr3.rs:253-262`
  - Contains the active paging-root physical address and root caching controls.
- Identity-map state - `src/kernel/src/mm/virt/identity_map.rs:82-86`
  - `KERNEL_PD_PADDR`: physical address of the kernel page directory.
  - `KERNEL_CR3`: saved kernel address-space root.
- x86_64 hardware hierarchy - `src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs`
  - `PtPage`: one 4 KiB hardware paging-structure page, line 49.
  - `PT_POOL`: storage containing active and reusable paging structures, lines 52-55.
  - `PT_POOL_NEXT`, `PT_FREELIST`, and `PT_FREELIST_LEN`: paging-structure lifetime state,
    lines 58-65.
  - `PML4_PADDR`: boot hardware root read from CR3, line 68.
  - `BOOT_PD0_PADDR`: shared kernel page-directory address, line 430.
  - `INITIALIZED`: validity state for the above roots, line 71.
- The hardware TLB is relevant state but has no Rust representation. It is manipulated through
  CR3 writes and `invlpg`.

## Direct Interaction Sites in `vmem.rs`

### Address-Space Construction

- `vmem.rs:118-125`: allocates a page-directory page and passes it to `PageDirectory::new`, which
  clears every PDE.
- `vmem.rs:136-138`: obtains each kernel page table's physical address and installs it into the new
  page directory.
- `vmem.rs:152-159`: constructs the kernel CR3 value containing the page-directory physical address
  and caching flags.
- `vmem.rs:167`: passes the kernel page-directory address and CR3 value to the identity-map
  subsystem.
- `vmem.rs:190-191`: constructs and clears a cloned page directory.
- `vmem.rs:198-201`: installs shared kernel page-table frames into a cloned page directory.
- `vmem.rs:214-216`: synchronizes kernel PDEs into the cloned directory.
- `vmem.rs:223`: allocates an actual per-process x86_64 PML4 hierarchy.

### Active Paging Root

- `vmem.rs:235-237`: resolves the page-directory physical address and invokes
  `load_page_directory`.
  - On x86 this writes CR3 and enables paging.
  - On x86_64 the called implementation is intentionally a no-op.
- `vmem.rs:254-261`: returns the paging-root value that external context-switch code must load into
  CR3.
  - x86_64 returns `hw_pml4`.
  - x86 returns the physical `pgdir` address.

### x86_64 Hardware-Table Bridge

- `vmem.rs:268-278`: forwards user mappings to `hwpt::map_user`.
- `vmem.rs:289-297`: forwards user unmapping to `hwpt::unmap_user`.
- `vmem.rs:305-313`: forwards permission changes to `hwpt::protect_user`.
- `vmem.rs:323-329`: forwards shared MMIO mappings to `hwpt::map_kernel_mmio`.

### Kernel Page Mapping

- `vmem.rs:357`: reads the relevant PDE.
- `vmem.rs:371-376`: writes a PDE pointing to a newly allocated kernel page table.
- `vmem.rs:392-399`: writes a PTE mapping the kernel frame, including privilege, writable,
  write-through, and cache-disable state.
- `vmem.rs:405`: reloads the page-directory root to flush cached translations.

### Page-Table Allocation

- `vmem.rs:431-432`: wraps a kernel frame as page-table storage and clears it through
  `PageTable::new`.
- `vmem.rs:448-453`: allocates and clears a physical frame, wraps it as page-table storage, and
  clears its PTE array again.

### User Mappings

- `vmem.rs:478`: reads the PDE covering the requested user address.
- `vmem.rs:492-495`: obtains a new page table's physical address and writes it into the PDE.
- `vmem.rs:509`: writes the user PTE.
- `vmem.rs:512-516`: mirrors the mapping into the x86_64 hardware hierarchy.
- `vmem.rs:519-522`: transfers ownership of the mapped physical frame to the page-table state.

### Software Page-Table Walks

These read the same entry words that the MMU may read or modify:

- `vmem.rs:819`: reads a PTE to resolve a user virtual page to a frame.
- `vmem.rs:854-855`: reads presence and then the mapped frame.
- `vmem.rs:892`: reads and decodes a complete PTE.
- `vmem.rs:950-952`: scans all present PTEs in every user page table.
- `vmem.rs:1184`: obtains a PTE when resolving copy-on-write.
- `vmem.rs:1308-1314`: translates a user virtual address through Nanvix's software tables.

### Copy-on-Write Entry Changes

- `vmem.rs:1069`: clears writable and sets the software COW bit in the logical PTE.
- `vmem.rs:1072`: clears writable in the x86_64 hardware PTE.
- `vmem.rs:1106`: clears COW and restores writable in the logical PTE.
- `vmem.rs:1109`: restores writable in the x86_64 hardware PTE.
- `vmem.rs:1148-1149`: replaces the logical PTE's physical frame and permissions.
- `vmem.rs:1153`: replaces the x86_64 hardware PTE.
- `vmem.rs:1213-1215`: reads the old physical frame and writes the new frame's contents before
  changing the mapping.

### Payload-Memory Access Through Identity Mappings

- `vmem.rs:1392-1398`: reads a user frame and writes kernel memory.
- `vmem.rs:1565-1568`: reads kernel memory and writes a user frame.
- `vmem.rs:1707-1709`: reads one process's physical frame and writes another process's physical
  frame.
- `vmem.rs:1750-1753`: writes an entire mapped user frame.
- These accesses may cause page walks, TLB use, accessed-bit updates, dirty-bit updates, or faults.

### Unmapping and Reclamation

- `vmem.rs:1802`: reads the PDE.
- `vmem.rs:1825`: reads the PTE and checks its physical target.
- `vmem.rs:1832`: clears the PTE and invalidates its TLB entry.
- `vmem.rs:1838`: clears the x86_64 hardware PTE and invalidates the translation.
- `vmem.rs:1854`: clears an empty PDE and invalidates the associated translation.
- `vmem.rs:2052-2055`: reclaims the per-process x86_64 hierarchy. Its precondition is that the PML4
  is no longer installed in CR3.

### Permission and MMIO Control

- `vmem.rs:1880`: reads the user PDE.
- `vmem.rs:1902`: changes user PTE privilege/write permissions and invalidates the TLB.
- `vmem.rs:1953`: reads the kernel PDE.
- `vmem.rs:1976-1977,1984-1986`: reads the kernel PTE's presence.
- `vmem.rs:1990-1997`: creates an identity-mapped, user-accessible, cache-disabled MMIO PTE.
- `vmem.rs:2000`: changes an existing MMIO PTE's permissions.
- `vmem.rs:2009-2013`: mirrors the MMIO entry into the x86_64 hardware hierarchy.

One notable detail is that `Vmem::uctrl()` updates only the logical `PageTable`; unlike the COW
operations, it does not call `hw_protect_user`. Thus on x86_64 this particular path does not
directly update the hierarchy the hardware walks.

## Shared Page-Directory Implementation

`src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs`:

- `47-59`: reads the existing raw PDE before mapping.
- `69-93`: constructs a present PDE with the page-table frame and MMU-consumed flags.
- `96`: writes the PDE.
- `116-128`: reads a PDE before unmapping.
- `144-159`: constructs a non-present/null PDE.
- `162`: writes the cleared PDE.
- `168`: executes `invlpg`.
- `173-176`: zeroes every PDE when constructing a directory.
- `179-181`: raw PDE read.
- `184-187`: raw PDE write.
- `189-193`: computes the physical address later installed into CR3 or a higher-level entry.

The MMU may independently set accessed or dirty fields in the same raw words read and overwritten
here.

## Shared Page-Table Implementation

`src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs`:

- `72-87`: reads an existing PTE before mapping.
- `101-127`: constructs a present PTE with frame, privilege, writable, caching, accessed, and dirty
  state.
- `130`: writes the PTE.
- `153-165`: reads the PTE before unmapping.
- `181-194`: constructs a non-present/null PTE.
- `197`: clears the PTE.
- `202`: invalidates the TLB entry.
- `226-238`: reads a PTE for address translation.
- `262-268`: reads a PTE's presence.
- `284-323`: reads, modifies, and writes PTE permission bits.
- `327`: invalidates the permission-cached translation.
- `354-382`: reads and validates the PTE before COW marking.
- `384-386`: clears writable, sets COW, and writes the PTE.
- `390`: invalidates the translation.
- `413-441`: reads and validates a COW PTE.
- `443-445`: clears COW, restores writable, and writes the PTE.
- `448`: invalidates the translation.
- `475-510`: reads and validates the old COW mapping.
- `512-518`: constructs and writes a replacement PTE pointing to a new physical frame.
- `521`: invalidates the translation.
- `572-575`: reads raw entries to ensure a bulk-filled range is absent.
- `594-596`: writes contiguous identity-mapped PTEs.
- `602-605`: zeroes every PTE in a newly created table.
- `608-611`: central raw PTE read.
- `614-617`: central raw PTE write.
- `633-637`: reads a complete present PTE.
- `640-643`: resolves the table's physical address.
- `652-657`: reads and decodes every raw entry while iterating present mappings.

## Paging-Storage Access

`src/kernel/src/mm/virt/mod.rs`:

- `43-52`: exposes BSS or frame-backed PTE memory for reads.
- `57-66`: exposes the same memory for writes.
- `75-84`: exposes BSS or frame-backed PDE memory for reads.
- `89-98`: exposes the same memory for writes.

The unsafe slice construction at `49-50`, `63-64`, `81-82`, and `95-96` creates CPU-visible aliases
to physical pages that may simultaneously be read or updated by hardware page walks.

## Identity-Map Implementation

`src/kernel/src/mm/virt/identity_map.rs`:

- `122-127`: constructs a `Table<PageDirectoryEntry>` over the kernel page-directory memory and
  checks/creates each PDE.
- `133-134`: publishes the kernel page-directory address and CR3 shadow state.
- `235-237`: reads source memory and writes destination memory through identity mappings.
- `279-281`: writes payload memory through an identity mapping.
- `313-330`: volatile reads of source and target PDEs and conditional volatile PDE writes during
  address-space cloning.
- `347-349`: restores a saved CR3 when the guard is dropped.
- `395`: reads Nanvix's saved kernel CR3 value.
- `402`: reads the hardware CR3.
- `416`: writes the kernel CR3, changing the active address space and flushing non-global
  translations.
- `473-474`: creates a page-directory view over active paging memory.
- `506`: volatile PDE read.
- `548-555`: allocates zeroed BSS memory that will become a page table.
- `576`: volatile PDE write installing that page table.
- `598`: volatile PTE read.
- `630`: volatile PTE write installing an identity mapping.
- `645`: invalidates the corresponding TLB entry.
- `680-684`: creates volatile views over the kernel PD and selected PT before ensuring a mapping.

## Primitive Volatile Paging-Memory Access

`src/libs/arch/src/x86/mem/paging/table.rs`:

- `116-119`: calculates an entry address and performs `read_volatile`.
- `130-133`: calculates an entry address and performs `write_volatile`.

These are direct accesses to memory that may also be accessed by an MMU page walk.

## x86 Control-Register and TLB Operations

- `src/kernel/src/hal/arch/x86/mem/mmu/mod.rs:20-30`
  - Writes the page-directory physical address to CR3.
  - Reads and writes CR0 to set paging enabled.
  - The CR3 write changes the active translation root and flushes applicable TLB state.
- `src/libs/arch/src/x86/cpu/cr3.rs:365-379`
  - Reads hardware CR3.
- `src/libs/arch/src/x86/cpu/cr3.rs:394-405`
  - Writes hardware CR3.
- `src/libs/arch/src/x86/mem/paging/mod.rs:64-69`
  - Executes `invlpg` for a virtual page.

## x86_64 Hardware Page-Table Implementation

`src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs`:

- `83-108`: allocates a paging-structure page; reused pages are zeroed with volatile writes at
  line 91.
- `113-118`: places unreachable paging pages on the free list.
- `126-128`: volatile hardware-entry read.
- `137-139`: volatile hardware-entry write.
- `149-168`: reads or creates intermediate PML4/PDPT/PD entries and may add the user bit.
- `176-190`: reads a 2 MiB PDE, writes 512 replacement 4 KiB PTEs, then replaces the PDE.
- `196-197`: executes `invlpg`.
- `211-220`: reads hardware CR3, then walks PML4 and PDPT entries to discover the shared boot PD.
- `250-263`: allocates a process PML4 and PDPT and writes their root entries.
- `283-284`: maps a user page through the hardware hierarchy.
- `292-293`: unmaps a user page.
- `308-310`: maps shared low-memory MMIO.
- `319-350`: walks all four levels, reads the leaf PTE, changes writable state, writes it back, and
  invalidates the TLB.
- `363-383`: walks a detached hierarchy before reclaiming its paging-structure pages.
- `393-421`: walks the hierarchy, clears a leaf PTE, and invalidates the translation.
- `433-476`: creates or traverses every paging level, optionally splits a large page, writes the
  final PTE, and invalidates the translation.

The `destroy_user_pml4` reads are MMU-related even though the table must no longer be reachable by
the MMU at that point; that unreachability is its central safety precondition.

## Deliberate Target-Memory Accesses

After establishing identity mappings, Nanvix accesses the translated payload frames through
assembly:

- x86 copy reads and writes:
  `src/kernel/src/hal/arch/x86/asm/fast_memcpy.rs:36-66`.
- x86 fill writes:
  `src/kernel/src/hal/arch/x86/asm/fast_memset.rs:38-62`.
- x86_64 copy reads and writes:
  `src/kernel/src/hal/arch/x86_64/asm/fast_memcpy.rs:35-51`.
- x86_64 fill writes:
  `src/kernel/src/hal/arch/x86_64/asm/fast_memset.rs:37-53`.
- `KernelFrame::new` ensures a frame is identity-mapped before dereferencing it:
  `src/kernel/src/mm/phys/kframe.rs:50-60`.
- `KernelFrame::clear` writes the frame through the identity-map subsystem:
  `src/kernel/src/mm/phys/kframe.rs:92-98`.

These operations interact with the MMU indirectly: the MMU translates their source and destination
addresses, may read paging entries, may set accessed/dirty bits, and may fault if the required
mapping is absent or insufficient.
