# Caller Analysis (LSP): virt.rs

- **Source file:** `/home/ruize/nanvix-phy/src/libs/sys/src/sys/mm/address/virt.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP
- **Crate:** `sys`
- **Depended on by:** `echo-rust-nostd`, `nvx`, `elf`, `syslog`, `sysalloc`, `arch`, `syscall`, `mount-bench-nostd`, `noop-rust-nostd`, `snapshot-rust-nostd`, `vfs-bench-nostd`, `hostfsd`, `hostfs-api`, `linuxd`, `net-backend`, `memd`, `proc`, `networkd`, `procd`, `vfsd`, `kernel`, `bitmap`, `raw-array`, `nanvix-slab`, `nanvix`, `nanvix-sandbox`, `uservm`, `nvx-crt0`, `posix`, `arch-rust`, `c-bindings-rust`, `cmdline-len-rust`, `cmdline-env-rust-nostd`, `dlfcn-rust`, `env-rust-nostd`, `file-rust`, `linux-app`, `memory-rust`, `misc-rust`, `mount-test`, `mount-multipart-test`, `network-rust`, `snapshot-test`, `stress-rust`, `test-kernel`, `test-mmio-fault`, `testd`, `thread-rust`, `vfs-test`, `hello-rust-nostd`

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 23 |
| Public / trait-pub | 23 |
| Private | 0 |
| Types | 1 |

## Public API — External Callers

### `new` (impl `VirtualAddress`) [pub] — 8 external caller(s)
```
pub const fn new(value: usize) -> Self
```

| File | Line | Context |
|------|-----:|---------|
| `src/libs/sys/src/sys/config.rs` | 22 | `pub const KERNEL_BASE: VirtualAddress = VirtualAddress::new(KERNEL_BASE_RAW);` |
| `src/libs/sys/src/sys/config.rs` | 34 | `pub const KERNEL_END: VirtualAddress = VirtualAddress::new(KERNEL_END_RAW);` |
| `src/libs/sys/src/sys/config.rs` | 46 | `pub const USER_BASE: VirtualAddress = VirtualAddress::new(USER_BASE_RAW);` |
| `src/libs/sys/src/sys/config.rs` | 57 | `pub const USER_END: VirtualAddress = VirtualAddress::new(USER_END_RAW);` |
| `src/libs/sys/src/sys/config.rs` | 68 | `pub const USER_STACK_BASE: VirtualAddress = VirtualAddress::new(USER_STACK_BASE_` |
| `src/libs/sys/src/sys/config.rs` | 79 | `pub const USER_MMAP_BASE: VirtualAddress = VirtualAddress::new(USER_MMAP_BASE_RA` |
| `src/libs/sys/src/sys/config.rs` | 90 | `pub const USER_MMAP_END: VirtualAddress = VirtualAddress::new(USER_MMAP_END_RAW)` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 38 | `pub const NULL_USER_FN: VirtualAddress = VirtualAddress::new(0);` |

*Internal callers (5):*
- **VirtualAddress::from_raw_value** (L62): `VirtualAddress::new(raw_addr)`
- **VirtualAddress::align_up** (L81): `mm::align_up(self.0, align).map(VirtualAddress::new)`
- **VirtualAddress::align_down** (L99): `VirtualAddress::new(mm::align_down(self.0, align))`
- **VirtualAddress::add** (L270): `VirtualAddress::new(self.0 + rhs)`
- **VirtualAddress::from** (L282): `VirtualAddress::new(value as usize)`

### `checked_sub` (impl `VirtualAddress`) [pub] — **0 external callers**
```
pub fn checked_sub(&self, rhs: usize) -> Option<Self>
```
> 
# Description

Performs a checked subtraction of a [`VirtualAddress`] and a `usize`.

# Parameters

- `rhs`: The value to subtract.

# Returns

Upon success, the new [`VirtualAddress`] is returned. Upon failure (underflow), `None` is
returned instead.



### `align_down` (impl `VirtualAddress`) [pub] — **0 external callers**
```
pub fn align_down(&self, align: Alignment) -> Self
```
> 
# Description

Aligns the target [`VirtualAddress`] down to the provided `alignment`. If the address is
already aligned, it is returned as is.

# Parameters

- `alignment`: The alignment to align the target address to.

# Returns

Upon success, the aligned address is returned. Upon failure, an error is returned instead.


*Internal callers (1):*
- **VirtualAddress::align_down** (L209): `Ok(self.align_down(align))`

### `align_up` (impl `VirtualAddress`) [pub] — **0 external callers**
```
pub fn align_up(&self, align: Alignment) -> Option<Self>
```
> 
# Description

Aligns the target [`VirtualAddress`] to the provided `alignment`. If the address is already
aligned, it is returned as is.

# Parameters

- `alignment`: The alignment to align the target address to.

# Returns

Upon success, the aligned address is returned. Upon failure (overflow), `None` is returned
instead.


*Internal callers (1):*
- **VirtualAddress::align_up** (L190): `self.align_up(align)`

### `is_aligned` (impl `VirtualAddress`) [pub] — **0 external callers**
```
pub fn is_aligned(&self, align: Alignment) -> bool
```
> 
# Description

Checks if the target [`VirtualAddress`] is aligned to the provided `alignment`.

# Parameters

- `alignment`: The alignment to check.

# Returns

Upon success, `true` is returned if the address is aligned, otherwise `false`. Upon failure,
an error is returned instead.


*Internal callers (1):*
- **VirtualAddress::is_aligned** (L227): `Ok(self.is_aligned(align))`

### `from_raw_value` (impl `VirtualAddress`) [pub] — 5 external caller(s)
```
pub fn from_raw_value(raw_addr: usize) -> Self
```
> 
# Description

Instantiates a new [`VirtualAddress`] from a raw value.

# Parameters

- `raw_addr`: The raw value.



| File | Line | Context |
|------|-----:|---------|
| `src/libs/sys/src/sys/mm/mmio.rs` | 126 | `VirtualAddress::from_raw_value(self.base as usize)` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 44 | `user_fn: VirtualAddress::from_raw_value(0),` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 47 | `user_stack_base: VirtualAddress::from_raw_value(0),` |
| `src/libs/sys/src/sys/pm/sync.rs` | 30 | `addr: VirtualAddress::from_raw_value(raw_addr),` |
| `src/libs/sys/src/sys/pm/sync.rs` | 58 | `addr: VirtualAddress::from_raw_value(raw_addr),` |

*Internal callers (3):*
- **VirtualAddress::checked_add** (L135): `self.0.checked_add(rhs).map(VirtualAddress::from_raw_value)`
- **VirtualAddress::checked_sub** (L153): `self.0.checked_sub(rhs).map(VirtualAddress::from_raw_value)`
- **VirtualAddress::from_raw_value** (L172): `Ok(VirtualAddress::from_raw_value(raw_addr))`

### `checked_add` (impl `VirtualAddress`) [pub] — **0 external callers**
```
pub fn checked_add(&self, rhs: usize) -> Option<Self>
```
> 
# Description

Performs a checked addition of a [`VirtualAddress`] and a `usize`.

# Parameters

- `rhs`: The value to add.

# Returns

Upon success, the new [`VirtualAddress`] is returned. Upon failure (overflow), `None` is
returned instead.



### `add` (trait `::core::ops::Add<usize>` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn add(self, rhs: usize) -> Self::Output
```

### `add_assign` (trait `::core::ops::AddAssign<usize>` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn add_assign(&mut self, rhs: usize)
```

### `fmt` (trait `core::fmt::Debug` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
```

### `max_addr` (trait `Address` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn max_addr() -> usize
```
> 
# Description

Returns the maximum address for [`VirtualAddress`].

# Returns

The maximum [`VirtualAddress`].



### `is_aligned` (trait `Address` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn is_aligned(&self, align: Alignment) -> Result<bool, Error>
```
> 
# Description

Checks if the target [`VirtualAddress`] is aligned to the provided `alignment`.

# Parameters

- `alignment`: The alignment to check.

# Returns

Upon success, `true` is returned if the address is aligned, otherwise `false`. Upon failure,
an error is returned instead.



### `into_raw_value` (trait `Address` for `VirtualAddress`) [trait-pub] — 3 external caller(s)
```
fn into_raw_value(self) -> usize
```

| File | Line | Context |
|------|-----:|---------|
| `src/libs/sys/src/sys/mm/mmio.rs` | 67 | `let base_raw: u32 = u32::try_from(base.into_raw_value()).map_err(|_| {` |
| `src/libs/sys/src/sys/pm/sync.rs` | 37 | `addr.addr.into_raw_value()` |
| `src/libs/sys/src/sys/pm/sync.rs` | 65 | `addr.addr.into_raw_value()` |


### `clone_address` (trait `Address` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn clone_address(&self) -> Self
```

### `as_ptr` (trait `Address` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn as_ptr(&self) -> *const u8
```

### `as_mut_ptr` (trait `Address` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn as_mut_ptr(&self) -> *mut u8
```

### `from_raw_value` (trait `Address` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn from_raw_value(raw_addr: usize) -> Result<Self, Error>
```
> 
# Description

Instantiates a new [`VirtualAddress`] from a raw value.

# Parameters

- `raw_addr`: The raw value.

# Returns

- `Ok(Self)`: The new address.



### `align_down` (trait `Address` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn align_down(&self, align: Alignment) -> Result<Self, Error>
```
> 
# Description

Aligns the target [`VirtualAddress`] down to the provided `alignment`. If the address is
already aligned, it is returned as is.

# Parameters

- `alignment`: The alignment to align the target address to.

# Returns

Upon success, the aligned address is returned. Upon failure, an error is returned instead.



### `align_up` (trait `Address` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn align_up(&self, align: Alignment) -> Result<Self, Error>
```
> 
# Description

Aligns the target [`VirtualAddress`] to the provided `alignment`. If the address is already
aligned, it is returned as is.

# Parameters

- `alignment`: The alignment to align the target address to.

# Returns

Upon success, the aligned address is returned. Upon failure, an error is returned instead.



### `from` (trait `From<u32>` for `VirtualAddress`) [trait-pub] — **0 external callers**
```
fn from(value: u32) -> Self
```

### `from` (trait `From<VirtualAddress>` for `u64`) [trait-pub] — **0 external callers**
```
fn from(value: VirtualAddress) -> Self
```

### `from` (trait `From<VirtualAddress>` for `usize`) [trait-pub] — **0 external callers**
```
fn from(value: VirtualAddress) -> Self
```

### `from` (trait `From<VirtualAddress>` for `u32`) [trait-pub] — **0 external callers**
```
fn from(value: VirtualAddress) -> Self
```

## Type References

### `VirtualAddress` [pub] — 32 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/libs/sys/src/sys/mm/mmio.rs` | 16 | `VirtualAddress,` |
| `src/libs/sys/src/sys/mm/mmio.rs` | 63 | `base: VirtualAddress,` |
| `src/libs/sys/src/sys/mm/mmio.rs` | 125 | `pub fn base(&self) -> VirtualAddress {` |
| `src/libs/sys/src/sys/mm/mmio.rs` | 126 | `VirtualAddress::from_raw_value(self.base as usize)` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 8 | `use crate::mm::VirtualAddress;` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 18 | `pub user_fn: VirtualAddress,` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 27 | `pub user_stack_base: VirtualAddress,` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 33 | `pub user_tda: Option<VirtualAddress>,` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 38 | `pub const NULL_USER_FN: VirtualAddress = VirtualAddress::new(0);` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 38 | `pub const NULL_USER_FN: VirtualAddress = VirtualAddress::new(0);` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 44 | `user_fn: VirtualAddress::from_raw_value(0),` |
| `src/libs/sys/src/sys/pm/thread_create_args.rs` | 47 | `user_stack_base: VirtualAddress::from_raw_value(0),` |
| `src/libs/sys/src/sys/config.rs` | 9 | `use crate::mm::VirtualAddress;` |
| `src/libs/sys/src/sys/config.rs` | 22 | `pub const KERNEL_BASE: VirtualAddress = VirtualAddress::new(KERNEL_BASE_RAW);` |
| `src/libs/sys/src/sys/config.rs` | 22 | `pub const KERNEL_BASE: VirtualAddress = VirtualAddress::new(KERNEL_BASE_RAW);` |
| `src/libs/sys/src/sys/config.rs` | 34 | `pub const KERNEL_END: VirtualAddress = VirtualAddress::new(KERNEL_END_RAW);` |
| `src/libs/sys/src/sys/config.rs` | 34 | `pub const KERNEL_END: VirtualAddress = VirtualAddress::new(KERNEL_END_RAW);` |
| `src/libs/sys/src/sys/config.rs` | 46 | `pub const USER_BASE: VirtualAddress = VirtualAddress::new(USER_BASE_RAW);` |
| `src/libs/sys/src/sys/config.rs` | 46 | `pub const USER_BASE: VirtualAddress = VirtualAddress::new(USER_BASE_RAW);` |
| `src/libs/sys/src/sys/config.rs` | 57 | `pub const USER_END: VirtualAddress = VirtualAddress::new(USER_END_RAW);` |

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `checked_sub`
- `align_down`
- `align_up`
- `is_aligned`
- `checked_add`
- `add`
- `add_assign`
- `fmt`
- `max_addr`
- `is_aligned`
- `clone_address`
- `as_ptr`
- `as_mut_ptr`
- `from_raw_value`
- `align_down`
- `align_up`
- `from`
- `from`
- `from`
- `from`

