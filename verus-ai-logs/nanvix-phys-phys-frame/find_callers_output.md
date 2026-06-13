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
> 
# Description

Checks whether the frame allocator tracks the frame at the given physical address.

# Returns

Returns `true` when the frame allocator tracks the frame at `phys_addr`.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 94 | `phys_view().initialized,` |


### `alloc_range` [pub(super)] — 1 external caller(s)
```
pub(super) fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error>
```
> Book every frame in the given physical memory region.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 67 | `match result {` |


### `init` [pub(super)] — 1 external caller(s)
```
pub(super) unsafe fn init(bitmap: Bitmap) -> Result<(), Error>
```
> Initialize the frame allocator singleton.

# Safety

Must be called exactly once during boot, before any other function
in this module.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 127 | `start += mem::FRAME_SIZE;` |


### `refcount` [pub(super)] — 1 external caller(s)
```
pub(super) fn refcount(frame: FrameAddress) -> Result<u8, Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/upool.rs` | 142 | `/// # Returns` |


### `alloc_contiguous` [pub(super)] — 1 external caller(s)
```
pub(super) fn alloc_contiguous(count: usize) -> Result<FrameAddress, Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/manager.rs` | 298 | `/// [`config::kernel::KERNEL_WATERMARK`].` |


### `free` [pub(super)] — 4 external caller(s)
```
pub(super) fn free(frame: FrameAddress) -> Result<(), Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/kframe.rs` | 170 | `fn deref(&self) -> &Self::Target {` |
| `src/kernel/src/mm/phys/manager.rs` | 255 | `///` |
| `src/kernel/src/mm/phys/manager.rs` | 311 | `Ok(()) => crate::mm::phys::phys_view().frames.free_count()` |
| `src/kernel/src/mm/phys/upool.rs` | 148 | `requires` |


### `free_count` [pub(super)] — 1 external caller(s)
```
pub(super) fn free_count() -> usize
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/manager.rs` | 232 | `invariant` |


### `book` [pub(super)] — 1 external caller(s)
```
pub(super) fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>
```
> Reserve a frame so [`alloc`] will skip it.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 95 | `match result {` |


### `share` [pub(super)] — 1 external caller(s)
```
pub(super) fn share(frame: FrameAddress) -> Result<(), Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/upool.rs` | 127 | `let this: ManuallyDrop<Self> = ManuallyDrop::new(self);` |


### `alloc` [pub(super)] — 2 external caller(s)
```
pub(super) fn alloc() -> Result<FrameAddress, Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/manager.rs` | 252 | `Ok(())` |
| `src/kernel/src/mm/phys/upool.rs` | 216 | `/// Thin facade over the module-level [`frame`](super::frame) allocator. Exists ` |


## Private Functions — Internal Call Graph

These are implementation details. Listed to show which public functions depend on them.

### `alloc` (impl `Inner`) [private]
```
fn alloc(&mut self) -> Result<FrameAddress, Error>
```
*Called by (2):*
- **Inner::alloc** (L138): `let frame_number: usize = match self.bitmap.alloc() {`
- **alloc** (L711): `instance().alloc()`

### `free` (impl `Inner`) [private]
```
fn free(&mut self, frame: FrameAddress) -> Result<(), Error>
```
*Called by (1):*
- **free** (L775): `instance().free(frame)`

### `share` (impl `Inner`) [private]
```
fn share(&mut self, frame: FrameAddress) -> Result<(), Error>
```
*Called by (1):*
- **share** (L818): `instance().share(frame)`

### `refcount` (impl `Inner`) [private]
```
fn refcount(&self, frame: FrameAddress) -> Result<u8, Error>
```
*Called by (1):*
- **refcount** (L839): `instance().refcount(frame)`

### `alloc_contiguous` (impl `Inner`) [private]
```
fn alloc_contiguous(&mut self, count: usize) -> Result<FrameAddress, Error>
```
*Called by (1):*
- **alloc_contiguous** (L738): `instance().alloc_contiguous(count)`

### `book` (impl `Inner`) [private]
```
fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>
```
*Called by (1):*
- **book** (L793): `instance().book(phys_addr)`

### `is_covered` (impl `Inner`) [private]
```
fn is_covered(&self, phys_addr: PageAligned<PhysicalAddress>) -> bool
```
*Called by (1):*
- **is_covered** (L788): `instance().is_covered(phys_addr)`

### `alloc_range` (impl `Inner`) [private]
```
fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error>
```
*Called by (2):*
- **Inner::alloc_contiguous** (L211): `let frame_number: usize = match self.bitmap.alloc_range(count) {`
- **alloc_range** (L798): `instance().alloc_range(region)`

### `instance` [private]
```
fn instance() -> &'static mut Inner
```
> Returns a mutable reference to the initialized singleton.

*Called by (9):*
- **alloc** (L711): `instance().alloc()`
- **alloc_contiguous** (L738): `instance().alloc_contiguous(count)`
- **free_count** (L758): `let inner = instance();`
- **free** (L775): `instance().free(frame)`
- **is_covered** (L788): `instance().is_covered(phys_addr)`
- **book** (L793): `instance().book(phys_addr)`
- **alloc_range** (L798): `instance().alloc_range(region)`
- **share** (L818): `instance().share(frame)`
- **refcount** (L839): `instance().refcount(frame)`

## Type References

### `Inner` [private] — 0 external reference(s)

