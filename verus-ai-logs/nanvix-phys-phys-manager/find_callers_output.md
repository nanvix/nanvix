# Caller Analysis (LSP): manager.rs

- **Source file:** `/home/ruize/nanvix-phy/src/kernel/src/mm/phys/manager.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 7 |
| Public / trait-pub | 6 |
| Private | 1 |
| Types | 1 |

## Public API — External Callers

### `get_mut` (impl `PhysMemoryManager`) [pub] — 7 external caller(s)
```
pub unsafe fn get_mut<'a>() -> &'a mut PhysMemoryManager
```
> 
# Description

Gets a mutable reference to the physical memory manager.

# Panics

Panics if the physical memory manager is not initialized.

# Safety

This function is unsafe because it returns a mutable reference to a global variable.

The caller must ensure:

- No other `&mut PhysMemoryManager` reference obtained from this function is live at the
same time (i.e., `&mut` references must not overlap). In practice this is guaranteed
because the kernel is single-threaded and runs with interrupts disabled, so no
re-entrant or concurrent call can alias the reference.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 217 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 267 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 600 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 611 | `unsafe { PhysMemoryManager::get_mut() }.alloc_many_user_frames(nframes, uframes)` |
| `src/kernel/src/mm/virt/manager.rs` | 705 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 748 | `unsafe { PhysMemoryManager::get_mut() }.alloc_many_kernel_frames(count, kframes)` |
| `src/kernel/src/mm/virt/vmem.rs` | 879 | `let new_frame: UserFrame = unsafe { PhysMemoryManager::get_mut() }.alloc_user_fr` |


### `alloc_user_frame` (impl `PhysMemoryManager`) [pub] — 1 external caller(s)
```
pub fn alloc_user_frame(&mut self) -> Result<UserFrame, Error>
```
> 
# Description

Allocates a single user frame, applying the same kernel watermark check as
[`Self::alloc_many_user_frames`]. This is the single-frame fast path used on
hot paths such as copy-on-write fault resolution, where allocating an
intermediate [`Vec`] would be wasteful.

# Returns

Upon success, a [`UserFrame`] is returned. Upon failure, an error is returned
instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/vmem.rs` | 879 | `let new_frame: UserFrame = unsafe { PhysMemoryManager::get_mut() }.alloc_user_fr` |


### `alloc_many_user_frames` (impl `PhysMemoryManager`) [pub] — 1 external caller(s)
```
pub fn alloc_many_user_frames(
        &mut self,
        count: usize,
        frames: &mut Vec<UserFrame>,
    ) -> Result<(), Error>
```
> 
# Description

Allocates user frames into caller-provided storage.

The returned frames are not guaranteed to be physically contiguous.
User allocations are gated by the kernel watermark: if fulfilling the request would
leave fewer than `KERNEL_WATERMARK` free frames, the allocation is rejected.

# Parameters

- `count`: Number of frames to allocate.
- `frames`: Mutable reference to a pre-allocated vector into which to store those
frames' addresses. It should have sufficient capacity for `count` entries.

# Return Values

Upon success, `Ok(())` is returned and `frames` is filled with `count` frames. Upon failure, an
error is returned and any frames allocated by this call are dropped by truncating `frames`
back to empty.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 611 | `unsafe { PhysMemoryManager::get_mut() }.alloc_many_user_frames(nframes, uframes)` |


### `alloc_kernel_frame` (impl `PhysMemoryManager`) [pub] — 4 external caller(s)
```
pub fn alloc_kernel_frame(&mut self) -> Result<KernelFrame, Error>
```
> 
# Description

Allocates a kernel frame.

Kernel allocations bypass the watermark — no artificial ceiling.

# Return Values

Upon success, a kernel frame is returned. Upon failure, an error is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 217 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 267 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 600 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 705 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |


### `alloc_many_kernel_frames` (impl `PhysMemoryManager`) [pub] — 1 external caller(s)
```
pub fn alloc_many_kernel_frames(
        &mut self,
        count: usize,
        frames: &mut Vec<KernelFrame>,
    ) -> Result<(), Error>
```
> 
# Description

Allocates a contiguous range of kernel frames into caller-provided storage.

Kernel stacks require physically contiguous frames because the kernel uses identity
mapping and the hardware stack pointer traverses the region linearly.
Kernel allocations bypass the watermark — no artificial ceiling.

# Parameters

- `count`: Number of frames to allocate.
- `frames`: Mutable reference to a pre-allocated vector into which to store
those frames' addresses.

# Return Values

Upon success, `Ok(())` is returned and `frames` is filled with `count`
contiguous entries. Upon failure, an error is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 748 | `unsafe { PhysMemoryManager::get_mut() }.alloc_many_kernel_frames(count, kframes)` |


### `init` (impl `PhysMemoryManager`) [pub(super)] — 1 external caller(s)
```
pub(super) fn init(upool: Upool) -> Result<(), Error>
```
> 
# Description

Initializes the physical memory manager singleton.

# Parameters

- `upool`: User page pool.

# Errors

Returns `InvalidArgument` if the singleton has already been initialized.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 138 | `if frame::is_covered(phys_addr) {` |


## Private Functions — Internal Call Graph

These are implementation details. Listed to show which public functions depend on them.

### `check_user_watermark` (impl `PhysMemoryManager`) [private]
```
fn check_user_watermark(count: usize) -> Result<(), Error>
```
> 
# Description

Rejects user allocations of `count` frames that would breach the kernel
watermark, i.e. that would drop the number of free frames below
[`config::kernel::KERNEL_WATERMARK`].

# Parameters

- `count`: Number of user frames the caller intends to allocate.

# Returns

Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.


*Called by (2):*
- **PhysMemoryManager::alloc_many_user_frames** (L177): `Self::check_user_watermark(count)?;`
- **PhysMemoryManager::alloc_user_frame** (L205): `Self::check_user_watermark(1)?;`

## Type References

### `PhysMemoryManager` [pub] — 11 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 27 | `PhysMemoryManager,` |
| `src/kernel/src/mm/virt/manager.rs` | 217 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 267 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 600 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 611 | `unsafe { PhysMemoryManager::get_mut() }.alloc_many_user_frames(nframes, uframes)` |
| `src/kernel/src/mm/virt/manager.rs` | 705 | `unsafe { PhysMemoryManager::get_mut() }.alloc_kernel_frame()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 748 | `unsafe { PhysMemoryManager::get_mut() }.alloc_many_kernel_frames(count, kframes)` |
| `src/kernel/src/mm/virt/vmem.rs` | 32 | `PhysMemoryManager,` |
| `src/kernel/src/mm/virt/vmem.rs` | 879 | `let new_frame: UserFrame = unsafe { PhysMemoryManager::get_mut() }.alloc_user_fr` |
| `src/kernel/src/mm/phys/mod.rs` | 51 | `manager::PhysMemoryManager,` |
| `src/kernel/src/mm/phys/mod.rs` | 138 | `if frame::is_covered(phys_addr) {` |

