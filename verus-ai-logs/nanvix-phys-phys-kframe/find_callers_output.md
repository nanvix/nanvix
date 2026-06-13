# Caller Analysis (LSP): kframe.rs

- **Source file:** `/home/ruize/nanvix-phy/src/kernel/src/mm/phys/kframe.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 6 |
| Public / trait-pub | 6 |
| Private | 0 |
| Types | 1 |

## Implicit Callers (runtime/compiler)

### impl Deref for KernelFrame
- **Trait:** `Deref`
- **Description:** Compiler inserts deref() for * operator and auto-deref
- **Methods dispatched:** `deref`

### impl Drop for KernelFrame
- **Trait:** `Drop`
- **Description:** Compiler inserts call to drop() when value goes out of scope
- **Methods dispatched:** `drop`

### impl DerefMut for KernelFrame
- **Trait:** `DerefMut`
- **Description:** Compiler inserts deref_mut() for * operator on &mut
- **Methods dispatched:** `deref_mut`

> These functions have **no explicit call sites** in source code. The Rust runtime dispatches to them via vtable / lang items.

## Public API — External Callers

### `drop` (trait `Drop` for `KernelFrame`) [trait-pub] — implicit via `Drop`
```
fn drop(&mut self)
```
> ⚡ **impl Drop for KernelFrame**: Compiler inserts call to drop() when value goes out of scope


### `base` (impl `KernelFrame`) [pub] — 2 external caller(s)
```
pub fn base(&self) -> FrameAddress
```
> 
# Description

Returns the base address of the target kernel frame.

# Returns

The base address of the target kernel frame.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/kpage.rs` | 58 | `.base()` |
| `src/kernel/src/mm/virt/kpage.rs` | 74 | `self.kframe.base()` |


### `clear` (impl `KernelFrame`) [pub] — 3 external caller(s)
```
pub fn clear(&mut self) -> Result<(), Error>
```
> 
# Description

Clears the target kernel frame.

Uses the identity-map `memset` backend so that the write runs in the kernel address space.
This avoids a page fault when the current CR3 points to a user page directory that lacks
the PDE for this frame's physical address.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/manager.rs` | 268 | `kframe.clear()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 601 | `kframe.clear()?;` |
| `src/kernel/src/mm/virt/manager.rs` | 707 | `kframe.clear()?;` |


### `deref_mut` (trait `DerefMut` for `KernelFrame`) [trait-pub] — implicit via `DerefMut`
```
fn deref_mut(&mut self) -> &mut Self::Target
```
> ⚡ **impl DerefMut for KernelFrame**: Compiler inserts deref_mut() for * operator on &mut


### `deref` (trait `Deref` for `KernelFrame`) [trait-pub] — implicit via `Deref`
```
fn deref(&self) -> &Self::Target
```
> ⚡ **impl Deref for KernelFrame**: Compiler inserts deref() for * operator and auto-deref


### `new` (impl `KernelFrame`) [pub(super)] — 2 external caller(s)
```
pub(super) fn new(base: FrameAddress) -> Result<Self, Error>
```

| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/manager.rs` | 253 | `}` |
| `src/kernel/src/mm/phys/manager.rs` | 302 | `/// - `count`: Number of user frames the caller intends to allocate.` |


## Type References

### `KernelFrame` [pub] — 17 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/kpage.rs` | 10 | `mm::phys::KernelFrame,` |
| `src/kernel/src/mm/virt/kpage.rs` | 20 | `kframe: KernelFrame,` |
| `src/kernel/src/mm/virt/kpage.rs` | 41 | `pub fn new(kframe: KernelFrame) -> Self {` |
| `src/kernel/src/mm/kstack.rs` | 11 | `phys::KernelFrame,` |
| `src/kernel/src/mm/kstack.rs` | 94 | `let mut kframes: Vec<KernelFrame> = Vec::with_capacity(count);` |
| `src/kernel/src/mm/phys/manager.rs` | 18 | `upool::{` |
| `src/kernel/src/mm/phys/manager.rs` | 251 | `}` |
| `src/kernel/src/mm/phys/manager.rs` | 253 | `}` |
| `src/kernel/src/mm/phys/manager.rs` | 284 | `)]` |
| `src/kernel/src/mm/phys/manager.rs` | 302 | `/// - `count`: Number of user frames the caller intends to allocate.` |
| `src/kernel/src/mm/virt/manager.rs` | 26 | `KernelFrame,` |
| `src/kernel/src/mm/virt/manager.rs` | 216 | `let kframe: KernelFrame =` |
| `src/kernel/src/mm/virt/manager.rs` | 266 | `let mut kframe: KernelFrame =` |
| `src/kernel/src/mm/virt/manager.rs` | 599 | `let mut kframe: KernelFrame =` |
| `src/kernel/src/mm/virt/manager.rs` | 704 | `let mut kframe: KernelFrame =` |
| `src/kernel/src/mm/virt/manager.rs` | 733 | `kframes: &mut Vec<KernelFrame>,` |
| `src/kernel/src/mm/phys/mod.rs` | 50 | `kframe::KernelFrame,` |

