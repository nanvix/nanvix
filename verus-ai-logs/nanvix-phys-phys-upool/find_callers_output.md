# Caller Analysis (LSP): upool.rs

- **Source file:** `/home/ruize/nanvix-phy/src/kernel/src/mm/phys/upool.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 8 |
| Public / trait-pub | 8 |
| Private | 0 |
| Types | 2 |

## Implicit Callers (runtime/compiler)

### impl Drop for UserFrame
- **Trait:** `Drop`
- **Description:** Compiler inserts call to drop() when value goes out of scope
- **Methods dispatched:** `drop`

> These functions have **no explicit call sites** in source code. The Rust runtime dispatches to them via vtable / lang items.

## Public API — External Callers

### `new` (impl `UserFrame`) [pub] — 4 external caller(s)
```
pub fn new(addr: FrameAddress) -> Self
```
> 
# Description

Instantiates a user frame.

# Parameters

- `addr`: Frame address.

# Returns

A user frame.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 349 | `let parent_handle: ManuallyDrop<UserFrame> = ManuallyDrop::new(UserFrame::new(fr` |
| `src/kernel/src/mm/virt/vmem.rs` | 869 | `let probe: ManuallyDrop<UserFrame> = ManuallyDrop::new(UserFrame::new(src_frame)` |
| `src/kernel/src/mm/virt/vmem.rs` | 895 | `drop(UserFrame::new(old_frame));` |
| `src/kernel/src/mm/virt/vmem.rs` | 1522 | `Ok(Some(UserFrame::new(frame_address)))` |

*Internal callers (1):*
- **Upool::alloc** (L187): `Ok(UserFrame::new(addr))`

### `refcount` (impl `UserFrame`) [pub] — 1 external caller(s)
```
pub fn refcount(&self) -> Result<u8, Error>
```
> 
# Description

Returns the current reference count of the underlying physical frame.

# Returns

Upon success, the current reference count of the underlying physical frame is returned.
Upon failure, an error is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/vmem.rs` | 870 | `if probe.refcount()? == 1 {` |


### `address` (impl `UserFrame`) [pub] — 3 external caller(s)
```
pub fn address(&self) -> FrameAddress
```
> 
# Description

Returns the physical address of the target user frame.

# Returns

The physical address of the target user frame.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/vmem.rs` | 361 | `page_table.map(PageAddress::new(vaddr), uframe.address(), false, false, true, ac` |
| `src/kernel/src/mm/virt/vmem.rs` | 882 | `let dst_paddr: usize = new_frame.address().into_raw_value();` |
| `src/kernel/src/mm/virt/vmem.rs` | 887 | `let new_frame_addr: FrameAddress = new_frame.address();` |


### `share` (impl `UserFrame`) [pub] — 1 external caller(s)
```
pub fn share(&self) -> Result<UserFrame, Error>
```
> 
# Description

Adds a new reference to the underlying physical frame and returns a fresh
[`UserFrame`] handle that owns that reference. The two handles share the
same physical frame, and the frame is only reclaimed once both handles are
dropped.

This is the building block for copy-on-write sharing: the parent retains
its handle, the child receives the returned handle.

# Returns

On success, a new [`UserFrame`] that aliases the same physical frame as
`self`. On failure, an error is returned.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 350 | `let child_handle: UserFrame = parent_handle.share()?;` |


### `leak` (impl `UserFrame`) [pub] — 2 external caller(s)
```
pub fn leak(self) -> FrameAddress
```
> 
# Description

Consumes the user frame without freeing the underlying physical frame.

# Returns

The frame address.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/vmem.rs` | 368 | `uframe.leak();` |
| `src/kernel/src/mm/virt/vmem.rs` | 891 | `let _ = new_frame.leak();` |


### `drop` (trait `Drop` for `UserFrame`) [trait-pub] — implicit via `Drop`
```
fn drop(&mut self)
```
> ⚡ **impl Drop for UserFrame**: Compiler inserts call to drop() when value goes out of scope


### `new` (impl `Upool`) [pub(super)] — 1 external caller(s)
```
pub(super) fn new() -> Self
```
> 
# Description

Instantiates a user frame pool.

# Returns

A user frame pool.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 134 | `` |


### `alloc` (impl `Upool`) [pub] — 2 external caller(s)
```
pub fn alloc(&mut self) -> Result<UserFrame, Error>
```
> 
# Description

Allocates a single user frame from the user frame pool.

# Returns

Upon success, a user frame is returned. Upon failure, an error is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/manager.rs` | 180 | `phys_view().initialized,` |
| `src/kernel/src/mm/phys/manager.rs` | 206 | `return Err(Error::new(ErrorCode::InvalidArgument, reason));` |


## Type References

### `UserFrame` [pub] — 19 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 28 | `UserFrame,` |
| `src/kernel/src/mm/virt/manager.rs` | 349 | `let parent_handle: ManuallyDrop<UserFrame> = ManuallyDrop::new(UserFrame::new(fr` |
| `src/kernel/src/mm/virt/manager.rs` | 349 | `let parent_handle: ManuallyDrop<UserFrame> = ManuallyDrop::new(UserFrame::new(fr` |
| `src/kernel/src/mm/virt/manager.rs` | 350 | `let child_handle: UserFrame = parent_handle.share()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 548 | `uframes: &mut Vec<UserFrame>,` |
| `src/kernel/src/mm/virt/vmem.rs` | 33 | `UserFrame,` |
| `src/kernel/src/mm/virt/vmem.rs` | 311 | `uframe: UserFrame,` |
| `src/kernel/src/mm/virt/vmem.rs` | 869 | `let probe: ManuallyDrop<UserFrame> = ManuallyDrop::new(UserFrame::new(src_frame)` |
| `src/kernel/src/mm/virt/vmem.rs` | 869 | `let probe: ManuallyDrop<UserFrame> = ManuallyDrop::new(UserFrame::new(src_frame)` |
| `src/kernel/src/mm/virt/vmem.rs` | 879 | `let new_frame: UserFrame = unsafe { PhysMemoryManager::get_mut() }.alloc_user_fr` |
| `src/kernel/src/mm/virt/vmem.rs` | 895 | `drop(UserFrame::new(old_frame));` |
| `src/kernel/src/mm/virt/vmem.rs` | 1448 | `) -> Result<Option<UserFrame>, Error> {` |
| `src/kernel/src/mm/virt/vmem.rs` | 1522 | `Ok(Some(UserFrame::new(frame_address)))` |
| `src/kernel/src/mm/phys/mod.rs` | 52 | `upool::UserFrame,` |
| `src/kernel/src/mm/elf.rs` | 25 | `phys::UserFrame,` |
| `src/kernel/src/mm/elf.rs` | 273 | `let mut uframe_buf: Vec<UserFrame> = Vec::with_capacity(1);` |
| `src/kernel/src/mm/phys/manager.rs` | 21 | `UserFrame,` |
| `src/kernel/src/mm/phys/manager.rs` | 159 | `/// # Parameters` |
| `src/kernel/src/mm/phys/manager.rs` | 204 | `let reason: &str = "frames vector is not empty";` |

### `Upool` [pub] — 6 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/manager.rs` | 20 | `Upool,` |
| `src/kernel/src/mm/phys/manager.rs` | 70 | `upool: Upool,` |
| `src/kernel/src/mm/phys/manager.rs` | 92 | `// lifecycle gate (raw global state Verus cannot model). The manager-singleton` |
| `src/kernel/src/mm/phys/mod.rs` | 30 | `mm::phys::upool::Upool,` |
| `src/kernel/src/mm/phys/mod.rs` | 134 | `` |
| `src/kernel/src/mm/phys/mod.rs` | 134 | `` |

