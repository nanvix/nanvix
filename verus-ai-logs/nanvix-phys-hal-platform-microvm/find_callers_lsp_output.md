# Caller Analysis (LSP): mod.rs

- **Source file:** `/home/ruize/nanvix-phy/src/kernel/src/hal/platform/microvm/mod.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP (intra-crate only)
- **Crate:** `kernel`
- **Depended on by:** *(none — no external callers possible)*

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 28 |
| Public / trait-pub | 21 |
| Private | 7 |
| Types | 3 |

## Public API — External Callers

### `setup_klog_backing_storage` [pub] — 1 external caller(s)
```
pub unsafe fn setup_klog_backing_storage() -> Result<(), ::sys::error::Error>
```
> 
# Description

Points the kernel log buffer at the BSS-resident `KLOG_BUFFER_STORAGE` buffer.

Must be called before the first logging macro invocation.

# Safety

This function accesses the `KLOG_BUFFER_STORAGE` static mutable.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/kmain.rs` | 328 | `if let Err(e) = unsafe { crate::hal::platform::setup_klog_backing_storage() } {` |


### `vmbus_read` [pub] — 1 external caller(s)
```
pub unsafe fn vmbus_read(addr: *mut u8)
```
> 
# Description

Places a read request to the platform's standard input device.

# Parameters

- `addr`: Address where data should be read into.

# Safety

This function is unsafe for multiple reasons:
- It assumes that the standard input device is present.
- It assumes that the standard input device was properly initialized.
- It does not prevent concurrent access to the standard input device.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/stdio.rs` | 117 | `platform::vmbus_read(&vmbus_msg as *const VmBusMessage as *mut u8);` |


### `virt_to_phys` [pub] — 4 external caller(s)
```
pub fn virt_to_phys(vaddr: usize) -> usize
```
> 
# Description

Translates a virtual address to a physical address.

# Returns

The physical address corresponding to the given virtual address.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs` | 193 | `let paddr: usize = crate::hal::platform::virt_to_phys(vaddr);` |
| `src/kernel/src/mm/virt/vmem.rs` | 1061 | `let dst_gpa: usize = crate::hal::platform::virt_to_phys(dst.into_raw_value());` |
| `src/kernel/src/mm/virt/vmem.rs` | 1234 | `let src_gpa: usize = crate::hal::platform::virt_to_phys(src.into_raw_value());` |
| `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs` | 642 | `let paddr: usize = crate::hal::platform::virt_to_phys(vaddr);` |


### `is_valid_physical_region` [pub] — 1 external caller(s)
```
pub fn is_valid_physical_region(start: usize, size: usize) -> bool
```
> 
# Description

Checks whether the given physical memory region lies entirely within physical memory on the
Microvm platform.

# Parameters

- `start`: Starting physical address of the region.
- `size`: Size of the region in bytes.

# Returns

`true` if the entire region lies within physical memory, `false` otherwise.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/virt/vmem.rs` | 475 | `crate::hal::platform::is_valid_physical_region(start, size)` |


### `putb` [pub] — 1 external caller(s)
```
pub unsafe fn putb(b: u8)
```
> 
# Description

Writes the 8-bit value `b` to the platform's standard output device.

# Parameters

- `b`: Value to write.

# Safety

This function is unsafe for multiple reasons:
- It assumes that the standard output device is present.
- It assumes that the standard output device was properly initialized.
- It does not prevent concurrent access to the standard output device.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/klog.rs` | 143 | `platform::putb(b);` |


### `snapshot` [pub] — 1 external caller(s)
```
pub fn snapshot()
```
> 
# Description

Requests the VMM to create a snapshot of the virtual machine state.
The snapshot command is issued via a port I/O write to the VMM control port.
The VMM will pause the vCPU, save VM state to disk, and resume execution.
On restore, execution resumes from the instruction following this call.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/kcall/dispatcher.rs` | 192 | `crate::hal::platform::snapshot();` |


### `signal_startup_complete` [pub] — 1 external caller(s)
```
pub fn signal_startup_complete()
```
> 
# Description

Signals the VMM that kernel startup is complete and user-space applications are about to start.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/kcall/handler.rs` | 70 | `crate::hal::platform::signal_startup_complete();` |


### `get_kstack_top` [pub] — **0 external callers**
```
pub fn get_kstack_top() -> *const u8
```
> 
# Description

Returns the boot kernel stack top pointer.

# Returns

A pointer to the top of the boot kernel stack.



### `wait_for_interrupt` [pub(super)] — 1 external caller(s)
```
pub(super) unsafe fn wait_for_interrupt()
```
> 
# Description

Waits for an interrupt to happen.

# Safety

This function is unsafe because it modifies the CPU state.

It is safe to call this function only when the CPU is able to receive interrupts.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/platform/mod.rs` | 90 | `wait_for_interrupt();` |


### `setup_heap_backing_storage` [pub] — 1 external caller(s)
```
pub unsafe fn setup_heap_backing_storage() -> Result<(), ::sys::error::Error>
```
> 
# Description

Points the kernel heap at the BSS-resident `HEAP_STORAGE` buffer.

Must be called before [`crate::mm::kheap::init()`].

# Safety

This function accesses the `HEAP_STORAGE` static mutable.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/kmain.rs` | 337 | `if let Err(e) = unsafe { crate::hal::platform::setup_heap_backing_storage() } {` |


### `enable_interrupts` [pub(super)] — 1 external caller(s)
```
pub(super) unsafe fn enable_interrupts()
```
> 
# Description

Enables all interrupts on the calling core.

# Safety

This function is unsafe because it modifies the CPU state.

It is safe to call this function only when the CPU is in a state where interrupts can be
enabled.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/platform/mod.rs` | 78 | `enable_interrupts();` |


### `parse_bootinfo` [pub] — 1 external caller(s)
```
pub fn parse_bootinfo(magic: u32, info: usize) -> Result<BootInfo, Error>
```
> 
# Description

Parses boot information.

# Parameters

- `magic`: Magic number.
- `info`:  Address of the boot information.

# Returns

A new boot information structure.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/kargs.rs` | 56 | `crate::hal::platform::parse_bootinfo(self.boot_magic, self.boot_info)` |


### `init` [pub] — 1 external caller(s)
```
pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    _memory_regions: &mut LinkedList<MemoryRegion<VirtualAddress>>,
    mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
    madt: &Option<MadtInfo>,
    _mem_lower: Option<usize>,
) -> Result<Platform, Error>
```
> 
# Description

Initializes the microvm platform.

# Parameters

- `ioports`: I/O port allocator.
- `ioaddresses`: I/O memory allocator.
- `_memory_regions`: Memory regions.
- `mmio_regions`: MMIO regions.
- `madt`: MADT information.
- `_mem_lower`: Lower memory size.

# Returns

Upon success, the initialized platform is returned. Upon failure, an error is returned instead.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mod.rs` | 141 | `let mut platform: Platform = platform::init(` |


### `vmbus_write` [pub] — 2 external caller(s)
```
pub unsafe fn vmbus_write(addr: *const u8)
```
> 
# Description

Places a write request to the platform's standard output device.

# Parameters

- `addr`: Address where data should be written from.

# Safety

This function is unsafe for multiple reasons:
- It assumes that the standard output device is present.
- It assumes that the standard output device was properly initialized.
- It does not prevent concurrent access to the standard output device.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/stdio.rs` | 69 | `platform::vmbus_write(&vmbus_msg as *const VmBusMessage as *const u8);` |
| `src/kernel/src/stdio.rs` | 197 | `platform::vmbus_write(&vmbus_msg as *const VmBusMessage as *const u8);` |


### `get_kstack_guard_base` [pub] — 1 external caller(s)
```
pub fn get_kstack_guard_base() -> usize
```
> 
# Description

Returns the base address of the boot kernel stack guard page.

# Returns

The base address of the boot kernel stack guard page.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/kstack.rs` | 281 | `let guard_base: usize = crate::hal::platform::get_kstack_guard_base();` |


### `is_valid_physical_address` [pub] — 1 external caller(s)
```
pub fn is_valid_physical_address(addr: VirtualAddress) -> bool
```
> 
# Description

Checks whether the given virtual address corresponds to a valid physical address on the Microvm
platform.

# Parameters

- `addr`: The virtual address to validate.

# Returns

`true` if `addr` falls within the physical address space, `false` otherwise.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mem/types/address/phys.rs` | 52 | `// Delegate to the per-platform validator to support sparse physical memory layo` |


### `max_physical_address` [pub] — 2 external caller(s)
```
pub fn max_physical_address() -> usize
```
> 
# Description

Returns the maximum physical address on the Microvm platform.

All physical memory is contiguous starting at GPA 0 up to `MEMORY_SIZE`.

# Returns

The maximum physical address value.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mem/types/address/phys.rs` | 229 | `///` |
| `src/kernel/src/mm/virt/boot_init.rs` | 76 | `let max_phys_addr: usize = crate::hal::platform::max_physical_address();` |


### `disable_interrupts` [pub(super)] — 1 external caller(s)
```
pub(super) unsafe fn disable_interrupts()
```
> 
# Description

Disables all interrupts on the calling core.

# Safety

This function is unsafe because it modifies the CPU state.

It is safe to call this function only when the CPU is in a state where interrupts can be
disabled.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/platform/mod.rs` | 98 | `disable_interrupts();` |


### `do_shutdown` [pub(in crate::hal::platform)] — 2 external caller(s)
```
pub(in crate::hal::platform) fn do_shutdown(status: usize) -> !
```
> 
# Description

Shuts down the machine.

# Parameters

- `status`: The shutdown status code.

# Returns

This function never returns.



| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/platform/mod.rs` | 24 | `use microvm::do_shutdown;` |
| `src/kernel/src/hal/platform/mod.rs` | 52 | `do_shutdown(status);` |


### `gva_to_gpa` [pub] — 1 external caller(s)
```
pub fn gva_to_gpa(gva: usize) -> usize
```
> 
# Description

Translates a guest virtual address to a guest physical address.

# Returns

The guest physical address corresponding to the given guest virtual address.




| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/mm/phys/mod.rs` | 84 | `) -> Result<(), Error> {` |


### `tsc_base_frequency_mhz` [pub] — **0 external callers**
```
pub fn tsc_base_frequency_mhz() -> u32
```
> 
# Description

Returns the TSC base frequency in MHz as provided by the VMM via a
microvm control register. Returns `0` when the VMM did not populate
the register.



## Private Functions — Internal Call Graph

These are implementation details. Listed to show which public functions depend on them.

### `register_pit` [private]
```
fn register_pit(ioports: &mut IoPortAllocator) -> Result<Pit, Error>
```
*Called by (1):*
- **init** (L986): `_pit: register_pit(ioports)?,`

### `read_ramfs_registers` [private]
```
fn read_ramfs_registers() -> Option<(usize, usize)>
```
> 
# Description

Reads the RAMFS base and size from MicroVM control registers.

# Safety

This function reads from memory-mapped control registers at addresses defined by
`DEFAULT_MICROVM_CTRL_BASE`. The caller must ensure that the MicroVM platform is
initialized and these addresses are valid and mapped.


*Called by (1):*
- **register_ramfs_mmio_region** (L750): `if let Some((ramfs_base, ramfs_size)) = read_ramfs_registers() {`

### `read_control_register` [private]
```
unsafe fn read_control_register(offset: usize) -> u32
```
> 
# Description

Reads a 32-bit value from a MicroVM control register.

# Parameters

- `offset`: Offset from `DEFAULT_MICROVM_CTRL_BASE` to read.

# Returns

The 32-bit value at the specified control register.

# Safety

The caller must ensure that `DEFAULT_MICROVM_CTRL_BASE + offset` points to a valid,
mapped memory-mapped I/O register. This function performs a volatile read.


*Called by (3):*
- **read_ramfs_registers** (L785): `read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_RAMFS_BASE) as usi`
- **read_ramfs_registers** (L787): `read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_RAMFS_SIZE) as usi`
- **tsc_base_frequency_mhz** (L832): `unsafe { read_control_register(::config::microvm::DEFAULT_MICROVM_CTRL_TSC_FREQ_`

### `register_pit_ports` [private]
```
fn register_pit_ports(ioports: &mut IoPortAllocator) -> Result<(), Error>
```
> Registers PIT calibration ports (channel 2 + speaker gate) so the interrupt controller can
allocate them during LAPIC timer calibration.

*Called by (1):*
- **init** (L949): `register_pit_ports(ioports)?;`

### `register_ramfs_mmio_region` [private]
```
fn register_ramfs_mmio_region(
    ioaddresses: &mut IoMemoryAllocator,
    mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<(), Error>
```
*Called by (1):*
- **init** (L944): `register_ramfs_mmio_region(ioaddresses, mmio_regions)?;`

### `log_control_registers` [private]
```
fn log_control_registers()
```
> 
# Description

Logs the values of the MicroVM control registers.


*Called by (1):*
- **init** (L943): `log_control_registers();`

### `register_pic_ioports` [private]
```
fn register_pic_ioports(ioports: &mut IoPortAllocator) -> Result<(), Error>
```
*Called by (1):*
- **init** (L898): `register_pic_ioports(ioports)?;`

## Type References

### `Platform` [pub] — 3 external reference(s)
| File | Line | Context |
|------|-----:|---------|
| `src/kernel/src/hal/mod.rs` | 32 | `Platform,` |
| `src/kernel/src/hal/mod.rs` | 88 | `_platform: Platform,` |
| `src/kernel/src/hal/mod.rs` | 141 | `let mut platform: Platform = platform::init(` |

### `KlogBufferStorage` [private] — 0 external reference(s)

### `HeapStorage` [private] — 0 external reference(s)

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `get_kstack_top`
- `tsc_base_frequency_mhz`

