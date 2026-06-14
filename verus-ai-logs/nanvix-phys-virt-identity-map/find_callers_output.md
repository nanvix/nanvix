# Caller Analysis (LSP): identity_map.rs

- **Source file:** `/home/ruize/nanvix-phy/src/kernel/src/mm/virt/identity_map.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 14 |
| Public / trait-pub | 7 |
| Private | 7 |
| Types | 1 |

## Implicit Callers (runtime/compiler)

### impl Drop for Cr3Guard
- **Trait:** `Drop`
- **Description:** Compiler inserts call to drop() when value goes out of scope
- **Methods dispatched:** `drop`

> These functions have **no explicit call sites** in source code. The Rust runtime dispatches to them via vtable / lang items.

## Public API — External Callers

### `drop` (trait `Drop` for `Cr3Guard`) [trait-pub] — implicit via `Drop`
```
fn drop(&mut self)
```
> 
# Description

Restores the previously active CR3 value saved in this guard.


> ⚡ **impl Drop for Cr3Guard**: Compiler inserts call to drop() when value goes out of scope


### `init` [pub(crate)] — 1 external caller(s)
```
pub(crate) fn init(
    kernel_pd_paddr: PageDirectoryAddress,
    kernel_cr3: Cr3Register,
) -> Result<(), Error>
```
> 
# Description

Records the kernel page-directory and root paging-structure physical addresses used by the
lazy identity mapper.

# Parameters

- `kernel_pd_paddr`: Physical address of the kernel page directory.
- `kernel_cr3`: CR3 register value for the kernel root paging structure used for CR3 switching.

# Returns

Upon success, `Ok(())`. Upon failure, an error is returned and the global state remains
uninitialized (atomics are not published).

# Notes

On x86, `kernel_cr3` equals `kernel_pd_paddr` (the page directory is the CR3 root).

This function pre-allocates a BSS page table for every PDE index in
`[0, MEMORY_SIZE)` that does not already have one. This covers all physical memory, so no
new PDEs are created at runtime. The kernel PD and CR3 atomics are published only after
pre-allocation succeeds, so other code never observes a partially-initialized identity map.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/mod.rs` | 15 | `use identity_map::init as identity_map_init;` |


### `memset` [pub(crate)] — 3 external caller(s)
```
pub(crate) fn memset(base: *mut u8, value: u8, size: usize) -> Result<(), Error>
```
> 
# Description

Fills bytes in a memory range with a byte value, after ensuring that the full target range is
identity-mapped in the kernel address space.

# Parameters

- `base`: Starting physical address of the target range.
- `value`: Byte value to fill.
- `size`: Number of bytes to fill.

# Returns

Upon success, empty is returned. Upon failure, an error is returned instead.

# Errors

- [`ErrorCode::BadAddress`]: The target range is invalid or overflows.
- Any error propagated by the lazy identity mapper while preparing the range.

# Notes

If `size == 0`, this function is a no-op and returns success.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/kframe.rs` | 135 | `pub fn base(&self) -> FrameAddress {` |
| `src/kernel/src/mm/virt/vmem.rs` | 1421 | `super::memset(base, value as u8, mem::PAGE_SIZE)?;` |
| `src/kernel/src/mm/virt/mod.rs` | 16 | `pub(in crate::mm) use identity_map::memset;` |


### `sync_kernel_pdes` [pub(crate)] — 2 external caller(s)
```
pub(crate) fn sync_kernel_pdes(target_pd_paddr: PageDirectoryAddress) -> Result<(), Error>
```
> 
# Description

Copies all present kernel identity-mapping PDEs from the kernel page directory into a target
page directory. This covers `[0, MEMORY_SIZE)` and ensures that the target PD (typically a
new user process PD) can access all kernel identity-mapped memory. Because all PDEs in this
range are pre-allocated at boot ([`init`]), this is a simple copy of already-present entries.

# Parameters

- `target_pd_paddr`: Physical address of the target page directory.

# Returns

Upon success, `Ok(())`. Upon failure, an error is returned.

# Notes

This function should be called once when constructing a new user address space.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/vmem.rs` | 202 | `super::sync_kernel_pdes(target_pd_paddr)?;` |
| `src/kernel/src/mm/virt/mod.rs` | 20 | `sync_kernel_pdes,` |

*Internal callers (2):*
- ? (L677): `sync_kernel_pdes,`
- **test_sync_kernel_pdes_copies_to_target** (L770): `if let Err(e) = sync_kernel_pdes(target_pd_paddr) {`

### `memcpy` [pub(crate)] — 5 external caller(s)
```
pub(crate) fn memcpy(dst: *mut u8, src: *const u8, size: usize) -> Result<(), Error>
```
> 
# Description

Copies bytes between two memory regions, after ensuring that both ranges are identity-mapped in
the kernel address space.

# Parameters

- `dst`: Destination physical address.
- `src`: Source physical address.
- `size`: Number of bytes to copy.

# Returns

Upon success, empty is returned. Upon failure, an error is returned instead.

# Errors

- [`ErrorCode::BadAddress`]: One of the physical ranges is invalid or overflows.
- Any error propagated by the lazy identity mapper while preparing the ranges.

# Notes

If `size == 0`, this function is a no-op and returns success.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/vmem.rs` | 883 | `super::memcpy(dst_paddr as *mut u8, src_paddr as *const u8, mem::PAGE_SIZE)?;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1062 | `super::memcpy(` |
| `src/kernel/src/mm/virt/vmem.rs` | 1236 | `let copy_result: Result<(), Error> = super::memcpy(dst, src, copy_size);` |
| `src/kernel/src/mm/virt/vmem.rs` | 1377 | `super::memcpy(dst_phys_addr as *mut u8, src_phys_addr as *const u8, copy_size)?;` |
| `src/kernel/src/mm/virt/mod.rs` | 19 | `memcpy,` |


### `identity_map_page` [pub(crate)] — 2 external caller(s)
```
pub(crate) fn identity_map_page(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>
```
> 
# Description

Identity-maps a single page in the kernel page directory.

If the target PTE is already present, this function is a no-op. If the PDE is absent, a new
page table is allocated from the BSS pool and installed before the identity-mapped PTE is
created.

# Parameters

- `phys_addr`: Page-aligned physical address to identity-map.

# Returns

Upon success, empty is returned. Upon failure, an error is returned instead.

# Errors

- [`ErrorCode::InvalidArgument`]: Failed to read a valid paging entry.

# Notes

If the lazy mapper has not been initialized yet (boot page tables still active), this function
is a no-op and returns success.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/kframe.rs` | 100 | `PageAligned::from_raw_value(base.into_raw_value()).map_err(|e| {` |
| `src/kernel/src/mm/virt/mod.rs` | 18 | `identity_map_page,` |

*Internal callers (1):*
- **ensure_identity_mapped_range** (L478): `identity_map_page(page_addr)?;`

### `test` [pub] — **0 external callers**
```
pub fn test() -> bool
```
> Runs all identity-mapping virtual memory tests.


## Private Functions — Internal Call Graph

These are implementation details. Listed to show which public functions depend on them.

### `page_aligned_cover` [private]
```
fn page_aligned_cover(
    addr: PhysicalAddress,
    size: usize,
) -> Result<(PageAligned<PhysicalAddress>, usize), Error>
```
> 
# Description

Computes the smallest page-aligned range that fully covers `[addr, addr + size)`.

# Parameters

- `addr`: Starting physical address of the target range.
- `size`: Number of bytes in the target range (must be > 0).

# Returns

Upon success, a tuple of the page-aligned start address and the page-aligned size is returned.
Upon failure, an error is returned instead.

# Errors

- [`ErrorCode::BadAddress`]: The range overflows or contains an invalid physical address.


*Called by (3):*
- **memcpy** (L201): `let (src_start, src_size) = page_aligned_cover(src_addr, size)?;`
- **memcpy** (L204): `let (dst_start, dst_size) = page_aligned_cover(dst_addr, size)?;`
- **memset** (L256): `let (base_start, base_size) = page_aligned_cover(base_addr, size)?;`

### `ensure_identity_mapped_range` [private]
```
fn ensure_identity_mapped_range(
    start: PageAligned<PhysicalAddress>,
    size: usize,
) -> Result<(), Error>
```
> 
# Description

Ensures that every page in `[start, start + size)` is identity-mapped in the kernel
page directory.

# Parameters

- `start`: Page-aligned starting physical address of the target range.
- `size`: Number of bytes in the target range (must be a multiple of [`PAGE_SIZE`]).

# Returns

Upon success, empty is returned. Upon failure, an error is returned instead.

# Errors

- [`ErrorCode::BadAddress`]: The range contains an invalid physical address.

# Notes

If `size == 0`, this function is a no-op and returns success.

*Called by (3):*
- **memcpy** (L202): `ensure_identity_mapped_range(src_start, src_size)?;`
- **memcpy** (L205): `ensure_identity_mapped_range(dst_start, dst_size)?;`
- **memset** (L257): `ensure_identity_mapped_range(base_start, base_size)?;`

### `ensure_pt` [private]
```
fn ensure_pt(pd: Table<PageDirectoryEntry>, pde_idx: TableIndex) -> Result<usize, Error>
```
> 
# Description

Ensures that a page table exists for the given PDE index in the kernel page directory. If the
PDE is already present, returns the physical address of the existing page table. Otherwise,
allocates a new page table from the BSS pool, installs the PDE, and returns the new page
table's physical address.

# Parameters

- `pd`: The kernel page directory table.
- `pde_idx`: The page directory index to check.

# Returns

Upon success, the physical address of the page table is returned. Upon failure, an error is
returned instead.

# Errors

- [`ErrorCode::InvalidArgument`]: Failed to read the PDE.
- [`ErrorCode::OutOfMemory`]: No BSS page table slots available.
- [`ErrorCode::BadAddress`]: The allocated page table frame number is out of range.


*Called by (2):*
- **init** (L133): `ensure_pt(pd, pde_idx)?;`
- **identity_map_page** (L663): `let pt_paddr: usize = ensure_pt(pd, pde_idx)?;`

### `ensure_pte` [private]
```
fn ensure_pte(
    pt: Table<PageTableEntry>,
    pte_idx: TableIndex,
    phys_addr: usize,
) -> Result<(), Error>
```
> 
# Description

Ensures that a page table entry for the given index is identity-mapped. If the PTE is already
present, this function is a no-op. Otherwise, it creates a new identity-mapped PTE and
invalidates the corresponding TLB entry.

# Parameters

- `pt`: The page table to write into.
- `pte_idx`: The page table index to check.
- `phys_addr`: The physical address to identity-map (used as both frame source and TLB target).

# Returns

Upon success, empty is returned. Upon failure, an error is returned instead.

# Errors

- [`ErrorCode::InvalidArgument`]: Failed to read the PTE.
- [`ErrorCode::BadAddress`]: The frame number is out of range.


*Called by (1):*
- **identity_map_page** (L667): `ensure_pte(pt, pte_idx, phys_addr)`

### `test_init_preallocates_identity_map_pdes` [private]
```
fn test_init_preallocates_identity_map_pdes() -> bool
```
> 
# Description

Verifies that [`super::init`] pre-allocates a page table for every PDE index in
`[0, MEMORY_SIZE)`. Reads each PDE from the kernel page directory and
asserts the present bit is set.


*No internal callers found (may be called via macro, closure, or conditional compilation).*

### `with_kernel_address_space` [private]
```
fn with_kernel_address_space<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
```
> 
# Description

Temporarily switches CR3 to the kernel address space, executes `f`, and restores the previous
CR3 value on return.

This gives `f` access to kernel identity mappings while preserving the caller's original
address-space context through RAII restoration.

# Parameters

- `f`: Closure to execute while CR3 points to the kernel address space.

# Returns

Returns the value produced by `f`.

# Interrupt Safety

The caller must ensure that interrupts are disabled or that interrupt handlers are
CR3-agnostic. If an interrupt fires while CR3 points to the kernel address space,
the handler will execute in the kernel address space rather than the original one.


*Called by (2):*
- **memcpy** (L199): `with_kernel_address_space(|| {`
- **memset** (L254): `with_kernel_address_space(|| {`

### `test_sync_kernel_pdes_copies_to_target` [private]
```
fn test_sync_kernel_pdes_copies_to_target() -> bool
```
> 
# Description

Allocates a zeroed kernel page, treats it as a page directory, calls
[`sync_kernel_pdes`], and verifies that every present kernel PDE in
`[0, MEMORY_SIZE)` was copied into the target PD with the same frame address.


*No internal callers found (may be called via macro, closure, or conditional compilation).*

## Type References

### `Cr3Guard` [private] — 0 external reference(s)

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `test`

