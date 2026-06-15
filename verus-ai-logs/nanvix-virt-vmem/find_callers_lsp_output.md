# Caller Analysis (LSP): vmem.rs

- **Source file:** `/home/ruize/nanvix-virt/src/kernel/src/mm/virt/vmem.rs`
- **Project dir:** `/home/ruize/nanvix-virt`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 35 |
| Public / trait-pub | 26 |
| Private | 9 |
| Types | 1 |

## Implicit Callers (runtime/compiler)

### impl Drop for Vmem
- **Trait:** `Drop`
- **Description:** Compiler inserts call to drop() when value goes out of scope
- **Methods dispatched:** `drop`

> These functions have **no explicit call sites** in source code. The Rust runtime dispatches to them via vtable / lang items.

## Public API — External Callers

### `is_user_page_mapped` (impl `Vmem`) [pub] — 2 external caller(s)
```
pub fn is_user_page_mapped(&self, vaddr: PageAligned<VirtualAddress>) -> Result<bool, Error>
```
> 
# Description

Checks whether a user page is currently mapped at the given virtual address.

# Parameters

- `vaddr`: Virtual address of the page to check.

# Returns

Returns `Ok(true)` if the page is mapped, `Ok(false)` if it is not, or `Err(_)` on
unexpected failures.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 468 | `match parent.is_user_page_mapped(vaddr) {` |
| `src/kernel/src/mm/virt/manager.rs` | 634 | `if vmem.is_user_page_mapped(check_addr)? {` |


### `is_user_region` (impl `Vmem`) [pub] — 6 external caller(s)
```
pub fn is_user_region(start: VirtualAddress, size: usize) -> bool
```
> 
# Description

Asserts whether a memory region lies entirely in user space.

# Parameters

- `start`: Starting virtual address of the region.
- `size`: Size of the region in bytes.

# Returns

Returns `true` if the entire region lies in user space, `false` otherwise.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/pm/kcall/create_thread.rs` | 62 | `if !Vmem::is_user_region(unsafe_thread_create_args, size_of::<ThreadCreateArgs>(` |
| `src/kernel/src/pm/kcall/create_thread.rs` | 89 | `if !Vmem::is_user_region(thread_create_args.user_stack_base, thread_create_args.` |
| `src/kernel/src/pm/process/manager/mod.rs` | 259 | `debug_assert!(Vmem::is_user_region(args.user_stack_base, args.user_stack_size));` |
| `src/kernel/src/pm/process/manager/mod.rs` | 750 | `if !Vmem::is_user_region(args.user_stack_base, args.user_stack_size) {` |
| `src/kernel/src/mm/virt/manager.rs` | 624 | `if !Vmem::is_user_region(vaddr.into_inner(), range_size) {` |
| `src/kernel/src/pm/kcall/duplicate.rs` | 66 | `if !Vmem::is_user_region(unsafe_args, size_of::<ThreadCreateArgs>()) {` |

*Internal callers (4):*
- **Vmem::copy_from_user_unaligned** (L1122): `if !Self::is_user_region(src, size) {`
- **Vmem::copy_to_user_unaligned_unchecked** (L1248): `if !Self::is_user_region(dst, size) {`
- **Vmem::copy_user_to_user** (L1430): `if !Self::is_user_region(src, size) {`
- **Vmem::copy_user_to_user** (L1437): `if !Self::is_user_region(dst, size) {`

### `copy_from_user_unaligned` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn copy_from_user_unaligned(
        &self,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error>
```
> 
# Description

Copies data from user space to kernel space. The source and destination addresses do not
have to be aligned, but the source address range must lie in user space, and the destination
address range must lie in kernel space.

# Parameters

- `dst`: Destination address in kernel space.
- `src`: Source address in user space.
- `size`: Number of bytes to copy.

# Returns

Upon successful completion, this function returns empty. Upon failure, this function returns
an error that indicates the reason for the failure.

# Errors

This function fails with the following error codes:
- [`ErrorCode::InvalidArgument`]: The size of the copy is zero.
- [`ErrorCode::BadAddress`]: The source memory region does not lie in user space.
- [`ErrorCode::BadAddress`]: The destination memory region does not lie in kernel space.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/pm/process/state/mod.rs` | 347 | `self.vmem.copy_from_user_unaligned(dst, src, size)` |


### `copy_to_user_unaligned` (impl `Vmem`) [pub] — 3 external caller(s)
```
pub fn copy_to_user_unaligned(
        &mut self,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error>
```
> 
# Description

Copies data from kernel space to user space. The source and destination addresses do not
have to be aligned, but the destination address range must lie in user space, and the source
address range must lie in kernel space.

Unlike [`Self::copy_to_user_unaligned_unchecked`], this function performs a dry run first to
check for errors before performing the actual copy. If any error occurs during the dry run,
it returns an error without performing the copy. If the dry run is successful, it proceeds
to perform the actual copy operation.

# Parameters

- `dst`: Destination address in user space.
- `src`: Source address in kernel space.
- `size`: Number of bytes to copy.

# Return Value

Upon successful completion, this function returns empty. Upon failure, this function returns
an error that indicates the reason for the failure.




| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/pm/process/state/mod.rs` | 356 | `self.vmem.copy_to_user_unaligned(dst, src, size)` |
| `src/kernel/src/pm/process/manager/mod.rs` | 480 | `vmem.copy_to_user_unaligned(dest, VirtualAddress::new(s.as_ptr() as usize), s.le` |
| `src/kernel/src/pm/process/manager/mod.rs` | 484 | `vmem.copy_to_user_unaligned(nul_vaddr, VirtualAddress::new(&NUL as *const u8 as ` |


### `unmap` (impl `Vmem`) [pub] — 4 external caller(s)
```
pub fn unmap(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<UserFrame>, Error>
```
> 
# Description

Unmaps a page from the target virtual address space.

If the page is not present (e.g., was never demand-paged), `Ok(None)` is returned without
logging any errors. This makes the method suitable for cleaning up lazily-allocated regions
such as user stacks.

# Parameters

- `vaddr`: Virtual address of the target page.

# Returns

- `Ok(Some(frame))` if the page was present and has been unmapped.
- `Ok(None)` if the page was not present.
- `Err(_)` on unexpected failures.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 396 | `if let Err(re) = child.unmap(vaddr) {` |
| `src/kernel/src/mm/virt/manager.rs` | 416 | `if let Err(re) = child.unmap(vaddr) {` |
| `src/kernel/src/mm/virt/manager.rs` | 489 | `if let Err(re) = child.unmap(vaddr) {` |
| `src/kernel/src/mm/virt/manager.rs` | 573 | `Ok(vmem.unmap(vaddr)?.is_some())` |


### `kctrl` (impl `Vmem`) [pub] — 3 external caller(s)
```
pub fn kctrl(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
        dry_run: bool,
    ) -> Result<(), Error>
```
> 
# Description

Changes access permissions on a kernel page. When `dry_run` is `true`, validates that the
operation would succeed without modifying any page table entries.

# Parameters

- `vaddr`: Virtual address of the target kernel page.
- `access`: New access permissions for the page.
- `dry_run`: If `true`, only validates the operation without applying changes.

# Returns

Upon successful completion, this function returns empty. Upon failure, this function returns
an error that indicates the reason for the failure.

# Errors

This function fails with the following error codes:
- [`ErrorCode::BadAddress`]: The provided address does not lie in kernel space.
- [`ErrorCode::TryAgain`]: Failed to read the page directory entry.
- [`ErrorCode::NoSuchEntry`]: The corresponding page table is not present.
- [`ErrorCode::NoSuchEntry`]: The page table entry was not found (dry run only).



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/pm/process/manager/mod.rs` | 2667 | `vmem.kctrl(vaddr, perm, false)?;` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2673 | `vmem.kctrl(vaddr, perm, true)?;` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2679 | `vmem.kctrl(vaddr, perm, false)?;` |


### `map` (impl `Vmem`) [pub] — 2 external caller(s)
```
pub fn map(
        &mut self,
        uframe: UserFrame,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error>
```
> Maps a page to the target virtual address space.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 386 | `child.map(child_handle, vaddr, access)?;` |
| `src/kernel/src/mm/virt/manager.rs` | 673 | `if let Err(e) = vmem.map(uframe, vaddr, access) {` |


### `for_each_user_mapping` (impl `Vmem`) [pub] — 2 external caller(s)
```
pub fn for_each_user_mapping<F>(&self, mut f: F) -> Result<(), Error>
    where
        F: FnMut(PageAligned<VirtualAddress>, PageTableEntry) -> Result<(), Error>,
```
> 
# Description

Invokes `f` once for each present user-space page in the target virtual memory
space, in the order they appear in the internal user page-table list.

# Parameters

- `f`: Callback invoked with `(vaddr, pte)` for every present user mapping. The
virtual address is page-aligned and lies in user space; `pte` is a decoded copy
of the page-table entry that backs the mapping. Returning an error from `f`
short-circuits the iteration and propagates the error to the caller.

# Returns

Upon success, `Ok(())` is returned. Upon failure, the first error returned by `f`
is propagated.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 311 | `parent.for_each_user_mapping(|vaddr, pte: PageTableEntry| {` |
| `src/kernel/src/mm/virt/manager.rs` | 464 | `let walk: Result<(), Error> = child.for_each_user_mapping(|vaddr, _pte| {` |


### `resolve_cow_for_region` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn resolve_cow_for_region(
        &mut self,
        addr: VirtualAddress,
        size: usize,
    ) -> Result<(), Error>
```
> 
# Description

Eagerly resolves all copy-on-write mappings overlapping the byte range `[addr, addr + size)`
in user space. Pages outside user space or not marked copy-on-write are left untouched.

This must be called by kernel-side write paths (e.g. `copy_to_user`) before they write
to user memory via its physical alias, so that the write does not silently mutate a
frame that is still shared with another address space.

# Parameters

- `addr`: Start of the byte range (need not be page-aligned).
- `size`: Length of the byte range, in bytes. A zero-length range is a no-op.

# Returns

Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/pm/process/manager/mod.rs` | 2171 | `.resolve_cow_for_region(dst, size)?;` |

*Internal callers (1):*
- **Vmem::copy_to_user_unaligned_unchecked** (L1262): `self.resolve_cow_for_region(dst, size)?;`

### `load` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn load(&self) -> Result<(), Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 221 | `root.load()?;` |

*Internal callers (1):*
- **Vmem::map_kpage** (L319): `self.load()?;`

### `pgdir` (impl `Vmem`) [pub] — 3 external caller(s)
```
pub fn pgdir(&self) -> &PageDirectory<PageDirectoryStorage>
```
> Returns a reference to the underlying page directory.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 254 | `new_vmem.pgdir().physical_address(),` |
| `src/kernel/src/mm/virt/manager.rs` | 255 | `vmem.pgdir().physical_address()` |
| `src/kernel/src/pm/process/manager/mod.rs` | 269 | `let cr3: u32 = vmem.pgdir().physical_address()?.into_raw_value() as u32;` |


### `is_user_addr` (impl `Vmem`) [pub] — 7 external caller(s)
```
pub fn is_user_addr(virt_addr: VirtualAddress) -> bool
```
> Asserts whether an address lies in the user space.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/pm/kcall/create_thread.rs` | 82 | `if !Vmem::is_user_addr(thread_create_args.user_fn) {` |
| `src/kernel/src/pm/kcall/create_thread.rs` | 111 | `if !Vmem::is_user_addr(user_tda) {` |
| `src/kernel/src/pm/process/manager/mod.rs` | 260 | `debug_assert!(Vmem::is_user_addr(args.user_fn));` |
| `src/kernel/src/pm/process/manager/mod.rs` | 321 | `debug_assert!(Vmem::is_user_addr(thread_create_args.user_fn));` |
| `src/kernel/src/pm/process/manager/mod.rs` | 745 | `if !Vmem::is_user_addr(args.user_fn) {` |
| `src/kernel/src/pm/process/manager/mod.rs` | 759 | `if !Vmem::is_user_addr(user_tda) {` |
| `src/kernel/src/pm/kcall/set_thread_data_area.rs` | 66 | `if !Vmem::is_user_addr(user_tda) {` |

*Internal callers (11):*
- **Vmem::map** (L379): `if !Self::is_user_addr(vaddr.into_inner()) {`
- **Vmem::is_user_page_mapped** (L451): `if !Self::is_user_addr(vaddr.into_inner()) {`
- **Vmem::is_user_region** (L485): `Some(end) => Self::is_user_addr(start) && Self::is_user_addr(end),`
- **Vmem::is_user_region** (L485): `Some(end) => Self::is_user_addr(start) && Self::is_user_addr(end),`
- **Vmem::is_kernel_addr** (L492): `!Self::is_user_addr(virt_addr)`
- **Vmem::mark_user_page_cow** (L834): `if !Self::is_user_addr(vaddr.into_inner()) {`
- **Vmem::unmark_user_page_cow** (L867): `if !Self::is_user_addr(vaddr.into_inner()) {`
- **Vmem::replace_user_page_cow_frame** (L905): `if !Self::is_user_addr(vaddr.into_inner()) {`
- **Vmem::resolve_cow_at** (L943): `if !Self::is_user_addr(vaddr.into_inner()) {`
- **Vmem::unmap** (L1549): `if !Self::is_user_addr(vaddr.into_inner()) {`

### `mark_user_page_cow` (impl `Vmem`) [pub] — 2 external caller(s)
```
pub fn mark_user_page_cow(&mut self, vaddr: PageAligned<VirtualAddress>) -> Result<(), Error>
```
> 
# Description

Marks the user page at `vaddr` as copy-on-write: clears the writable bit
and sets the AVL copy-on-write bit on the underlying page-table entry.

The page must be currently mapped and present. This is intended to be used
when sharing a user page between two address spaces (e.g. during fork).

# Parameters

- `vaddr`: Virtual address of the user page to mark.

# Returns

Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 395 | `if let Err(e) = parent.mark_user_page_cow(vaddr) {` |
| `src/kernel/src/mm/virt/manager.rs` | 405 | `if let Err(e) = child.mark_user_page_cow(vaddr) {` |


### `copy_user_to_user` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn copy_user_to_user(
        src_vmem: &Vmem,
        src: VirtualAddress,
        dst_vmem: &Vmem,
        dst: VirtualAddress,
        size: usize,
    ) -> Result<(), Error>
```
> 
# Description

Copies data directly between the user spaces of two processes. The source address is
resolved using `src_vmem` and the destination address is resolved using `dst_vmem`. Both
addresses must lie in user space. The copy is performed page-by-page using physical frame
addresses, bypassing kernel space entirely.

# Parameters

- `src_vmem`: Source process's virtual memory space.
- `src`: Source address in `src_vmem`'s user space.
- `dst_vmem`: Destination process's virtual memory space.
- `dst`: Destination address in `dst_vmem`'s user space.
- `size`: Number of bytes to copy.

# Returns

Upon successful completion, empty is returned. On failure, an error is returned instead.

# Errors

- [`ErrorCode::InvalidArgument`]: The size of the copy is zero.
- [`ErrorCode::BadAddress`]: The source memory region does not lie in user space.
- [`ErrorCode::BadAddress`]: The destination memory region does not lie in user space.
- [`ErrorCode::NoSuchEntry`]: A page in the source or destination region is not mapped.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/pm/process/manager/mod.rs` | 2178 | `Vmem::copy_user_to_user(src_vmem, src, dst_vmem, dst, size)` |


### `new` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn new(
        mut kernel_pages: LinkedList<KernelPage>,
        mut kernel_page_tables: LinkedList<(PageTableAddress, PageTable<PageTableStorage>)>,
    ) -> Result<Self, Error>
```
> Initializes a new virtual memory space.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 218 | `let root: Vmem = Vmem::new(kernel_pages, kernel_page_tables)?;` |


### `copy_to_user_unaligned_unchecked` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn copy_to_user_unaligned_unchecked(
        &mut self,
        mut dst: VirtualAddress,
        mut src: VirtualAddress,
        mut size: usize,
        dry_run: bool,
    ) -> Result<(), Error>
```
> have to be aligned, but the destination address range must lie in user space, and the source
address range must lie in kernel space.

# Parameters

- `dst`: Destination address in user space.
- `src`: Source address in kernel space.
- `size`: Number of bytes to copy.
- `dry_run`: If `true`, the function does not actually copy any data.

# Return Value

Upon successful completion, this function returns empty. Upon failure, this function returns
an error that indicates the reason for the failure.

# Errors

This function fails with the following error codes:
- [`ErrorCode::InvalidArgument`]: The size of the copy is zero.
- [`ErrorCode::BadAddress`]: The source memory region does not lie in kernel space.
- [`ErrorCode::BadAddress`]: The destination memory region does not lie in user space.
- [`ErrorCode::BadAddress`]: The source memory region does not lie within physical memory.
- [`ErrorCode::BadAddress`]: The destination memory region does not lie within physical memory.

# Safety Notes

When not running in dry-run mode, this function performs a physical memory copy. Any
errors that occur while copying data will cause this function to panic.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/elf.rs` | 362 | `vmem.copy_to_user_unaligned_unchecked(` |

*Internal callers (3):*
- **Vmem::copy_to_user_unaligned** (L1380): `self.copy_to_user_unaligned_unchecked(dst, src, size, false)`
- **Vmem::copy_to_user_unaligned** (L1383): `self.copy_to_user_unaligned_unchecked(dst, src, size, true)?;`
- **Vmem::copy_to_user_unaligned** (L1384): `self.copy_to_user_unaligned_unchecked(dst, src, size, false)`

### `uctrl` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn uctrl(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error>
```
> Changes access permissions on a page.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 739 | `vmem.uctrl(vaddr, access)` |


### `user_vaddr_to_paddr` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn user_vaddr_to_paddr(&self, vaddr: VirtualAddress) -> Result<usize, Error>
```
> 
# Description

Translates a user-space virtual address to a guest physical address by walking the page
tables. The returned physical address includes the intra-page offset from the original
virtual address.

# Parameters

- `vaddr`: User-space virtual address to translate.

# Returns

Upon success, the guest physical address corresponding to `vaddr` is returned. Upon
failure, an error is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/pm/process/manager/mod.rs` | 2202 | `proc_ref.state().vmem().user_vaddr_to_paddr(vaddr)` |


### `unmark_user_page_cow` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn unmark_user_page_cow(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error>
```
> 
# Description

Inverse of [`Self::mark_user_page_cow`]: clears the copy-on-write mark on the user
page at `vaddr`, restoring its writable bit and clearing the AVL copy-on-write bit.

# Parameters

- `vaddr`: Virtual address of the user page to be unmarked.

# Returns

Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 409 | `if let Err(re) = parent.unmark_user_page_cow(vaddr) {` |

*Internal callers (1):*
- **Vmem::resolve_cow_at** (L966): `self.unmark_user_page_cow(vaddr)?;`

### `memset` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn memset(&mut self, dst: PageAligned<VirtualAddress>, value: u32) -> Result<(), Error>
```
> 
# Description

Fills a page with a given value in the target virtual address space.

# Parameters

- `dst`: Virtual address of the target page.

# Returns

Upon success, empty is returned. Upon failure, an error code is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 679 | `if let Err(e) = vmem.memset(vaddr, 0) {` |


### `map_kpage` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn map_kpage(
        &mut self,
        kpage: KernelPage,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error>
```
> 
# Description

Maps a kernel page to the target virtual address space.

# Parameters
- `kpage`: Kernel page to be mapped.
- `vaddr`: Virtual address of the target page.

# Returns

Upon success, empty is returned. Upon failure, an error code is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/kernel_vas.rs` | 151 | `vmem.map_kpage(kpage, vaddr)?;` |


### `clone` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn clone(from: &Vmem, pgdir_page: KernelPage) -> Result<Vmem, Error>
```
> Clones the target virtual memory space.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 250 | `let new_vmem: Vmem = Vmem::clone(vmem, pgdir_page)?;` |


### `is_physical_region` (impl `Vmem`) [pub] — **0 external callers**
```
pub fn is_physical_region(start: usize, size: usize) -> bool
```
> 
# Description

Asserts whether a memory region lies within physical memory.

# Parameters

- `start`: Starting physical address of the region.
- `size`: Size of the region in bytes.

# Returns

Returns `true` if the entire region lies within physical memory, `false` otherwise.


*Internal callers (2):*
- **Vmem::copy_to_user_unaligned_unchecked** (L1287): `if !Self::is_physical_region(src_phys_addr_raw, copy_size) {`
- **Vmem::copy_to_user_unaligned_unchecked** (L1318): `if !Self::is_physical_region(dst_phys_addr_raw, copy_size) {`

### `try_find_user_pte` (impl `Vmem`) [pub(crate)] — 1 external caller(s)
```
pub(crate) fn try_find_user_pte(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<PageTableEntry>, Error>
```
> 
# Description

Attempts to find the page-table entry that backs the user page at `vaddr`.

# Parameters

- `vaddr`: Virtual address of the target page.

# Returns

- `Ok(Some(pte))` if the page is present, where `pte` is a decoded copy of the
page-table entry that backs the mapping.
- `Ok(None)` if the page table or page is not present.
- `Err(_)` on unexpected failures.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 312 | `if count < LINK_CHUNK && child.try_find_user_pte(vaddr)?.is_none() {` |

*Internal callers (1):*
- **Vmem::resolve_cow_at** (L947): `let pte: PageTableEntry = match self.try_find_user_pte(vaddr)? {`

### `resolve_cow_at` (impl `Vmem`) [pub] — 1 external caller(s)
```
pub fn resolve_cow_at(&mut self, vaddr: PageAligned<VirtualAddress>) -> Result<bool, Error>
```
> 
# Description

Resolves a copy-on-write mapping at `vaddr`, if any. Allocates a private user frame,
copies the shared frame's contents into it, repoints the PTE at the new frame, and
drops the reference on the previously-shared frame.

This is the building block used by both the page-fault handler (lazy resolution on a
user-mode write) and the kernel-side write paths (eager resolution before the kernel
writes to a user page via its physical alias, which would otherwise silently mutate
the shared frame and bypass the copy-on-write contract).

# Parameters

- `vaddr`: Page-aligned user virtual address to resolve.

# Returns

- `Ok(true)` if a copy-on-write mapping was found at `vaddr` and resolved.
- `Ok(false)` if `vaddr` is not mapped or the PTE is not marked copy-on-write.
- `Err(_)` if the resolution failed (e.g. out of frames).



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 548 | `vmem.resolve_cow_at(vaddr)` |

*Internal callers (1):*
- **Vmem::resolve_cow_for_region** (L1039): `self.resolve_cow_at(vaddr)?;`

### `drop` (trait `Drop` for `Vmem`) [trait-pub] — implicit via `Drop`
```
fn drop(&mut self)
```
> ⚡ **impl Drop for Vmem**: Compiler inserts call to drop() when value goes out of scope


## Private Functions — Internal Call Graph

These are implementation details. Listed to show which public functions depend on them.

### `lookup_kernel_page_table` (impl `Vmem`) [private]
```
fn lookup_kernel_page_table(
        &mut self,
        pde: &PageDirectoryEntry,
    ) -> Result<Rc<RefCell<(PageTableAddress, PageTable<PageTableStorage>)>>, Error>
```
*Called by (1):*
- **Vmem::kctrl** (L1733): `self.lookup_kernel_page_table(&pde)?`

### `allocate_kernel_page_table` (impl `Vmem`) [private]
```
fn allocate_kernel_page_table() -> Result<PageTable<PageTableStorage>, Error>
```
> 
# Description

Allocate a page table for mapping kernel memory.

# Returns

Upon success, `Ok(page_table)` is returned. Upon failure, an error is returned.


*Called by (1):*
- **Vmem::map_kpage** (L277): `let page_table: PageTable<PageTableStorage> = Self::allocate_kernel_page_table()`

### `replace_user_page_cow_frame` (impl `Vmem`) [private]
```
fn replace_user_page_cow_frame(
        &mut self,
        vaddr: PageAligned<VirtualAddress>,
        new_frame: FrameAddress,
    ) -> Result<FrameAddress, Error>
```
> 
# Description

Resolves a copy-on-write fault on the user page at `vaddr` by repointing
its page-table entry at `new_frame`, clearing the AVL copy-on-write bit,
and restoring the writable bit.

# Parameters

- `vaddr`: Virtual address of the user page being resolved.
- `new_frame`: Physical frame to install in the PTE.

# Returns

Upon success, the previous frame address (the shared frame the PTE pointed
at) is returned. The caller is responsible for releasing that reference.
Upon failure, an error is returned instead.


*Called by (1):*
- **Vmem::resolve_cow_at** (L983): `let old_frame: FrameAddress = self.replace_user_page_cow_frame(vaddr, new_frame_`

### `find_user_frame` (impl `Vmem`) [private]
```
fn find_user_frame(&self, vaddr: PageAligned<VirtualAddress>) -> Result<FrameAddress, Error>
```
> 
# Description

Finds a user frame in the target virtual memory space.

# Parameters

- `vaddr`: Virtual address of the target page.

# Returns

Upon success, a reference to the target user page is returned. Upon failure, an error code is
returned instead.


*Called by (6):*
- **Vmem::user_vaddr_to_paddr** (L1075): `let frame: FrameAddress = self.find_user_frame(page_aligned)?;`
- **Vmem::copy_from_user_unaligned** (L1152): `let src_frame: FrameAddress = self.find_user_frame(vaddr)?;`
- **Vmem::copy_to_user_unaligned_unchecked** (L1305): `let dst_frame: FrameAddress = match self.find_user_frame(vaddr) {`
- **Vmem::copy_user_to_user** (L1464): `let src_frame: FrameAddress = src_vmem.find_user_frame(src_page)?;`
- **Vmem::copy_user_to_user** (L1465): `let dst_frame: FrameAddress = dst_vmem.find_user_frame(dst_page)?;`
- **Vmem::memset** (L1516): `let uframe: FrameAddress = self.find_user_frame(dst)?;`

### `lookup_user_page_table` (impl `Vmem`) [private]
```
fn lookup_user_page_table(
        &mut self,
        pt_vaddr: PageTableAddress,
    ) -> Result<&mut PageTable<PageTableStorage>, Error>
```
> 
# Description

Looks up a user page table by its virtual base address. The first lookup in a given region
is O(n) in the number of user page tables, but moves the found entry to the front of the
list so that subsequent lookups for the same 4 MB region complete in O(1). This exploits
spatial locality: consecutive pages within the same region share the same page table.

# Preconditions

The caller must ensure that the page table identified by `pt_vaddr` has already been mapped
in the page directory (i.e., the corresponding PDE is present).

# Parameters

- `pt_vaddr`: Virtual base address of the page table to look up.

# Returns

Upon success, a mutable reference to the page table is returned. Upon failure, an error
code is returned instead.


*Called by (6):*
- **Vmem::map** (L419): `self.lookup_user_page_table(pgtable_vaddr)?`
- **Vmem::mark_user_page_cow** (L845): `self.lookup_user_page_table(pgtable_vaddr)?;`
- **Vmem::unmark_user_page_cow** (L878): `self.lookup_user_page_table(pgtable_vaddr)?;`
- **Vmem::replace_user_page_cow_frame** (L916): `self.lookup_user_page_table(pgtable_vaddr)?;`
- **Vmem::unmap** (L1586): `(pgtable_vaddr, self.lookup_user_page_table(pgtable_vaddr)?)`
- **Vmem::uctrl** (L1660): `self.lookup_user_page_table(pgtable_vaddr)?`

### `is_kernel_addr` (impl `Vmem`) [private]
```
fn is_kernel_addr(virt_addr: VirtualAddress) -> bool
```
> Asserts whether an address lies in the kernel space.

*Called by (2):*
- **Vmem::is_kernel_region** (L517): `Some(end) => Self::is_kernel_addr(start) && Self::is_kernel_addr(end),`
- **Vmem::kctrl** (L1705): `if !Self::is_kernel_addr(vaddr.into_inner()) {`

### `try_find_user_frame` (impl `Vmem`) [private]
```
fn try_find_user_frame(
        &self,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<Option<FrameAddress>, Error>
```
> 
# Description

Attempts to find a user frame in the target virtual memory space.

# Parameters

- `vaddr`: Virtual address of the target page.

# Returns

- `Ok(Some(addr))` if the page is present.
- `Ok(None)` if the page table or page is not present.
- `Err(_)` on unexpected failures.


*Called by (2):*
- **Vmem::is_user_page_mapped** (L455): `Ok(self.try_find_user_frame(vaddr)?.is_some())`
- **Vmem::unmap** (L1556): `let frame_address: FrameAddress = match self.try_find_user_frame(vaddr)? {`

### `allocate_user_page_table` (impl `Vmem`) [private]
```
fn allocate_user_page_table() -> Result<PageTable<PageTableStorage>, Error>
```
> 
# Description

Allocate a page table for mapping user memory.

# Returns

Upon success, `Ok(page_table)` is returned. Upon failure, an error is returned.


*Called by (1):*
- **Vmem::map** (L404): `let page_table: PageTable<PageTableStorage> = Self::allocate_user_page_table()?;`

### `is_kernel_region` (impl `Vmem`) [private]
```
fn is_kernel_region(start: VirtualAddress, size: usize) -> bool
```
> 
# Description

Asserts whether a memory region lies entirely in kernel space.

# Parameters

- `start`: Starting virtual address of the region.
- `size`: Size of the region in bytes.

# Returns

Returns `true` if the entire region lies in kernel space, `false` otherwise.


*Called by (2):*
- **Vmem::copy_from_user_unaligned** (L1133): `if !Self::is_kernel_region(dst, size) {`
- **Vmem::copy_to_user_unaligned_unchecked** (L1236): `if !Self::is_kernel_region(src, size) {`

## Type References

### `Vmem` [pub] — 76 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/pm/kcall/create_thread.rs` | 12 | `Vmem,` |
| `src/kernel/src/pm/kcall/create_thread.rs` | 62 | `if !Vmem::is_user_region(unsafe_thread_create_args, size_of::<ThreadCreateArgs>(` |
| `src/kernel/src/pm/kcall/create_thread.rs` | 82 | `if !Vmem::is_user_addr(thread_create_args.user_fn) {` |
| `src/kernel/src/pm/kcall/create_thread.rs` | 89 | `if !Vmem::is_user_region(thread_create_args.user_stack_base, thread_create_args.` |
| `src/kernel/src/pm/kcall/create_thread.rs` | 111 | `if !Vmem::is_user_addr(user_tda) {` |
| `src/kernel/src/pm/mod.rs` | 27 | `mm::Vmem,` |
| `src/kernel/src/pm/mod.rs` | 123 | `pub fn init(root: Vmem) -> Result<(), Error> {` |
| `src/kernel/src/pm/kcall/duplicate.rs` | 12 | `Vmem,` |
| `src/kernel/src/pm/kcall/duplicate.rs` | 66 | `if !Vmem::is_user_region(unsafe_args, size_of::<ThreadCreateArgs>()) {` |
| `src/kernel/src/mm/virt/mod.rs` | 43 | `pub use vmem::Vmem;` |
| `src/kernel/src/pm/process/manager/unsafe.rs` | 20 | `Vmem,` |
| `src/kernel/src/pm/process/manager/unsafe.rs` | 129 | `pub fn init(interrupt_capable: bool, kernel: ReadyThread, root: Vmem, tm: Thread` |
| `src/kernel/src/pm/process/manager/unsafe.rs` | 393 | `let vmem: &mut Vmem = process_ref.state_mut().vmem_mut();` |
| `src/kernel/src/pm/process/manager/mod.rs` | 34 | `Vmem,` |
| `src/kernel/src/pm/process/manager/mod.rs` | 183 | `root: Vmem,` |
| `src/kernel/src/pm/process/manager/mod.rs` | 248 | `vmem: &mut Vmem,` |
| `src/kernel/src/pm/process/manager/mod.rs` | 259 | `debug_assert!(Vmem::is_user_region(args.user_stack_base, args.user_stack_size));` |
| `src/kernel/src/pm/process/manager/mod.rs` | 260 | `debug_assert!(Vmem::is_user_addr(args.user_fn));` |
| `src/kernel/src/pm/process/manager/mod.rs` | 321 | `debug_assert!(Vmem::is_user_addr(thread_create_args.user_fn));` |
| `src/kernel/src/pm/process/manager/mod.rs` | 475 | `vmem: &mut Vmem,` |

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `is_physical_region`

