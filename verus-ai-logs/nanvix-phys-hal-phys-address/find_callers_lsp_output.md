# Caller Analysis (LSP): phys.rs

- **Source file:** `/home/ruize/nanvix-phy/src/kernel/src/hal/mem/types/address/phys.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 16 |
| Public / trait-pub | 16 |
| Private | 0 |
| Types | 1 |

## Public API — External Callers

### `from_number` (impl `PhysicalAddress`) [pub] — 1 external caller(s)
```
pub fn from_number(frame: FrameNumber) -> Self
```
> 
# Description

Constructs a [`PhysicalAddress`] from a [`FrameNumber`].

# Parameters

- `frame`: The frame number.

# Returns

A [`PhysicalAddress`] associated with the given `frame_number`.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mem/types/address/frame.rs` | 78 | `Ok(Self(PageAligned::from_address(PhysicalAddress::from_number(frame_number))?))` |


### `from_mmio_address` (impl `PhysicalAddress`) [pub] — 3 external caller(s)
```
pub unsafe fn from_mmio_address(addr: VirtualAddress) -> Result<Self, Error>
```
> 
# Description

Constructs a physical address from a memory-mapped I/O address.

# Parameters

- `addr`: The memory-mapped I/O address.

# Return Values

Upon success, a physical address associated with the given memory-mapped I/O address is
returned. Upon failure, an error is returned instead.

# Safety

Behavior is undefined if the provided memory-mapped I/O address is invalid.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/boot_init.rs` | 100 | `unsafe { PhysicalAddress::from_mmio_address(mmio_addr)? };` |
| `src/kernel/src/mm/virt/boot_init.rs` | 255 | `unsafe { PhysicalAddress::from_mmio_address(mmio_addr)? };` |
| `src/kernel/src/mm/phys/mod.rs` | 88 | `for region in physical_memory_regions.iter() {` |


### `from_frame_address` (impl `PhysicalAddress`) [pub] — **0 external callers**
```
pub fn from_frame_address(frame_addr: FrameAddress) -> Self
```
> 
# Description

Constructs a [`PhysicalAddress`] from a [`FrameAddress`].

# Parameters

- `frame_addr`: The frame address.

# Returns

A [`PhysicalAddress`] associated with the given `frame_addr`.



### `from_into_frame_address` (impl `PhysicalAddress`) [pub] — **0 external callers**
```
pub fn from_into_frame_address(frame_addr: FrameAddress) -> Self
```

### `from_virtual_address` (impl `PhysicalAddress`) [pub] — 2 external caller(s)
```
pub fn from_virtual_address(addr: VirtualAddress) -> Result<Self, Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/kernel_vas.rs` | 68 | `if PhysicalAddress::from_virtual_address(region.start()).is_ok() {` |
| `src/kernel/src/hal/mem/types/region.rs` | 435 | `PageAligned::from_address(PhysicalAddress::from_virtual_address(region.start())?` |

*Internal callers (3):*
- **PhysicalAddress::from_raw_value** (L153): `Self::from_virtual_address(VirtualAddress::from_raw_value(value))`
- **PhysicalAddress::align_up** (L180): `Self::from_virtual_address(aligned)`
- **PhysicalAddress::align_down** (L198): `Self::from_virtual_address(self.0.align_down(align))`

### `into_frame_number` (impl `PhysicalAddress`) [pub] — 4 external caller(s)
```
pub fn into_frame_number(self) -> FrameNumber
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mem/types/address/frame.rs` | 82 | `self.0.into_frame_number()` |
| `src/kernel/src/mm/phys/frame.rs` | 482 | `let frame_number: usize = phys_addr.into_frame_number().into_raw_value();` |
| `src/kernel/src/mm/phys/frame.rs` | 518 | `let frame_number: usize = phys_addr.into_frame_number().into_raw_value();` |
| `src/kernel/src/mm/phys/frame.rs` | 569 | `let start_frame_number: usize = region.start().into_frame_number().into_raw_valu` |


### `into_virtual_address` (impl `PhysicalAddress`) [pub] — 2 external caller(s)
```
pub fn into_virtual_address(self) -> VirtualAddress
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/kmain.rs` | 441 | `let raw_start: usize = module.region_base().into_virtual_address().into_raw_valu` |
| `src/kernel/src/hal/mem/types/address/aligned/page.rs` | 199 | `impl<T: Address> PartialOrd for PageAligned<T> {` |


### `fmt` (trait `core::fmt::Debug` for `PhysicalAddress`) [trait-pub] — **0 external callers**
```
fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result
```

### `is_aligned` (trait `Address` for `PhysicalAddress`) [trait-pub] — **0 external callers**
```
fn is_aligned(&self, align: Alignment) -> Result<bool, Error>
```
> 
# Description

Checks if the target [`PhysicalAddress`] is aligned to the provided `alignment`.

# Parameters

- `alignment`: The alignment to check.

# Returns

Upon success, `true` is returned if the address is aligned, otherwise `false`. Upon failure,
an error is returned instead.



### `max_addr` (trait `Address` for `PhysicalAddress`) [trait-pub] — **0 external callers**
```
fn max_addr() -> usize
```
> 
# Description

Returns the maximum address for [`PhysicalAddress`].

# Returns

The maximum [`PhysicalAddress`].



### `into_raw_value` (trait `Address` for `PhysicalAddress`) [trait-pub] — 2 external caller(s)
```
fn into_raw_value(self) -> usize
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/kmain.rs` | 287 | `let elf: &Elf32Fhdr = unsafe { Elf32Fhdr::from_address(kmod.start().into_raw_val` |
| `src/kernel/src/mm/virt/identity_map.rs` | 420 | `let raw: usize = addr.into_raw_value();` |


### `as_ptr` (trait `Address` for `PhysicalAddress`) [trait-pub] — **0 external callers**
```
fn as_ptr(&self) -> *const u8
```

### `as_mut_ptr` (trait `Address` for `PhysicalAddress`) [trait-pub] — **0 external callers**
```
fn as_mut_ptr(&self) -> *mut u8
```

### `from_raw_value` (trait `Address` for `PhysicalAddress`) [trait-pub] — 12 external caller(s)
```
fn from_raw_value(value: usize) -> Result<Self, Error>
```
> 
# Description

Instantiates a new [`PhysicalAddress`] from a raw value.

# Parameters

- `raw_addr`: The raw value.

# Returns

- `Ok(Self)`: The new address.
- `Err(Error::BadAddress)`: If the provided address is invalid.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mem/types/address/frame.rs` | 86 | `Ok(Self(PageAligned::from_address(PhysicalAddress::from_raw_value(raw_addr)?)?))` |
| `src/kernel/src/multibin.rs` | 86 | `PhysicalAddress::from_raw_value(entry_phys_addr)?,` |
| `src/kernel/src/multibin.rs` | 88 | `PhysicalAddress::from_raw_value(initrd_base)?,` |
| `src/kernel/src/mm/virt/boot_init.rs` | 105 | `_ => FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value` |
| `src/kernel/src/mm/virt/boot_init.rs` | 231 | `PhysicalAddress::from_raw_value(raw_vaddr)?,` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs` | 194 | `Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(p` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 643 | `Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(p` |
| `src/kernel/src/mm/virt/identity_map.rs` | 195 | `let src_addr: PhysicalAddress = PhysicalAddress::from_raw_value(src as usize)?;` |
| `src/kernel/src/mm/virt/identity_map.rs` | 198 | `let dst_addr: PhysicalAddress = PhysicalAddress::from_raw_value(dst as usize)?;` |
| `src/kernel/src/mm/virt/identity_map.rs` | 250 | `let base_addr: PhysicalAddress = PhysicalAddress::from_raw_value(base as usize)?` |
| `src/kernel/src/hal/platform/microvm/mod.rs` | 685 | `PhysicalAddress::from_raw_value(initrd_base)?,` |
| `src/kernel/src/hal/mem/types/address/aligned/page.rs` | 191 | `impl<T: Address> PartialEq for PageAligned<T> {` |


### `align_up` (trait `Address` for `PhysicalAddress`) [trait-pub] — **0 external callers**
```
fn align_up(&self, align: Alignment) -> Result<Self, Error>
```
> 
# Description

Aligns the target [`PhysicalAddress`] to the provided `alignment`. If the address is already
aligned, it is returned as is.

# Parameters

- `alignment`: The alignment to align the target address to.

# Returns

Upon success, the aligned address is returned. Upon failure, an error is returned instead.



### `align_down` (trait `Address` for `PhysicalAddress`) [trait-pub] — 1 external caller(s)
```
fn align_down(&self, align: Alignment) -> Result<Self, Error>
```
> 
# Description

Aligns the target [`PhysicalAddress`] down to the provided `alignment`. If the address is
already aligned, it is returned as is.

# Parameters

- `alignment`: The alignment to align the target address to.

# Returns

Upon success, the aligned address is returned. Upon failure, an error is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/identity_map.rs` | 417 | `PageAligned::from_address(addr.align_down(PAGE_ALIGNMENT)?)?;` |


## Type References

### `PhysicalAddress` [pub] — 76 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/frame.rs` | 21 | `PhysicalAddress,` |
| `src/kernel/src/mm/phys/frame.rs` | 481 | `fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error>` |
| `src/kernel/src/mm/phys/frame.rs` | 517 | `fn is_covered(&self, phys_addr: PageAligned<PhysicalAddress>) -> bool {` |
| `src/kernel/src/mm/phys/frame.rs` | 567 | `region: &TruncatedMemoryRegion<PhysicalAddress>,` |
| `src/kernel/src/mm/phys/frame.rs` | 802 | `})` |
| `src/kernel/src/mm/phys/frame.rs` | 822 | `///` |
| `src/kernel/src/mm/phys/frame.rs` | 844 | `let inner = instance();` |
| `src/kernel/src/mm/phys/mod.rs` | 26 | `PhysicalAddress,` |
| `src/kernel/src/mm/phys/mod.rs` | 60 | `// iterator cannot be given a Verus `for`-loop specification from this crate (th` |
| `src/kernel/src/mm/phys/mod.rs` | 85 | `info!("booking physical memory regions ...");` |
| `src/kernel/src/mm/phys/mod.rs` | 88 | `for region in physical_memory_regions.iter() {` |
| `src/kernel/src/mm/phys/mod.rs` | 120 | `info!("booking memory-mapped i/o regions ...");` |
| `src/kernel/src/kmod.rs` | 8 | `use crate::hal::mem::PhysicalAddress;` |
| `src/kernel/src/kmod.rs` | 16 | `start: PhysicalAddress,` |
| `src/kernel/src/kmod.rs` | 22 | `region_base: PhysicalAddress,` |
| `src/kernel/src/kmod.rs` | 40 | `pub fn new(start: PhysicalAddress, size: usize, cmdline: &'static str) -> Self {` |
| `src/kernel/src/kmod.rs` | 53 | `start: PhysicalAddress,` |
| `src/kernel/src/kmod.rs` | 55 | `region_base: PhysicalAddress,` |
| `src/kernel/src/kmod.rs` | 70 | `pub fn start(&self) -> PhysicalAddress {` |
| `src/kernel/src/kmod.rs` | 80 | `pub fn region_base(&self) -> PhysicalAddress {` |

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `from_frame_address`
- `from_into_frame_address`
- `fmt`
- `is_aligned`
- `max_addr`
- `as_ptr`
- `as_mut_ptr`
- `align_up`

