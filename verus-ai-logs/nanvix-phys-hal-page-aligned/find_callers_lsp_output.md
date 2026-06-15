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

### `from_address` (impl `PageAligned<T>`) [pub] — 20 external caller(s)
```
pub fn from_address(addr: T) -> Result<Self, Error>
```
> Constructs a page address from an aligned virtual address.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mem/types/address/frame.rs` | 78 | `Ok(Self(PageAligned::from_address(PhysicalAddress::from_number(frame_number))?))` |
| `src/kernel/src/hal/mem/types/address/frame.rs` | 86 | `Ok(Self(PageAligned::from_address(PhysicalAddress::from_raw_value(raw_addr)?)?))` |
| `src/kernel/src/mm/phys/mod.rs` | 85 | `info!("booking physical memory regions ...");` |
| `src/kernel/src/hal/mem/types/region.rs` | 340 | `let start: PageAligned<T> = PageAligned::from_address(start)?;` |
| `src/kernel/src/hal/mem/types/region.rs` | 435 | `PageAligned::from_address(PhysicalAddress::from_virtual_address(region.start())?` |
| `src/kernel/src/pm/process/manager/mod.rs` | 568 | `let envp_vaddr: PageAligned<VirtualAddress> = PageAligned::<VirtualAddress>::fro` |
| `src/kernel/src/mm/virt/boot_init.rs` | 102 | `PageAligned::from_address(phys_addr)?;` |
| `src/kernel/src/mm/virt/boot_init.rs` | 105 | `_ => FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value` |
| `src/kernel/src/mm/virt/boot_init.rs` | 230 | `paddr = FrameAddress::new(PageAligned::from_address(` |
| `src/kernel/src/mm/virt/boot_init.rs` | 256 | `FrameAddress::new(PageAligned::from_address(phys_addr)?)` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs` | 194 | `Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(p` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 643 | `Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(p` |
| `src/kernel/src/mm/virt/identity_map.rs` | 417 | `PageAligned::from_address(addr.align_down(PAGE_ALIGNMENT)?)?;` |
| `src/kernel/src/mm/virt/vmem.rs` | 978 | `PageAligned::from_address(vaddr.align_down(PAGE_ALIGNMENT))?;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1053 | `PageAligned::from_address(src.align_down(PAGE_ALIGNMENT))?;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1172 | `match PageAligned::from_address(dst.align_down(PAGE_ALIGNMENT)) {` |
| `src/kernel/src/mm/virt/vmem.rs` | 1356 | `PageAligned::from_address(cur_src.align_down(PAGE_ALIGNMENT))?;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1362 | `PageAligned::from_address(cur_dst.align_down(PAGE_ALIGNMENT))?;` |
| `src/kernel/src/mm/elf.rs` | 284 | `let vaddr: PageAligned<VirtualAddress> = PageAligned::from_address(vaddr)?;` |
| `src/kernel/src/mm/elf.rs` | 391 | `Ok((entry, PageAligned::from_address(aligned_last)?))` |

*Internal callers (5):*
- **PageAligned<T>::from_raw_value** (L76): `Self::from_address(T::from_raw_value(raw_addr)?)`
- **PageAligned<T>::align_up** (L94): `Self::from_address(self.0.align_up(align)?)`
- **PageAligned<T>::align_down** (L111): `Self::from_address(self.0.align_down(align)?)`
- **PageAligned<VirtualAddress>::into_physical_address** (L191): `PageAligned::from_address(PhysicalAddress::from_raw_value(self.into_raw_value())`
- **PageAligned<PhysicalAddress>::into_virtual_address** (L199): `PageAligned::from_address(self.0.into_virtual_address()).unwrap()`

### `into_inner` (impl `PageAligned<T>`) [pub] — 21 external caller(s)
```
pub fn into_inner(self) -> T
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/kernel_vas.rs` | 144 | `while vaddr.into_inner() < end {` |
| `src/kernel/src/io/kcall/mmio_info.rs` | 62 | `MmioRegionInfo::new(base.into_inner(), size, perm)` |
| `src/kernel/src/pm/process/manager/mod.rs` | 554 | `Self::write_nul_terminated_to_user(&mut vmem, args_vaddr.into_inner(), args)?;` |
| `src/kernel/src/pm/process/manager/mod.rs` | 581 | `Self::write_nul_terminated_to_user(&mut vmem, envp_vaddr.into_inner(), env)?;` |
| `src/kernel/src/pm/process/manager/mod.rs` | 599 | `user_stack_base: user_stack.base().into_inner(),` |
| `src/kernel/src/pm/kcall/mcopy.rs` | 51 | `kpage.base().into_virtual_address().into_inner(),` |
| `src/kernel/src/pm/kcall/mcopy.rs` | 52 | `src_vaddr.into_inner(),` |
| `src/kernel/src/pm/kcall/mcopy.rs` | 59 | `dst_vaddr.into_inner(),` |
| `src/kernel/src/pm/kcall/mcopy.rs` | 60 | `kpage.base().into_virtual_address().into_inner(),` |
| `src/kernel/src/mm/virt/boot_init.rs` | 97 | `let mmio_addr: VirtualAddress = region.start().into_inner();` |
| `src/kernel/src/mm/virt/vmem.rs` | 317 | `if !Self::is_user_addr(vaddr.into_inner()) {` |
| `src/kernel/src/mm/virt/vmem.rs` | 389 | `if !Self::is_user_addr(vaddr.into_inner()) {` |
| `src/kernel/src/mm/virt/vmem.rs` | 739 | `if !Self::is_user_addr(vaddr.into_inner()) {` |
| `src/kernel/src/mm/virt/vmem.rs` | 772 | `if !Self::is_user_addr(vaddr.into_inner()) {` |
| `src/kernel/src/mm/virt/vmem.rs` | 810 | `if !Self::is_user_addr(vaddr.into_inner()) {` |
| `src/kernel/src/mm/virt/vmem.rs` | 848 | `if !Self::is_user_addr(vaddr.into_inner()) {` |
| `src/kernel/src/mm/virt/vmem.rs` | 1450 | `if !Self::is_user_addr(vaddr.into_inner()) {` |
| `src/kernel/src/mm/virt/vmem.rs` | 1532 | `if !Self::is_user_addr(vaddr.into_inner()) {` |
| `src/kernel/src/mm/virt/vmem.rs` | 1606 | `if !Self::is_kernel_addr(vaddr.into_inner()) {` |
| `src/kernel/src/mm/virt/manager.rs` | 570 | `if !Vmem::is_user_region(vaddr.into_inner(), range_size) {` |
| `src/kernel/src/mm/elf.rs` | 363 | `vaddr.into_inner(),` |


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

### `into_raw_value` (trait `Address` for `PageAligned<T>`) [trait-pub] — 62 external caller(s)
```
fn into_raw_value(self) -> usize
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/io/mmio/allocator.rs` | 189 | `let start: usize = region.start().into_raw_value();` |
| `src/kernel/src/hal/io/mmio/allocator.rs` | 193 | `let reg_start: usize = entry.region.start().into_raw_value();` |
| `src/kernel/src/hal/arch/shared/cpu/interrupt/ioapic.rs` | 78 | `let base_addr: usize = self.base.base().into_raw_value();` |
| `src/kernel/src/mm/phys/mod.rs` | 81 | `)]` |
| `src/kernel/src/mm/phys/frame.rs` | 589 | `let region_start: usize = region.start().into_raw_value();` |
| `src/kernel/src/hal/arch/shared/cpu/interrupt/xapic.rs` | 64 | `ptr: xapic::Xapic::new(self.base.base().into_raw_value() as *mut u32),` |
| `src/kernel/src/hal/arch/shared/cpu/interrupt/xapic.rs` | 199 | `self.base.base().into_raw_value()` |
| `src/kernel/src/pm/process/manager/unsafe.rs` | 381 | `let base: usize = user_stack.base().into_raw_value();` |
| `src/kernel/src/pm/process/manager/unsafe.rs` | 382 | `let top: usize = user_stack.top().into_raw_value();` |
| `src/kernel/src/mm/virt/manager.rs` | 586 | `.into_raw_value()` |
| `src/kernel/src/mm/virt/manager.rs` | 633 | `match PageAligned::from_raw_value(vaddr.into_raw_value() + mem::PAGE_SIZE) {` |
| `src/kernel/src/mm/virt/manager.rs` | 652 | `rollback_addr.into_raw_value() + mem::PAGE_SIZE,` |
| `src/kernel/src/mm/virt/vmem.rs` | 245 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 326 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 592 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 630 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 668 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 746 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 779 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 817 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 979 | `let offset: usize = vaddr.into_raw_value() - page_aligned.into_raw_value();` |
| `src/kernel/src/mm/virt/vmem.rs` | 1054 | `let offset: usize = src.into_raw_value() - vaddr.into_raw_value();` |
| `src/kernel/src/mm/virt/vmem.rs` | 1186 | `let offset: usize = dst.into_raw_value() - vaddr.into_raw_value();` |
| `src/kernel/src/mm/virt/vmem.rs` | 1357 | `let src_offset: usize = cur_src.into_raw_value() - src_page.into_raw_value();` |
| `src/kernel/src/mm/virt/vmem.rs` | 1363 | `let dst_offset: usize = cur_dst.into_raw_value() - dst_page.into_raw_value();` |
| `src/kernel/src/mm/virt/vmem.rs` | 1419 | `let base: *mut u8 = dst.into_raw_value() as *mut u8;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1466 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 1541 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 1615 | `::sys::mm::align_down(vaddr.into_raw_value(), PGTAB_ALIGNMENT),` |
| `src/kernel/src/mm/virt/vmem.rs` | 1654 | `FrameAddress::new(PageAligned::from_raw_value(vaddr.into_raw_value())?);` |
| ... | | +32 more |

*Internal callers (1):*
- **PageAligned<VirtualAddress>::into_physical_address** (L191): `PageAligned::from_address(PhysicalAddress::from_raw_value(self.into_raw_value())`

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



### `from_raw_value` (trait `Address` for `PageAligned<T>`) [trait-pub] — 33 external caller(s)
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



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/pm/process/manager/mod.rs` | 590 | `UserStack::new(PageAligned::from_raw_value(USER_STACK_TOP_RAW)?);` |
| `src/kernel/src/pm/process/manager/mod.rs` | 612 | `PageAligned::from_raw_value(user_stack.top().into_raw_value() - USER_STACK_MIN_S` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2115 | `match PageAligned::from_raw_value(raw_addr) {` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2203 | `PageAligned::from_raw_value(current_vaddr.into_raw_value() + batch * PAGE_SIZE)?` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2233 | `match PageAligned::from_raw_value(raw) {` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2297 | `let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(raw_vaddr)?` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2303 | `let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(raw_vaddr)?` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2309 | `let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(raw_vaddr)?` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2495 | `let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(page_addr)?` |
| `src/kernel/src/pm/process/manager/unsafe.rs` | 396 | `let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(raw_a` |
| `src/kernel/src/mm/virt/manager.rs` | 486 | `let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(page_` |
| `src/kernel/src/mm/virt/manager.rs` | 584 | `check_addr = PageAligned::from_raw_value(` |
| `src/kernel/src/mm/virt/manager.rs` | 633 | `match PageAligned::from_raw_value(vaddr.into_raw_value() + mem::PAGE_SIZE) {` |
| `src/kernel/src/mm/virt/manager.rs` | 651 | `rollback_addr = match PageAligned::from_raw_value(` |
| `src/kernel/src/mm/virt/vmem.rs` | 714 | `let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(raw_vaddr)?` |
| `src/kernel/src/mm/virt/vmem.rs` | 939 | `let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(page)?;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1654 | `FrameAddress::new(PageAligned::from_raw_value(vaddr.into_raw_value())?);` |
| `src/kernel/src/mm/virt/identity_map.rs` | 472 | `PageAligned::from_raw_value(start_raw + i * mem::PAGE_SIZE)?;` |
| `src/kernel/src/mm/virt/boot_init.rs` | 239 | `PageAddress::new(PageAligned::from_raw_value(raw_vaddr)?),` |
| `src/kernel/src/mm/ustack.rs` | 79 | `PageAligned::from_raw_value(self.base.into_raw_value() + self.size()).unwrap()` |
| `src/kernel/src/mm/kstack.rs` | 157 | `PageAligned::from_raw_value(self.kpages[0].base().into_raw_value()).unwrap()` |
| `src/kernel/src/mm/kstack.rs` | 180 | `PageAligned::from_raw_value(base + size).unwrap()` |
| `src/kernel/src/hal/mem/types/address/pd.rs` | 26 | `Ok(Self(PageAligned::from_raw_value(value)?))` |
| `src/kernel/src/mm/kernel_vas.rs` | 169 | `Some(raw_addr) => vaddr = PageAligned::from_raw_value(raw_addr)?,` |
| `src/kernel/src/pm/kcall/mcopy.rs` | 115 | `let src_vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(a` |
| `src/kernel/src/pm/kcall/mcopy.rs` | 126 | `let dst_vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(a` |
| `src/kernel/src/pm/kcall/mctrl.rs` | 88 | `let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(arg1 ` |
| `src/kernel/src/pm/kcall/munmap.rs` | 71 | `let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(arg1 ` |
| `src/kernel/src/hal/platform/microvm/mod.rs` | 749 | `PageAligned::from_raw_value(ramfs_base)?,` |
| `src/kernel/src/hal/platform/microvm/mod.rs` | 902 | `PageAligned::from_raw_value(::config::microvm::DEFAULT_MICROVM_CTRL_BASE)?,` |
| ... | | +3 more |


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


### `partial_cmp` (trait `PartialOrd` for `PageAligned<T>`) [trait-pub] — 1 external caller(s)
```
fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mem/types/address/page.rs` | 42 | `self.0.partial_cmp(&other.0)` |


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

### `PageAligned` [pub] — 176 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/frame.rs` | 20 | `PageAligned,` |
| `src/kernel/src/mm/phys/frame.rs` | 481 | `fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>` |
| `src/kernel/src/mm/phys/frame.rs` | 517 | `fn is_covered(&self, phys_addr: PageAligned<PhysicalAddress>) -> bool {` |
| `src/kernel/src/mm/phys/frame.rs` | 802 | `})` |
| `src/kernel/src/mm/phys/frame.rs` | 822 | `///` |
| `src/kernel/src/hal/io/mmio/region.rs` | 15 | `PageAligned,` |
| `src/kernel/src/hal/io/mmio/region.rs` | 74 | `pub fn base(&self) -> PageAligned<VirtualAddress> {` |
| `src/kernel/src/mm/phys/mod.rs` | 25 | `PageAligned,` |
| `src/kernel/src/mm/phys/mod.rs` | 85 | `info!("booking physical memory regions ...");` |
| `src/kernel/src/mm/phys/mod.rs` | 85 | `info!("booking physical memory regions ...");` |
| `src/kernel/src/pm/process/manager/mod.rs` | 24 | `PageAligned,` |
| `src/kernel/src/pm/process/manager/mod.rs` | 534 | `let (entry, args_vaddr): (VirtualAddress, PageAligned<VirtualAddress>) =` |
| `src/kernel/src/pm/process/manager/mod.rs` | 568 | `let envp_vaddr: PageAligned<VirtualAddress> = PageAligned::<VirtualAddress>::fro` |
| `src/kernel/src/pm/process/manager/mod.rs` | 568 | `let envp_vaddr: PageAligned<VirtualAddress> = PageAligned::<VirtualAddress>::fro` |
| `src/kernel/src/pm/process/manager/mod.rs` | 590 | `UserStack::new(PageAligned::from_raw_value(USER_STACK_TOP_RAW)?);` |
| `src/kernel/src/pm/process/manager/mod.rs` | 611 | `let initial_stack_base: PageAligned<VirtualAddress> =` |
| `src/kernel/src/pm/process/manager/mod.rs` | 612 | `PageAligned::from_raw_value(user_stack.top().into_raw_value() - USER_STACK_MIN_S` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2114 | `let vaddr: PageAligned<VirtualAddress> =` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2115 | `match PageAligned::from_raw_value(raw_addr) {` |
| `src/kernel/src/pm/process/manager/mod.rs` | 2171 | `vaddr: PageAligned<VirtualAddress>,` |

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `eq`
- `fmt`
- `is_aligned`
- `max_addr`
- `as_ptr`
- `as_mut_ptr`
- `align_up`
- `align_down`
- `cmp`
- `into_physical_address`

