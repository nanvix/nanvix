# Verification Order

**Target:** `src/kernel/src/mm/phys/manager.rs`  
**Modules:** 14  
**Back-edges:** 21

**Entry functions:**
- `src/kernel/src/mm/phys/manager.rs:PhysMemoryManager::init`
- `src/kernel/src/mm/phys/manager.rs:PhysMemoryManager::get_mut`
- `src/kernel/src/mm/phys/manager.rs:PhysMemoryManager::alloc_many_user_frames`
- `src/kernel/src/mm/phys/manager.rs:PhysMemoryManager::alloc_kernel_frame`
- `src/kernel/src/mm/phys/manager.rs:PhysMemoryManager::alloc_many_kernel_frames`

---

### 1. `src/kernel/src/mm/phys/manager.rs`  ⟲ 3

**Depends on:** error, kernel::mm::phys::kpool, kernel::mm::phys::upool

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `PhysMemoryManager::init` | L84 | — | `error` |
| 2 | `PhysMemoryManager::get_mut` | L118 | — | — |
| 3 | `PhysMemoryManager::alloc_many_user_frames` | L146 | — | `error`, `kernel::mm::phys::upool` |
| 4 | `PhysMemoryManager::alloc_many_kernel_frames` | L203 | — | `error`, `kernel::mm::phys::kpool` |
| 5 | `PhysMemoryManager::alloc_kernel_frame` | L183 | — | `kernel::mm::phys::kpool` |

### 2. `src/kernel/src/mm/phys/upool.rs`  ⟲ 1

**Depends on:** kernel::mm::phys::frame

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `UserFrame::new` | L44 | — | — |
| 2 | `Upool::alloc` | L123 | `UserFrame::new` | `kernel::mm::phys::frame` |

### 3. `src/kernel/src/mm/phys/frame.rs`  ⟲ 4

**Depends on:** arch::x86::mem::paging::frame::number, error, kernel::hal::mem::types::address::frame, sparse-bitmap

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `Inner::alloc` | L96 | — | `arch::x86::mem::paging::frame::number`, `error`, `kernel::hal::mem::types::address::frame`, `sparse-bitmap` |
| 2 | `instance` | L326 | — | — |
| 3 | `alloc` | L365 | `Inner::alloc`, `instance` | — |

### 4. `src/kernel/src/mm/phys/kpool.rs`  ⟲ 5

**Depends on:** bitmap, error, kernel::hal::mem::types::address::aligned::page, kernel::hal::mem::types::address::frame, sys::sys::mm::address

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `instance` | L322 | — | — |
| 2 | `Inner::alloc_range` | L214 | — | `bitmap`, `error`, `kernel::hal::mem::types::address::aligned::page`, `kernel::hal::mem::types::address::frame`, `sys::sys::mm::address` |
| 3 | `Inner::alloc` | L147 | — | `bitmap`, `kernel::hal::mem::types::address::aligned::page`, `kernel::hal::mem::types::address::frame`, `sys::sys::mm::address` |
| 4 | `alloc_range` | L402 | `Inner::alloc_range`, `instance` | — |
| 5 | `KernelFrame::new` | L466 | — | — |
| 6 | `alloc` | L382 | `Inner::alloc`, `instance` | — |
| 7 | `Kpool::alloc_many` | L567 | `KernelFrame::new`, `alloc_range` | `error` |
| 8 | `Kpool::alloc` | L545 | `KernelFrame::new`, `alloc` | — |

### 5. `src/kernel/src/hal/mem/types/address/frame.rs`  ⟲ 2

**Depends on:** kernel::hal::mem::types::address::aligned::page, kernel::hal::mem::types::address::phys

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `FrameAddress` | L29 | — | — |
| 2 | `FrameAddress::new` | L65 | `FrameAddress` | — |
| 3 | `FrameAddress::from_frame_number` | L77 | `FrameAddress` | `kernel::hal::mem::types::address::aligned::page`, `kernel::hal::mem::types::address::phys` |

### 6. `src/libs/bitmap/src/lib.rs`  ⟲ 2

**Depends on:** error, raw-array

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `Bitmap::index_unchecked` | L706 | — | — |
| 2 | `Bitmap::alloc_range` | L310 | `Bitmap::index_unchecked` | `error`, `raw-array` |
| 3 | `Bitmap::alloc` | L250 | `Bitmap::alloc_range` | — |

### 7. `src/kernel/src/hal/mem/types/address/phys.rs`  ⟲ 2

**Depends on:** arch::x86::mem::paging::frame::number, sys::sys::mm::address::virt

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `PhysicalAddress` | L37 | — | — |
| 2 | `PhysicalAddress::from_number` | L95 | `PhysicalAddress` | `arch::x86::mem::paging::frame::number`, `sys::sys::mm::address::virt` |

### 8. `src/kernel/src/hal/mem/types/address/aligned/page.rs`  ⟲ 2

**Depends on:** error, sys::sys::mm::address

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `PageAligned` | L32 | — | — |
| 2 | `PageAligned::into_raw_value` | L51 | — | `sys::sys::mm::address` |
| 3 | `PageAligned::from_address` | L36 | `PageAligned` | `error`, `sys::sys::mm::address` |

### 9. `src/libs/raw-array/src/lib.rs`

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `RawArrayStorage::get_mut` | L167 | — | — |
| 2 | `RawArray::set` | L321 | `RawArrayStorage::get_mut` | — |

### 10. `src/libs/sparse-bitmap/src/lib.rs`

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `SparseBitmap::alloc` | L309 | — | — |

### 11. `src/libs/arch/src/x86/mem/paging/frame/number.rs`

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `FrameNumber` | L21 | — | — |
| 2 | `FrameNumber::into_raw_value` | L64 | — | — |
| 3 | `FrameNumber::from_raw_value` | L47 | `FrameNumber` | — |

### 12. `src/libs/sys/src/sys/mm/address/mod.rs`

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `is_aligned` | L98 | — | — |
| 2 | `into_raw_value` | L51 | — | — |
| 3 | `from_raw_value` | L49 | — | — |

### 13. `src/libs/error/src/lib.rs`

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `Error::new` | L460 | — | — |

### 14. `src/libs/sys/src/sys/mm/address/virt.rs`

| # | Function | Line | Calls (intra-module) | Deps (cross-module) |
|---|----------|------|----------------------|---------------------|
| 1 | `VirtualAddress` | L32 | — | — |
| 2 | `VirtualAddress::new` | L42 | `VirtualAddress` | — |
