# Caller Analysis (LSP): frame.rs

- **Source file:** `/home/ruize/nanvix-phy/src/kernel/src/mm/phys/frame.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 19 |
| Public / trait-pub | 10 |
| Private | 9 |
| Types | 1 |

## Public API — External Callers

### `is_covered` [pub(super)] — 1 external caller(s)
```
pub(super) fn is_covered(phys_addr: PageAligned<PhysicalAddress>) -> bool
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 94 | `}` |


### `alloc_contiguous` [pub(super)] — 1 external caller(s)
```
pub(super) fn alloc_contiguous(count: usize) -> Result<FrameAddress, Error>
```
> # Description

Allocates `count` physically contiguous frames.

# Returns

Returns the base `FrameAddress` of the contiguous range.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/manager.rs` | 298 | `phys_view().inv(),` |


### `init` [pub(super)] — 1 external caller(s)
```
pub(super) unsafe fn init(bitmap: Bitmap) -> Result<(), Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 127 | `while start < end {` |


### `alloc` [pub(super)] — 2 external caller(s)
```
pub(super) fn alloc() -> Result<FrameAddress, Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/manager.rs` | 252 | `phys_view().initialized,` |
| `src/kernel/src/mm/phys/upool.rs` | 186 | `impl Drop for UserFrame {` |


### `book` [pub(super)] — 1 external caller(s)
```
pub(super) fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 95 | `` |


### `share` [pub(super)] — 1 external caller(s)
```
pub(super) fn share(frame: FrameAddress) -> Result<(), Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/upool.rs` | 117 | `/// its handle, the child receives the returned handle.` |


### `refcount` [pub(super)] — 1 external caller(s)
```
pub(super) fn refcount(frame: FrameAddress) -> Result<u8, Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/upool.rs` | 132 | `// On success: a fresh handle aliasing the same physical frame (equal` |


### `free_count` [pub(super)] — 1 external caller(s)
```
pub(super) fn free_count() -> usize
```
> 
# Description

Returns the number of free frames in the system.

# Returns

The number of free frames in the system.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/manager.rs` | 232 | `` |


### `alloc_range` [pub(super)] — 1 external caller(s)
```
pub(super) fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 67 | `// the contents of the (un-viewable) `LinkedList`, they cannot be enumerated in ` |


### `free` [pub(super)] — 4 external caller(s)
```
pub(super) fn free(frame: FrameAddress) -> Result<(), Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/kframe.rs` | 154 | `/// (i.e., the kernel page directory, or a user page directory into which the PD` |
| `src/kernel/src/mm/phys/manager.rs` | 255 | `phys_view().inv(),` |
| `src/kernel/src/mm/phys/manager.rs` | 311 | `error!("{reason}");` |
| `src/kernel/src/mm/phys/upool.rs` | 138 | `&&& handle.inv()` |


## Private Functions — Internal Call Graph

These are implementation details. Listed to show which public functions depend on them.

### `alloc` (impl `Inner`) [private]
```
fn alloc(&mut self) -> Result<FrameAddress, Error>
```
*Called by (2):*
- **Inner::alloc** (L138): `let frame_number: usize = match self.bitmap.alloc() {`
- **alloc** (L732): `instance().alloc()`

### `free` (impl `Inner`) [private]
```
fn free(&mut self, frame: FrameAddress) -> Result<(), Error>
```
*Called by (1):*
- **free** (L779): `instance().free(frame)`

### `share` (impl `Inner`) [private]
```
fn share(&mut self, frame: FrameAddress) -> Result<(), Error>
```
*Called by (1):*
- **share** (L870): `instance().share(frame)`

### `refcount` (impl `Inner`) [private]
```
fn refcount(&self, frame: FrameAddress) -> Result<u8, Error>
```
*Called by (1):*
- **refcount** (L895): `instance().refcount(frame)`

### `alloc_contiguous` (impl `Inner`) [private]
```
fn alloc_contiguous(&mut self, count: usize) -> Result<FrameAddress, Error>
```
*Called by (1):*
- **alloc_contiguous** (L744): `instance().alloc_contiguous(count)`

### `book` (impl `Inner`) [private]
```
fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>
```
*Called by (1):*
- **book** (L823): `instance().book(phys_addr)`

### `is_covered` (impl `Inner`) [private]
```
fn is_covered(&self, phys_addr: PageAligned<PhysicalAddress>) -> bool
```
*Called by (1):*
- **is_covered** (L803): `instance().is_covered(phys_addr)`

### `alloc_range` (impl `Inner`) [private]
```
fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error>
```
*Called by (2):*
- **Inner::alloc_contiguous** (L211): `let frame_number: usize = match self.bitmap.alloc_range(count) {`
- **alloc_range** (L845): `instance().alloc_range(region)`

### `instance` [private]
```
fn instance() -> &'static mut Inner
```
> Returns a mutable reference to the initialized singleton.

*Called by (9):*
- **alloc** (L732): `instance().alloc()`
- **alloc_contiguous** (L744): `instance().alloc_contiguous(count)`
- **free_count** (L757): `let inner = instance();`
- **free** (L779): `instance().free(frame)`
- **is_covered** (L803): `instance().is_covered(phys_addr)`
- **book** (L823): `instance().book(phys_addr)`
- **alloc_range** (L845): `instance().alloc_range(region)`
- **share** (L870): `instance().share(frame)`
- **refcount** (L895): `instance().refcount(frame)`

## Type References

### `Inner` [private] — 0 external reference(s)

