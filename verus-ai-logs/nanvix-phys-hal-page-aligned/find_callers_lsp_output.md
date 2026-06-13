# Caller Analysis (LSP): page.rs

- **Source file:** `/home/ruize/nanvix-phy/src/kernel/src/hal/mem/types/address/aligned/page.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 17 |
| Public / trait-pub | 17 |
| Private | 0 |
| Types | 1 |

## Implicit Callers (runtime/compiler)

### impl Deref for PageAligned<T>
- **Trait:** `Deref`
- **Description:** Compiler inserts deref() for * operator and auto-deref
- **Methods dispatched:** `deref`

> These functions have **no explicit call sites** in source code. The Rust runtime dispatches to them via vtable / lang items.

## Public API — External Callers

### `from_address` (impl `PageAligned<T>`) [pub] — 19 external caller(s)
```
pub fn from_address(addr: T) -> Result<Self, Error>
```
> Constructs a page address from an aligned virtual address.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/elf.rs` | 284 | `let vaddr: PageAligned<VirtualAddress> = PageAligned::from_address(vaddr)?;` |
| `src/kernel/src/mm/elf.rs` | 391 | `Ok((entry, PageAligned::from_address(aligned_last)?))` |
| `src/kernel/src/mm/virt/identity_map.rs` | 416 | `PageAligned::from_address(addr.align_down(PAGE_ALIGNMENT)?)?;` |
| `src/kernel/src/hal/mem/types/address/frame.rs` | 78 | `Ok(Self(PageAligned::from_address(PhysicalAddress::from_number(frame_number))?))` |
| `src/kernel/src/hal/mem/types/address/frame.rs` | 100 | `Ok(Self(PageAligned::from_address(PhysicalAddress::from_raw_value(raw_addr)?)?))` |
| `src/kernel/src/mm/phys/mod.rs` | 85 | `}` |
| `src/kernel/src/hal/mem/types/region.rs` | 340 | `let start: PageAligned<T> = PageAligned::from_address(start)?;` |
| `src/kernel/src/hal/mem/types/region.rs` | 435 | `PageAligned::from_address(PhysicalAddress::from_virtual_address(region.start())?` |
| `src/kernel/src/pm/process/manager/mod.rs` | 568 | `let envp_vaddr: PageAligned<VirtualAddress> = PageAligned::<VirtualAddress>::fro` |
| `src/kernel/src/mm/virt/boot_init.rs` | 102 | `PageAligned::from_address(phys_addr)?;` |
| `src/kernel/src/mm/virt/boot_init.rs` | 105 | `_ => FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value` |
| `src/kernel/src/mm/virt/boot_init.rs` | 230 | `paddr = FrameAddress::new(PageAligned::from_address(` |
| `src/kernel/src/mm/virt/boot_init.rs` | 256 | `FrameAddress::new(PageAligned::from_address(phys_addr)?)` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 643 | `Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(p` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs` | 194 | `Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(p` |
| `src/kernel/src/mm/virt/vmem.rs` | 1053 | `PageAligned::from_address(src.align_down(PAGE_ALIGNMENT))?;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1172 | `match PageAligned::from_address(dst.align_down(PAGE_ALIGNMENT)) {` |
| `src/kernel/src/mm/virt/vmem.rs` | 1356 | `PageAligned::from_address(cur_src.align_down(PAGE_ALIGNMENT))?;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1362 | `PageAligned::from_address(cur_dst.align_down(PAGE_ALIGNMENT))?;` |

*Internal callers (5):*
- **PageAligned<T>::from_raw_value** (L76): `Self::from_address(T::from_raw_value(raw_addr)?)`
- **PageAligned<T>::align_up** (L94): `Self::from_address(self.0.align_up(align)?)`
- **PageAligned<T>::align_down** (L111): `Self::from_address(self.0.align_down(align)?)`
- **PageAligned<VirtualAddress>::into_physical_address** (L191): `PageAligned::from_address(PhysicalAddress::from_raw_value(self.into_raw_value())`
- **PageAligned<PhysicalAddress>::into_virtual_address** (L199): `PageAligned::from_address(self.0.into_virtual_address()).unwrap()`

### `into_inner` (impl `PageAligned<T>`) [pub] — **0 external callers**
```
pub fn into_inner(self) -> T
```

### `eq` (trait `PartialEq` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn eq(&self, other: &Self) -> bool
```

### `fmt` (trait `core::fmt::Debug` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result
```

### `is_aligned` (trait `Address` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn is_aligned(&self, align: Alignment) -> Result<bool, Error>
```
> 
# Description

Checks if the target [`PageAligned`] is aligned to the provided `alignment`.

# Parameters

- `alignment`: The alignment to check.

# Returns

Upon success, `true` is returned if the address is aligned, otherwise `false`. Upon failure,
an error is returned instead.



### `max_addr` (trait `Address` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn max_addr() -> usize
```
> 
# Description

Returns the maximum address for [`PageAligned`].

# Returns

The maximum [`PageAligned`].



### `as_ptr` (trait `Address` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn as_ptr(&self) -> *const u8
```

### `as_mut_ptr` (trait `Address` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn as_mut_ptr(&self) -> *mut u8
```

### `into_raw_value` (trait `Address` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn into_raw_value(self) -> usize
```

### `align_up` (trait `Address` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn align_up(&self, align: Alignment) -> Result<Self, Error>
```
> 
# Description

Aligns the target [`PageAligned`] to the provided `alignment`. If the address is already
aligned, it is returned as is.

# Parameters

- `alignment`: The alignment to align the target address to.

# Returns

Upon success, the aligned address is returned. Upon failure, an error is returned instead.



### `from_raw_value` (trait `Address` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn from_raw_value(raw_addr: usize) -> Result<Self, Error>
```
> 
# Description

Instantiates a new [`PageAligned`] from a raw value.

# Parameters

- `raw_addr`: The raw value.

# Returns

- `Ok(Self)`: The new address.
- `Err(Error::BadAddress)`: If the provided address is invalid.



### `align_down` (trait `Address` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn align_down(&self, align: Alignment) -> Result<Self, Error>
```
> 
# Description

Aligns the target [`PageAligned`] down to the provided `alignment`. If the address is
already aligned, it is returned as is.

# Parameters

- `alignment`: The alignment to align the target address to.

# Returns

Upon success, the aligned address is returned. Upon failure, an error is returned instead.



### `deref` (trait `Deref` for `PageAligned<T>`) [trait-pub] — implicit via `Deref`
```
fn deref(&self) -> &Self::Target
```
> ⚡ **impl Deref for PageAligned<T>**: Compiler inserts deref() for * operator and auto-deref


### `partial_cmp` (trait `PartialOrd` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering>
```

### `cmp` (trait `Ord` for `PageAligned<T>`) [trait-pub] — **0 external callers**
```
fn cmp(&self, other: &Self) -> core::cmp::Ordering
```
*Internal callers (1):*
- **PageAligned<T>::partial_cmp** (L178): `Some(self.cmp(other))`

### `into_physical_address` (impl `PageAligned<VirtualAddress>`) [pub] — **0 external callers**
```
pub fn into_physical_address(self) -> Result<PageAligned<PhysicalAddress>, Error>
```
> Converts a page-aligned virtual address to a page-aligned physical address.


### `into_virtual_address` (impl `PageAligned<PhysicalAddress>`) [pub] — 1 external caller(s)
```
pub fn into_virtual_address(self) -> PageAligned<VirtualAddress>
```
> Converts a page-aligned physical address to a page-aligned virtual address.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mem/types/address/frame.rs` | 74 | `PageAddress::new(PageAligned::into_virtual_address(self.0))` |


## Type References

### `PageAligned` [pub] — 171 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/kframe.rs` | 23 | `PageAligned,` |
| `src/kernel/src/mm/phys/kframe.rs` | 95 | `// Ensure the frame is identity-mapped in the kernel address space so that` |
| `src/kernel/src/mm/phys/kframe.rs` | 96 | `// Deref/DerefMut can safely access it. This lazily installs a page` |
| `src/kernel/src/pm/kcall/mmap.rs` | 12 | `PageAligned,` |
| `src/kernel/src/pm/kcall/mmap.rs` | 39 | `vaddr: PageAligned<VirtualAddress>,` |
| `src/kernel/src/pm/kcall/mmap.rs` | 83 | `let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(arg1 ` |
| `src/kernel/src/pm/kcall/mmap.rs` | 83 | `let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(arg1 ` |
| `src/kernel/src/mm/phys/frame.rs` | 20 | `PageAligned,` |
| `src/kernel/src/mm/phys/frame.rs` | 481 | `final(self).inv(),` |
| `src/kernel/src/mm/phys/frame.rs` | 517 | `///` |
| `src/kernel/src/mm/phys/frame.rs` | 787 | `pub(super) fn alloc_contiguous(count: usize) -> Result<FrameAddress, Error> {` |
| `src/kernel/src/mm/phys/frame.rs` | 792 | `///` |
| `src/kernel/src/hal/io/mmio/region.rs` | 15 | `PageAligned,` |
| `src/kernel/src/hal/io/mmio/region.rs` | 74 | `pub fn base(&self) -> PageAligned<VirtualAddress> {` |
| `src/kernel/src/mm/phys/mod.rs` | 25 | `PageAligned,` |
| `src/kernel/src/mm/phys/mod.rs` | 85 | `}` |
| `src/kernel/src/mm/phys/mod.rs` | 85 | `}` |
| `src/kernel/src/pm/process/manager/mod.rs` | 24 | `PageAligned,` |
| `src/kernel/src/pm/process/manager/mod.rs` | 534 | `let (entry, args_vaddr): (VirtualAddress, PageAligned<VirtualAddress>) =` |
| `src/kernel/src/pm/process/manager/mod.rs` | 568 | `let envp_vaddr: PageAligned<VirtualAddress> = PageAligned::<VirtualAddress>::fro` |

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `into_inner`
- `eq`
- `fmt`
- `is_aligned`
- `max_addr`
- `as_ptr`
- `as_mut_ptr`
- `into_raw_value`
- `align_up`
- `from_raw_value`
- `align_down`
- `partial_cmp`
- `cmp`
- `into_physical_address`

