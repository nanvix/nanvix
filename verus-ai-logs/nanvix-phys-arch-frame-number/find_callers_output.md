# Caller Analysis (LSP): number.rs

- **Source file:** `/home/ruize/nanvix-phy/src/libs/arch/src/x86/mem/paging/frame/number.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP
- **Crate:** `arch`
- **Depended on by:** `sysalloc`, `syscall`, `mkramfs`, `vfsd`, `kernel`, `uservm`, `arch-rust`, `test-kernel`, `test-mmio-fault`, `testd`

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 4 |
| Public / trait-pub | 2 |
| Private | 2 |
| Types | 1 |

## Public API — External Callers

### `from_raw_value` (impl `FrameNumber`) [pub] — 8 external caller(s)
```
pub fn from_raw_value(value: usize) -> Option<Self>
```
> 
# Description

Constructs a [`FrameNumber`].

# Parameters

- `value`: The value of the frame number.

# Returns

- `Some(`[`FrameNumber`]`)`: Upon success.
- `None`: If the value is greater than [`Self::MAX`].



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 589 | `let frame: FrameNumber = FrameNumber::from_raw_value(raw_frame).ok_or_else(|| {` |
| `src/libs/arch/src/x86/mem/paging/pde.rs` | 303 | `frame: FrameNumber::from_raw_value(value as usize >> crate::mem::FRAME_SHIFT)?,` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 110 | `/// returned. Upon failure, an error is returned instead.` |
| `src/libs/arch/src/x86/mem/paging/pte.rs` | 304 | `frame: FrameNumber::from_raw_value(value as usize >> mem::FRAME_SHIFT)?,` |
| `src/kernel/src/mm/virt/identity_map.rs` | 530 | `FrameNumber::from_raw_value(pt_paddr / mem::PAGE_SIZE).ok_or_else(|| {` |
| `src/kernel/src/mm/virt/identity_map.rs` | 593 | `FrameNumber::from_raw_value(phys_addr / mem::PAGE_SIZE).ok_or_else(|| {` |
| `src/kernel/src/mm/phys/frame.rs` | 148 | `let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) ` |
| `src/kernel/src/mm/phys/frame.rs` | 223 | `let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) ` |

*Internal callers (2):*
- **test_frame_number_from_raw_value_zero** (L83): `let frame_number: FrameNumber = FrameNumber::from_raw_value(raw_value).unwrap();`
- **test_frame_number_from_raw_value_max** (L91): `let frame_number: FrameNumber = FrameNumber::from_raw_value(raw_value).unwrap();`

### `into_raw_value` (impl `FrameNumber`) [pub] — 12 external caller(s)
```
pub fn into_raw_value(self) -> usize
```
> 
# Description

Converts a [`FrameNumber`] into a raw value.

# Returns

The raw value of the target [`FrameNumber`].



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 584 | `let raw_frame: usize = base_frame.into_raw_value().checked_add(i).ok_or_else(|| ` |
| `src/libs/arch/src/x86/mem/paging/pde.rs` | 320 | `value |= (self.frame.into_raw_value() << crate::mem::FRAME_SHIFT) as PteWord;` |
| `src/libs/arch/src/x86/mem/paging/pde.rs` | 375 | `self.frame.into_raw_value() << crate::mem::FRAME_SHIFT` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 102 | `///` |
| `src/libs/arch/src/x86/mem/paging/pte.rs` | 321 | `value |= (self.frame.into_raw_value() << crate::mem::FRAME_SHIFT) as PteWord;` |
| `src/libs/arch/src/x86/mem/paging/pte.rs` | 362 | `self.frame.into_raw_value() << crate::mem::FRAME_SHIFT` |
| `src/kernel/src/mm/phys/frame.rs` | 291 | `let frame_number: usize = frame.into_frame_number().into_raw_value();` |
| `src/kernel/src/mm/phys/frame.rs` | 369 | `let frame_number: usize = frame.into_frame_number().into_raw_value();` |
| `src/kernel/src/mm/phys/frame.rs` | 429 | `let frame_number: usize = frame.into_frame_number().into_raw_value();` |
| `src/kernel/src/mm/phys/frame.rs` | 482 | `let frame_number: usize = phys_addr.into_frame_number().into_raw_value();` |
| `src/kernel/src/mm/phys/frame.rs` | 518 | `let frame_number: usize = phys_addr.into_frame_number().into_raw_value();` |
| `src/kernel/src/mm/phys/frame.rs` | 569 | `let start_frame_number: usize = region.start().into_frame_number().into_raw_valu` |

*Internal callers (2):*
- **test_frame_number_from_raw_value_zero** (L84): `assert_eq!(frame_number.into_raw_value(), raw_value);`
- **test_frame_number_from_raw_value_max** (L92): `assert_eq!(frame_number.into_raw_value(), raw_value);`

## Private Functions — Internal Call Graph

These are implementation details. Listed to show which public functions depend on them.

### `test_frame_number_from_raw_value_zero` [private]
```
fn test_frame_number_from_raw_value_zero()
```
> Tests if [`FrameNumber::from_raw_value()`] successfully constructs frame zero.

*No internal callers found (may be called via macro, closure, or conditional compilation).*

### `test_frame_number_from_raw_value_max` [private]
```
fn test_frame_number_from_raw_value_max()
```
> Tests if [`FrameNumber::from_raw_value()`] successfully constructs the maximum frame number.

*No internal callers found (may be called via macro, closure, or conditional compilation).*

## Type References

### `FrameNumber` [pub] — 36 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mem/types/address/frame.rs` | 20 | `use ::arch::mem::paging::FrameNumber;` |
| `src/kernel/src/hal/mem/types/address/frame.rs` | 83 | `/// # Description` |
| `src/kernel/src/hal/mem/types/address/frame.rs` | 87 | `/// # Returns` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 21 | `paging::FrameNumber,` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 101 | `/// Constructs a physical address from a memory-mapped I/O address.` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 106 | `///` |
| `src/kernel/src/hal/mem/types/address/phys.rs` | 110 | `/// returned. Upon failure, an error is returned instead.` |
| `src/kernel/src/mm/phys/frame.rs` | 28 | `paging::FrameNumber,` |
| `src/kernel/src/mm/phys/frame.rs` | 148 | `let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) ` |
| `src/kernel/src/mm/phys/frame.rs` | 148 | `let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) ` |
| `src/kernel/src/mm/phys/frame.rs` | 223 | `let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) ` |
| `src/kernel/src/mm/phys/frame.rs` | 223 | `let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) ` |
| `src/libs/arch/src/x86/mem/paging/frame/mod.rs` | 14 | `pub use number::FrameNumber;` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs` | 19 | `FrameNumber,` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs` | 161 | `FrameNumber::NULL,` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 20 | `FrameNumber,` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 201 | `FrameNumber::NULL,` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 582 | `let base_frame: FrameNumber = base_address.into_frame_number();` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 589 | `let frame: FrameNumber = FrameNumber::from_raw_value(raw_frame).ok_or_else(|| {` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 589 | `let frame: FrameNumber = FrameNumber::from_raw_value(raw_frame).ok_or_else(|| {` |

