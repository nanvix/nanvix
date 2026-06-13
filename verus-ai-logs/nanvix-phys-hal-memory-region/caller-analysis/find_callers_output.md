No cross-crate dependents, but crate is multi-module — using LSP for intra-crate callers.
# Caller Analysis (LSP): region.rs

- **Source file:** `/home/ruize/nanvix-phy/src/kernel/src/hal/mem/types/region.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 28 |
| Public / trait-pub | 28 |
| Private | 0 |
| Types | 4 |

## Public API — External Callers

### `new` (impl `MmioCachePolicy`) [pub] — **0 external callers**
```
pub const fn new(write_through: bool, cache_enabled: bool) -> Self
```
> 
# Description

Creates a new MMIO cache policy.

# Parameters

- `write_through`: If `true`, enables the Write-Through attribute (PWT=1).
- `cache_enabled`: If `true`, enables caching for the page (PCD=0).

# Returns

A new [`MmioCachePolicy`] instance.



### `write_through` (impl `MmioCachePolicy`) [pub] — 1 external caller(s)
```
pub fn write_through(&self) -> bool
```
> 
# Description

Returns whether the Write-Through attribute is set.

# Returns

`true` if the page is mapped with the Write-Through attribute (PWT=1).



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/boot_init.rs` | 242 | `cache_policy.write_through(),` |


### `cache_enabled` (impl `MmioCachePolicy`) [pub] — 1 external caller(s)
```
pub fn cache_enabled(&self) -> bool
```
> 
# Description

Returns whether caching is enabled.

# Returns

`true` if caching is enabled for the page (PCD=0).



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/boot_init.rs` | 243 | `cache_policy.cache_enabled(),` |


### `new` (impl `MemoryRegion<T>`) [pub] — 5 external caller(s)
```
pub fn new(
        name: &str,
        start: T,
        size: usize,
        typ: MemoryRegionType,
        perm: AccessPermission,
    ) -> Result<Self, Error>
```
> Creates a new memory region.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/kimage.rs` | 53 | `let text = MemoryRegion::new(` |
| `src/kernel/src/kimage.rs` | 69 | `let rodata = MemoryRegion::new(` |
| `src/kernel/src/kimage.rs` | 86 | `Some(MemoryRegion::new(` |
| `src/kernel/src/kimage.rs` | 105 | `let bss = MemoryRegion::new(` |
| `src/kernel/src/kmain.rs` | 475 | `MemoryRegion::new(name, start, aligned_size, typ, AccessPermission::RDWR)` |

*Internal callers (1):*
- **TruncatedMemoryRegion<T>::new** (L310): `Ok(Self(MemoryRegion::new(name, start, size, typ, perm)?))`

### `size` (impl `MemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn size(&self) -> usize
```
> Returns the size of the target memory region.

*Internal callers (2):*
- **TruncatedMemoryRegion<T>::from_memory_region** (L348): `let size: usize = region.size();`
- **TruncatedMemoryRegion<T>::size** (L369): `self.0.size()`

### `typ` (impl `MemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn typ(&self) -> MemoryRegionType
```
> Returns the type of the target memory region.

*Internal callers (2):*
- **TruncatedMemoryRegion<T>::from_memory_region** (L349): `let typ: MemoryRegionType = region.typ();`
- **TruncatedMemoryRegion<T>::typ** (L374): `self.0.typ()`

### `name` (impl `MemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn name(&self) -> String
```
*Internal callers (3):*
- **TruncatedMemoryRegion<T>::from_memory_region** (L347): `let name: String = region.name();`
- **TruncatedMemoryRegion<T>::name** (L359): `self.0.name()`
- **TruncatedMemoryRegion<PhysicalAddress>::from_virtual_memory_region** (L439): `let name: String = region.name();`

### `start` (impl `MemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn start(&self) -> T
```
> Returns the first valid address that lies in the target memory region.

*Internal callers (3):*
- **TruncatedMemoryRegion<T>::from_memory_region** (L345): `let start: T = region.start().align_down(PAGE_ALIGNMENT)?;`
- **TruncatedMemoryRegion<T>::start** (L364): `self.0.start()`
- **TruncatedMemoryRegion<PhysicalAddress>::from_virtual_memory_region** (L441): `PageAligned::from_address(PhysicalAddress::from_virtual_address(region.start())?`

### `perm` (impl `MemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn perm(&self) -> AccessPermission
```
> Returns the permissions of the target memory region.

*Internal callers (2):*
- **TruncatedMemoryRegion<T>::from_memory_region** (L350): `let perm: AccessPermission = region.perm();`
- **TruncatedMemoryRegion<T>::perm** (L379): `self.0.perm()`

### `cache_policy` (impl `MemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn cache_policy(&self) -> Option<MmioCachePolicy>
```
> 
# Description

Returns the MMIO cache policy of the target memory region, if set.

# Returns

The [`MmioCachePolicy`] if one was assigned, or `None` otherwise.


*Internal callers (2):*
- **TruncatedMemoryRegion<T>::from_memory_region** (L344): `let cache_policy: Option<MmioCachePolicy> = region.cache_policy();`
- **TruncatedMemoryRegion<T>::cache_policy** (L392): `self.0.cache_policy()`

### `set_cache_policy` (impl `MemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn set_cache_policy(&mut self, policy: MmioCachePolicy)
```
> 
# Description

Sets the MMIO cache policy of the target memory region.

# Parameters

- `policy`: The cache policy to assign.


*Internal callers (2):*
- **TruncatedMemoryRegion<T>::new_mmio** (L339): `region.0.set_cache_policy(cache_policy);`
- **TruncatedMemoryRegion<T>::from_memory_region** (L353): `truncated.0.set_cache_policy(policy);`

### `eq` (trait `PartialEq` for `MemoryRegion<T>`) [trait-pub] — **0 external callers**
```
fn eq(&self, other: &Self) -> bool
```

### `partial_cmp` (trait `PartialOrd` for `MemoryRegion<T>`) [trait-pub] — **0 external callers**
```
fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering>
```

### `cmp` (trait `Ord` for `MemoryRegion<T>`) [trait-pub] — **0 external callers**
```
fn cmp(&self, other: &Self) -> core::cmp::Ordering
```
*Internal callers (2):*
- **MemoryRegion<T>::partial_cmp** (L272): `Some(self.cmp(other))`
- **TruncatedMemoryRegion<T>::cmp** (L412): `self.0.cmp(&other.0)`

### `new` (impl `TruncatedMemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn new(
        name: &str,
        start: PageAligned<T>,
        size: usize,
        typ: MemoryRegionType,
        perm: AccessPermission,
    ) -> Result<Self, Error>
```
> Creates a new truncated memory region.

*Internal callers (3):*
- **TruncatedMemoryRegion<T>::new_mmio** (L338): `let mut region: Self = Self::new(name, start, size, MemoryRegionType::Mmio, perm`
- **TruncatedMemoryRegion<T>::from_memory_region** (L351): `let mut truncated: Self = Self::new(&name, start, size, typ, perm)?;`
- **TruncatedMemoryRegion<PhysicalAddress>::from_virtual_memory_region** (L445): `TruncatedMemoryRegion::new(&name, start, size, typ, perm)`

### `start` (impl `TruncatedMemoryRegion<T>`) [pub] — 4 external caller(s)
```
pub fn start(&self) -> PageAligned<T>
```
> Returns the first valid address that lies in the target memory region.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/io/mmio/allocator.rs` | 189 | `let start: usize = region.start().into_raw_value();` |
| `src/kernel/src/hal/io/mmio/region.rs` | 75 | `self.region.start()` |
| `src/kernel/src/mm/phys/frame.rs` | 569 | `allocated_frames: old(self)@.allocated_frames.union(frames),` |
| `src/kernel/src/mm/phys/frame.rs` | 589 | `let end_frame_number: usize = start_frame_number + region.size() / mem::FRAME_SI` |

*Internal callers (1):*
- **TruncatedMemoryRegion<T>::fmt** (L423): `self.start(),`

### `from_memory_region` (impl `TruncatedMemoryRegion<T>`) [pub] — 2 external caller(s)
```
pub fn from_memory_region(region: MemoryRegion<T>) -> Result<Self, Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/kernel_vas.rs` | 77 | `.push_back(TruncatedMemoryRegion::from_memory_region(region)?);` |
| `src/kernel/src/mm/kernel_vas.rs` | 80 | `.push_back(TruncatedMemoryRegion::from_memory_region(region)?);` |


### `name` (impl `TruncatedMemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn name(&self) -> String
```
*Internal callers (1):*
- **TruncatedMemoryRegion<T>::fmt** (L422): `self.name(),`

### `new_mmio` (impl `TruncatedMemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn new_mmio(
        name: &str,
        start: PageAligned<T>,
        size: usize,
        perm: AccessPermission,
        cache_policy: MmioCachePolicy,
    ) -> Result<Self, Error>
```
> 
# Description

Creates a new truncated MMIO memory region with an explicit cache policy.

# Parameters

- `name`: Name of the memory region.
- `start`: Page-aligned start address.
- `size`: Size of the region in bytes (rounded up to page alignment).
- `perm`: Access permissions for the region.
- `cache_policy`: Caching policy that controls PWT/PCD bits in page table entries.

# Returns

Upon successful completion, a new [`TruncatedMemoryRegion`] with type
[`MemoryRegionType::Mmio`] is returned. Upon failure, an error is returned instead.



### `size` (impl `TruncatedMemoryRegion<T>`) [pub] — 4 external caller(s)
```
pub fn size(&self) -> usize
```
> Returns the size of the target memory region.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/io/mmio/allocator.rs` | 190 | `let end: usize = compute_inclusive_end(start, region.size())?;` |
| `src/kernel/src/hal/io/mmio/region.rs` | 103 | `self.region.size()` |
| `src/kernel/src/mm/phys/frame.rs` | 570 | `free_frames: old(self)@.free_frames.difference(frames),` |
| `src/kernel/src/mm/phys/frame.rs` | 590 | `` |

*Internal callers (1):*
- **TruncatedMemoryRegion<T>::fmt** (L424): `self.size(),`

### `typ` (impl `TruncatedMemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn typ(&self) -> MemoryRegionType
```
> Returns the type of the target memory region.

*Internal callers (1):*
- **TruncatedMemoryRegion<T>::fmt** (L425): `self.typ(),`

### `perm` (impl `TruncatedMemoryRegion<T>`) [pub] — 1 external caller(s)
```
pub fn perm(&self) -> AccessPermission
```
> Returns the permissions of the target memory region.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/io/mmio/region.rs` | 89 | `self.region.perm()` |

*Internal callers (1):*
- **TruncatedMemoryRegion<T>::fmt** (L426): `self.perm(),`

### `cache_policy` (impl `TruncatedMemoryRegion<T>`) [pub] — **0 external callers**
```
pub fn cache_policy(&self) -> Option<MmioCachePolicy>
```
> 
# Description

Returns the MMIO cache policy of the target memory region, if set.

# Returns

The [`MmioCachePolicy`] if one was assigned, or `None` otherwise.


*Internal callers (1):*
- **TruncatedMemoryRegion<T>::fmt** (L427): `self.cache_policy()`

### `eq` (trait `PartialEq` for `TruncatedMemoryRegion<T>`) [trait-pub] — **0 external callers**
```
fn eq(&self, other: &Self) -> bool
```

### `partial_cmp` (trait `PartialOrd` for `TruncatedMemoryRegion<T>`) [trait-pub] — **0 external callers**
```
fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering>
```

### `cmp` (trait `Ord` for `TruncatedMemoryRegion<T>`) [trait-pub] — **0 external callers**
```
fn cmp(&self, other: &Self) -> core::cmp::Ordering
```
*Internal callers (1):*
- **TruncatedMemoryRegion<T>::partial_cmp** (L406): `Some(self.cmp(other))`

### `fmt` (trait `core::fmt::Debug` for `TruncatedMemoryRegion<T>`) [trait-pub] — **0 external callers**
```
fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result
```

### `from_virtual_memory_region` (impl `TruncatedMemoryRegion<PhysicalAddress>`) [pub] — 1 external caller(s)
```
pub fn from_virtual_memory_region(region: MemoryRegion<VirtualAddress>) -> Result<Self, Error>
```
> Attempts to create a virtual memory region from a physical memory region.


| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/kernel_vas.rs` | 69 | `match TruncatedMemoryRegion::from_virtual_memory_region(region.clone()) {` |


## Type References

### `MmioCachePolicy` [pub] — 3 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/boot_init.rs` | 26 | `MmioCachePolicy,` |
| `src/kernel/src/mm/virt/boot_init.rs` | 235 | `let cache_policy: MmioCachePolicy = region` |
| `src/kernel/src/mm/virt/boot_init.rs` | 237 | `.unwrap_or(MmioCachePolicy::UNCACHEABLE);` |

### `TruncatedMemoryRegion` [pub] — 42 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/mod.rs` | 46 | `TruncatedMemoryRegion,` |
| `src/kernel/src/mm/mod.rs` | 168 | `type VirtMemRegion = LinkedList<TruncatedMemoryRegion<VirtualAddress>>;` |
| `src/kernel/src/mm/mod.rs` | 169 | `type PhysMemRegion = LinkedList<TruncatedMemoryRegion<PhysicalAddress>>;` |
| `src/kernel/src/mm/kernel_vas.rs` | 19 | `TruncatedMemoryRegion,` |
| `src/kernel/src/mm/kernel_vas.rs` | 59 | `let mut virtual_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>` |
| `src/kernel/src/mm/kernel_vas.rs` | 61 | `let mut other_virtual_memory_regions: LinkedList<TruncatedMemoryRegion<VirtualAd` |
| `src/kernel/src/mm/kernel_vas.rs` | 63 | `let mut physical_memory_regions: LinkedList<TruncatedMemoryRegion<PhysicalAddres` |
| `src/kernel/src/mm/kernel_vas.rs` | 69 | `match TruncatedMemoryRegion::from_virtual_memory_region(region.clone()) {` |
| `src/kernel/src/mm/kernel_vas.rs` | 77 | `.push_back(TruncatedMemoryRegion::from_memory_region(region)?);` |
| `src/kernel/src/mm/kernel_vas.rs` | 80 | `.push_back(TruncatedMemoryRegion::from_memory_region(region)?);` |
| `src/kernel/src/mm/kernel_vas.rs` | 106 | `mmio_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>>,` |
| `src/kernel/src/mm/kernel_vas.rs` | 111 | `type VirtMemRegions = LinkedList<TruncatedMemoryRegion<VirtualAddress>>;` |
| `src/kernel/src/mm/kernel_vas.rs` | 112 | `type PhysMemRegions = LinkedList<TruncatedMemoryRegion<PhysicalAddress>>;` |
| `src/kernel/src/hal/io/mmio/allocator.rs` | 14 | `TruncatedMemoryRegion,` |
| `src/kernel/src/hal/io/mmio/allocator.rs` | 41 | `Rc<RefCell<VecDeque<(MmioTag, TruncatedMemoryRegion<VirtualAddress>)>>>;` |
| `src/kernel/src/hal/io/mmio/allocator.rs` | 57 | `region: TruncatedMemoryRegion<VirtualAddress>,` |
| `src/kernel/src/hal/io/mmio/allocator.rs` | 172 | `region: TruncatedMemoryRegion<VirtualAddress>,` |
| `src/kernel/src/hal/io/mmio/allocator.rs` | 239 | `let region: TruncatedMemoryRegion<VirtualAddress> = entry.region.clone();` |
| `src/kernel/src/hal/io/mmio/allocator.rs` | 282 | `VecDeque<(MmioTag, TruncatedMemoryRegion<VirtualAddress>)>,` |
| `src/kernel/src/mm/phys/frame.rs` | 22 | `TruncatedMemoryRegion,` |

### `MemoryRegion` [pub] — 26 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/kernel_vas.rs` | 14 | `MemoryRegion,` |
| `src/kernel/src/mm/kernel_vas.rs` | 51 | `memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,` |
| `src/kernel/src/mm/kernel_vas.rs` | 53 | `let mut memory_regions: LinkedList<MemoryRegion<VirtualAddress>> = {` |
| `src/kernel/src/mm/kernel_vas.rs` | 105 | `memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,` |
| `src/kernel/src/kimage.rs` | 10 | `MemoryRegion,` |
| `src/kernel/src/kimage.rs` | 33 | `text: MemoryRegion<VirtualAddress>,` |
| `src/kernel/src/kimage.rs` | 34 | `data: Option<MemoryRegion<VirtualAddress>>,` |
| `src/kernel/src/kimage.rs` | 35 | `rodata: MemoryRegion<VirtualAddress>,` |
| `src/kernel/src/kimage.rs` | 36 | `bss: MemoryRegion<VirtualAddress>,` |
| `src/kernel/src/kimage.rs` | 53 | `let text = MemoryRegion::new(` |
| `src/kernel/src/kimage.rs` | 69 | `let rodata = MemoryRegion::new(` |
| `src/kernel/src/kimage.rs` | 81 | `let data: Option<MemoryRegion<VirtualAddress>> = if data_size > 0 {` |
| `src/kernel/src/kimage.rs` | 86 | `Some(MemoryRegion::new(` |
| `src/kernel/src/kimage.rs` | 105 | `let bss = MemoryRegion::new(` |
| `src/kernel/src/kimage.rs` | 121 | `pub fn text(&self) -> MemoryRegion<VirtualAddress> {` |
| `src/kernel/src/kimage.rs` | 125 | `pub fn data(&self) -> Option<MemoryRegion<VirtualAddress>> {` |
| `src/kernel/src/kimage.rs` | 129 | `pub fn rodata(&self) -> MemoryRegion<VirtualAddress> {` |
| `src/kernel/src/kimage.rs` | 133 | `pub fn bss(&self) -> MemoryRegion<VirtualAddress> {` |
| `src/kernel/src/kmain.rs` | 37 | `MemoryRegion,` |
| `src/kernel/src/kmain.rs` | 353 | `LinkedList<MemoryRegion<VirtualAddress>>,` |

### `MemoryRegionType` [pub] — 14 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/kernel_vas.rs` | 15 | `MemoryRegionType,` |
| `src/kernel/src/mm/kernel_vas.rs` | 67 | `if region.typ() == MemoryRegionType::Reserved || region.typ() == MemoryRegionTyp` |
| `src/kernel/src/mm/kernel_vas.rs` | 67 | `if region.typ() == MemoryRegionType::Reserved || region.typ() == MemoryRegionTyp` |
| `src/kernel/src/mm/virt/boot_init.rs` | 25 | `MemoryRegionType,` |
| `src/kernel/src/mm/virt/boot_init.rs` | 96 | `MemoryRegionType::Mmio => {` |
| `src/kernel/src/mm/virt/boot_init.rs` | 180 | `if region.typ() != MemoryRegionType::Mmio {` |
| `src/kernel/src/kimage.rs` | 11 | `MemoryRegionType,` |
| `src/kernel/src/kimage.rs` | 57 | `MemoryRegionType::Reserved,` |
| `src/kernel/src/kimage.rs` | 73 | `MemoryRegionType::Reserved,` |
| `src/kernel/src/kimage.rs` | 90 | `MemoryRegionType::Reserved,` |
| `src/kernel/src/kimage.rs` | 109 | `MemoryRegionType::Reserved,` |
| `src/kernel/src/kmain.rs` | 38 | `MemoryRegionType,` |
| `src/kernel/src/kmain.rs` | 473 | `let typ: MemoryRegionType = MemoryRegionType::Reserved;` |
| `src/kernel/src/kmain.rs` | 473 | `let typ: MemoryRegionType = MemoryRegionType::Reserved;` |

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `new`
- `size`
- `typ`
- `name`
- `start`
- `perm`
- `cache_policy`
- `set_cache_policy`
- `eq`
- `partial_cmp`
- `cmp`
- `new`
- `name`
- `new_mmio`
- `typ`
- `cache_policy`
- `eq`
- `partial_cmp`
- `cmp`
- `fmt`

