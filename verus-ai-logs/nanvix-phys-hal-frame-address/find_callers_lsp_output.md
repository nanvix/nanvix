# Caller Analysis (LSP): frame.rs

- **Source file:** `/home/ruize/nanvix-phy/src/kernel/src/hal/mem/types/address/frame.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 9 |
| Public / trait-pub | 9 |
| Private | 0 |
| Types | 1 |

## Public API — External Callers

### `new` (impl `FrameAddress`) [pub] — 7 external caller(s)
```
pub fn new(address: PageAligned<PhysicalAddress>) -> Self
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs` | 194 | `Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(p` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 643 | `Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(p` |
| `src/kernel/src/mm/virt/vmem.rs` | 1654 | `FrameAddress::new(PageAligned::from_raw_value(vaddr.into_raw_value())?);` |
| `src/kernel/src/mm/virt/boot_init.rs` | 103 | `FrameAddress::new(page_aligned_phys_addr)` |
| `src/kernel/src/mm/virt/boot_init.rs` | 105 | `_ => FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value` |
| `src/kernel/src/mm/virt/boot_init.rs` | 230 | `paddr = FrameAddress::new(PageAligned::from_address(` |
| `src/kernel/src/mm/virt/boot_init.rs` | 256 | `FrameAddress::new(PageAligned::from_address(phys_addr)?)` |


### `into_frame_number` (impl `FrameAddress`) [pub] — 7 external caller(s)
```
pub fn into_frame_number(self) -> FrameNumber
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs` | 105 | `paddr.into_frame_number(),` |
| `src/kernel/src/mm/phys/frame.rs` | 291 | `let frame_number: usize = frame.into_frame_number().into_raw_value();` |
| `src/kernel/src/mm/phys/frame.rs` | 369 | `let frame_number: usize = frame.into_frame_number().into_raw_value();` |
| `src/kernel/src/mm/phys/frame.rs` | 429 | `let frame_number: usize = frame.into_frame_number().into_raw_value();` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 143 | `paddr.into_frame_number(),` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 494 | `let new_pte: PageTableEntry = PageTableEntry::new(new_flags, new_frame.into_fram` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 582 | `let base_frame: FrameNumber = base_address.into_frame_number();` |


### `into_page_address` (impl `FrameAddress`) [pub] — 1 external caller(s)
```
pub fn into_page_address(self) -> PageAddress
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/kpage.rs` | 59 | `.into_page_address()` |


### `into_physical_address` (impl `FrameAddress`) [pub] — 1 external caller(s)
```
pub fn into_physical_address(self) -> PageAligned<PhysicalAddress>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/vmem.rs` | 1418 | `let dst: PageAligned<PhysicalAddress> = uframe.into_physical_address();` |


### `from_frame_number` (impl `FrameAddress`) [pub] — 9 external caller(s)
```
pub fn from_frame_number(frame_number: FrameNumber) -> Result<Self, Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs` | 147 | `let paddr: FrameAddress = FrameAddress::from_frame_number(pde.frame_number())?;` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 188 | `let paddr: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 250 | `let paddr: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 489 | `let old_frame: FrameAddress = FrameAddress::from_frame_number(pte.frame_number()` |
| `src/kernel/src/mm/virt/manager.rs` | 293 | `let frame: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;` |
| `src/kernel/src/mm/virt/vmem.rs` | 553 | `let pgtab_addr: FrameAddress = FrameAddress::from_frame_number(pde.frame_number(` |
| `src/kernel/src/mm/virt/vmem.rs` | 866 | `let src_frame: FrameAddress = FrameAddress::from_frame_number(pte.frame_number()` |
| `src/kernel/src/mm/phys/frame.rs` | 158 | `match FrameAddress::from_frame_number(frame_number) {` |
| `src/kernel/src/mm/phys/frame.rs` | 232 | `match FrameAddress::from_frame_number(frame_number) {` |


### `into_raw_value` (impl `FrameAddress`) [pub] — 20 external caller(s)
```
pub fn into_raw_value(self) -> usize
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs` | 219 | `map(vaddr, paddr.into_raw_value(), true, true);` |
| `src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs` | 318 | `map_in(pml4_paddr, vaddr, paddr.into_raw_value(), true, true);` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 127 | `)]` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 132 | `///` |
| `src/kernel/src/mm/virt/vmem.rs` | 139 | `let pd_paddr_raw: usize = pgdir.physical_address()?.into_raw_value();` |
| `src/kernel/src/mm/virt/vmem.rs` | 201 | `PageDirectoryAddress::from_raw_value(pgdir.physical_address()?.into_raw_value())` |
| `src/kernel/src/mm/virt/vmem.rs` | 216 | `unsafe { mmu::load_page_directory(pgdir_addr.into_raw_value()) };` |
| `src/kernel/src/mm/virt/vmem.rs` | 882 | `let dst_paddr: usize = new_frame.address().into_raw_value();` |
| `src/kernel/src/mm/virt/vmem.rs` | 981 | `Ok(frame.into_raw_value() + offset)` |
| `src/kernel/src/mm/virt/vmem.rs` | 1064 | `(src_frame.into_raw_value() + offset) as *const u8,` |
| `src/kernel/src/mm/virt/vmem.rs` | 1221 | `let dst_phys_addr_raw: usize = dst_frame.into_raw_value() + offset;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1233 | `let dst: *mut u8 = (dst_frame.into_raw_value() + offset) as *mut u8;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1375 | `let src_phys_addr: usize = src_frame.into_raw_value() + src_offset;` |
| `src/kernel/src/mm/virt/vmem.rs` | 1376 | `let dst_phys_addr: usize = dst_frame.into_raw_value() + dst_offset;` |
| `src/kernel/src/mm/phys/kframe.rs` | 83 | `// for `base` (membership, alignment) onto the returned handle. On` |
| `src/kernel/src/mm/phys/kframe.rs` | 118 | `#[verus_spec(result =>` |
| `src/kernel/src/mm/phys/kframe.rs` | 137 | `// frame's raw address (`usize as *mut u8`) and writes through the identity-map` |
| `src/kernel/src/mm/phys/kframe.rs` | 147 | `}` |
| `src/kernel/src/pm/process/manager/mod.rs` | 245 | `let cr3: u32 = vmem.pgdir().physical_address()?.into_raw_value() as u32;` |
| `src/kernel/src/mm/phys/manager.rs` | 299 | `phys_view().initialized,` |

*Internal callers (1):*
- **FrameAddress::fmt** (L120): `write!(f, "FrameAddress({:#010x})", self.into_raw_value())`

### `from_raw_value` (impl `FrameAddress`) [pub] — 3 external caller(s)
```
pub fn from_raw_value(raw_addr: usize) -> Result<Self, Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/manager.rs` | 302 | `Ok(()) => spec_watermark_ok(phys_view().frames, count as int),` |
| `src/kernel/src/mm/phys/manager.rs` | 310 | `let reason: &str = "watermark + count overflow";` |
| `src/kernel/src/mm/virt/boot_init.rs` | 207 | `FrameAddress::from_raw_value(raw_vaddr)?,` |


### `fmt` (trait `core::fmt::Debug` for `FrameAddress`) [trait-pub] — **0 external callers**
```
fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result
```

### `eq` (trait `PartialEq` for `FrameAddress`) [trait-pub] — **0 external callers**
```
fn eq(&self, other: &Self) -> bool
```

## Type References

### `FrameAddress` [pub] — 93 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/frame.rs` | 19 | `FrameAddress,` |
| `src/kernel/src/mm/phys/frame.rs` | 137 | `fn alloc(&mut self) -> Result<FrameAddress, Error> {` |
| `src/kernel/src/mm/phys/frame.rs` | 158 | `match FrameAddress::from_frame_number(frame_number) {` |
| `src/kernel/src/mm/phys/frame.rs` | 210 | `fn alloc_contiguous(&mut self, count: usize) -> Result<FrameAddress, Error> {` |
| `src/kernel/src/mm/phys/frame.rs` | 232 | `match FrameAddress::from_frame_number(frame_number) {` |
| `src/kernel/src/mm/phys/frame.rs` | 290 | `fn free(&mut self, frame: FrameAddress) -> Result<(), Error> {` |
| `src/kernel/src/mm/phys/frame.rs` | 368 | `fn share(&mut self, frame: FrameAddress) -> Result<(), Error> {` |
| `src/kernel/src/mm/phys/frame.rs` | 428 | `fn refcount(&self, frame: FrameAddress) -> Result<u8, Error> {` |
| `src/kernel/src/mm/phys/frame.rs` | 731 | `#[verus_spec(result =>` |
| `src/kernel/src/mm/phys/frame.rs` | 743 | `// allocation moves one free frame into `allocated_frames` with refcount 1;` |
| `src/kernel/src/mm/phys/frame.rs` | 778 | `phys_view().inv(),` |
| `src/kernel/src/mm/phys/frame.rs` | 869 | `// `UserFrame::drop` / `KernelFrame::drop`, whose trait-fixed `drop(&mut self)`` |
| `src/kernel/src/mm/phys/frame.rs` | 894 | `no_unwind` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 16 | `FrameAddress,` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 126 | `result matches Ok(r) ==> r@ == addr@ && r.inv(),` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 131 | `` |
| `src/kernel/src/mm/virt/kpage.rs` | 73 | `pub fn frame_address(&self) -> crate::hal::mem::FrameAddress {` |
| `src/kernel/src/mm/virt/boot_init.rs` | 24 | `FrameAddress,` |
| `src/kernel/src/mm/virt/boot_init.rs` | 95 | `let mut paddr: FrameAddress = match region.typ() {` |
| `src/kernel/src/mm/virt/boot_init.rs` | 103 | `FrameAddress::new(page_aligned_phys_addr)` |

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `fmt`
- `eq`

