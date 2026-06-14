# Caller Analysis (LSP): pde.rs

- **Source file:** `/home/ruize/nanvix-phy/src/libs/arch/src/x86/mem/paging/pde.rs`
- **Project dir:** `/home/ruize/nanvix-phy`
- **Parser:** rust-analyzer LSP
- **Crate:** `arch`
- **Depended on by:** `sysalloc`, `syscall`, `mkramfs`, `vfsd`, `kernel`, `uservm`, `arch-rust`, `test-kernel`, `test-mmio-fault`, `testd`

## Module Summary

| Category | Count |
|----------|------:|
| Total exec functions | 23 |
| Public / trait-pub | 21 |
| Private | 2 |
| Types | 2 |

## Public API — External Callers

### `new` (impl `PageDirectoryEntryFlags`) [pub] — **0 external callers**
```
pub fn new(
        present: PresentFlag,
        read_write: ReadWriteFlag,
        user_supervisor: UserSupervisorFlag,
        page_write_through: PageWriteThroughFlag,
        page_cache_disable: PageCacheDisableFlag,
        accessed: AccessedFlag,
        dirty: DirtyFlag,
        page_size: PageSizeFlag,
    ) -> Self
```
> 
# Description

Constructs a [`PageDirectoryEntryFlags`] with the given flags.

# Parameters

- `present`: The present flag.
- `read_write`: The read/write flag.
- `user_supervisor`: The user/supervisor flag.
- `page_write_through`: The page write-through flag.
- `page_cache_disable`: The page cache disable flag.
- `accessed`: The accessed flag.
- `dirty`: The dirty flag.
- `page_size`: The page size flag.

# Returns

A [`PageDirectoryEntryFlags`].



### `set_read_write` (impl `PageDirectoryEntryFlags`) [pub] — **0 external callers**
```
pub fn set_read_write(&mut self, read_write: ReadWriteFlag)
```
> 
# Description

Sets read/write flag.

# Parameters

- `read_write`: The read/write flag.


*Internal callers (1):*
- **PageDirectoryEntry::set_read_write** (L420): `self.flags.set_read_write(read_write);`

### `is_writable` (impl `PageDirectoryEntryFlags`) [pub] — **0 external callers**
```
pub fn is_writable(&self) -> bool
```
> 
# Description

Checks if the read/write flag is set (i.e., the page is writable).

# Returns

`true` if the page is writable, `false` otherwise.



### `is_present` (impl `PageDirectoryEntryFlags`) [pub] — **0 external callers**
```
pub fn is_present(&self) -> bool
```
> 
# Description

Checks if the present flag is set.

# Returns

`true` if the present flag is set, `false` otherwise.


*Internal callers (1):*
- **PageDirectoryEntry::is_present** (L355): `self.flags.is_present()`

### `is_user` (impl `PageDirectoryEntryFlags`) [pub] — **0 external callers**
```
pub fn is_user(&self) -> bool
```
> 
# Description

Checks if the user flag is set (i.e., user-mode access is allowed).

# Returns

`true` if the user flag is set, `false` otherwise.



### `set_user_supervisor` (impl `PageDirectoryEntryFlags`) [pub] — **0 external callers**
```
pub fn set_user_supervisor(&mut self, user_supervisor: UserSupervisorFlag)
```
> 
# Description

Sets user/supervisor flag.

# Parameters

- `user_supervisor`: The user/supervisor flag.


*Internal callers (1):*
- **PageDirectoryEntry::set_user_supervisor** (L433): `self.flags.set_user_supervisor(user_supervisor);`

### `is_large_page` (impl `PageDirectoryEntryFlags`) [pub] — **0 external callers**
```
pub fn is_large_page(&self) -> bool
```
> 
# Description

Checks if the page size flag is set (large page).

# Returns

`true` if the page size flag is set, `false` otherwise.


*Internal callers (1):*
- **PageDirectoryEntry::is_large_page** (L394): `self.flags.is_large_page()`

### `set_page_size` (impl `PageDirectoryEntryFlags`) [pub] — **0 external callers**
```
pub fn set_page_size(&mut self, page_size: PageSizeFlag)
```
> 
# Description

Sets page size.

# Parameters

- `page_size`: The page size flag.


*Internal callers (1):*
- **PageDirectoryEntry::set_page_size** (L407): `self.flags.set_page_size(page_size);`

### `from_raw` (trait `TableEntry` for `PageDirectoryEntry`) [trait-pub] — **0 external callers**
```
fn from_raw(raw: PteWord) -> Option<Self>
```

### `raw` (trait `TableEntry` for `PageDirectoryEntry`) [trait-pub] — **0 external callers**
```
fn raw(self) -> PteWord
```

### `into_raw_value` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn into_raw_value(self) -> PteWord
```
> 
# Description

Converts a [`PageDirectoryEntry`] into a raw 32-bit value.

# Returns

The raw value.


*Internal callers (1):*
- **PageDirectoryEntry::raw** (L443): `self.into_raw_value()`

### `from_raw_value` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn from_raw_value(value: PteWord) -> Option<Self>
```
> 
# Description

Constructs a [`PageDirectoryEntry`] from a raw 32-bit value.

# Parameters

- `value`: The raw value.

# Returns

- `Some(`[`PageDirectoryEntry`]`)`: If the raw value is valid.
- `None`: Otherwise.


*Internal callers (1):*
- **PageDirectoryEntry::from_raw** (L439): `Self::from_raw_value(raw)`

### `is_present` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn is_present(&self) -> bool
```
> 
# Description

Checks if the target page directory entry is marked as present.

# Returns

`true`: If the target page directory entry is marked as present.
`false`: Otherwise.



### `flags` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn flags(&self) -> PageDirectoryEntryFlags
```
> 
# Description

Returns the flags associated with the target page directory entry.

# Returns

The flags.



### `frame_number` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn frame_number(&self) -> FrameNumber
```
> 
# Description

Returns the frame number of the target page directory entry.

# Returns

The frame number.



### `frame_address` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn frame_address(&self) -> usize
```
> 
# Description

Returns the physical address (frame number × frame size) of the page frame.

# Returns

The physical address.



### `is_large_page` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn is_large_page(&self) -> bool
```
> 
# Description

Checks if the page size flag is set.

# Returns

`true` if the page size flag is set, `false` otherwise.



### `set_page_size` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn set_page_size(&mut self, page_size: PageSizeFlag)
```
> 
# Description

Sets page size.

# Parameters

- `page_size`: The page size flag.



### `set_read_write` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn set_read_write(&mut self, read_write: ReadWriteFlag)
```
> 
# Description

Sets read/write flag in the target page directory entry.

# Parameters

- `read_write`: The read/write flag.



### `set_user_supervisor` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn set_user_supervisor(&mut self, user_supervisor: UserSupervisorFlag)
```
> 
# Description

Sets user/supervisor flag in the target page directory entry.

# Parameters

- `user_supervisor`: The user/supervisor flag.



### `new` (impl `PageDirectoryEntry`) [pub] — **0 external callers**
```
pub fn new(flags: PageDirectoryEntryFlags, frame: FrameNumber) -> Self
```
> 
# Description

Constructs a [`PageDirectoryEntry`] with the given flags and frame number.

# Parameters

- `flags`: The flags.
- `frame`: The frame number.

# Returns

A [`PageDirectoryEntry`].



## Private Functions — Internal Call Graph

These are implementation details. Listed to show which public functions depend on them.

### `from_raw_value` (impl `PageDirectoryEntryFlags`) [private]
```
fn from_raw_value(value: PteWord) -> Self
```
> 
# Description

Constructs a [`PageDirectoryEntryFlags`] from a raw value.

# Parameters

- `value`: The raw value.

# Returns

A [`PageDirectoryEntryFlags`].


*Called by (11):*
- **PageDirectoryEntryFlags::from_raw_value** (L217): `present: PresentFlag::from_raw_value(value),`
- **PageDirectoryEntryFlags::from_raw_value** (L218): `read_write: ReadWriteFlag::from_raw_value(value),`
- **PageDirectoryEntryFlags::from_raw_value** (L219): `user_supervisor: UserSupervisorFlag::from_raw_value(value),`
- **PageDirectoryEntryFlags::from_raw_value** (L220): `page_write_through: PageWriteThroughFlag::from_raw_value(value),`
- **PageDirectoryEntryFlags::from_raw_value** (L221): `page_cache_disable: PageCacheDisableFlag::from_raw_value(value),`
- **PageDirectoryEntryFlags::from_raw_value** (L222): `accessed: AccessedFlag::from_raw_value(value),`
- **PageDirectoryEntryFlags::from_raw_value** (L223): `dirty: DirtyFlag::from_raw_value(value),`
- **PageDirectoryEntryFlags::from_raw_value** (L224): `page_size: PageSizeFlag::from_raw_value(value),`
- **PageDirectoryEntry::from_raw_value** (L308): `flags: PageDirectoryEntryFlags::from_raw_value(value),`
- **PageDirectoryEntry::from_raw_value** (L309): `frame: FrameNumber::from_raw_value(value as usize >> crate::mem::FRAME_SHIFT)?,`

### `into_raw_value` (impl `PageDirectoryEntryFlags`) [private]
```
fn into_raw_value(self) -> PteWord
```
> 
# Description

Converts a [`PageDirectoryEntryFlags`] into a raw value.

# Returns

The raw value.


*Called by (12):*
- **PageDirectoryEntryFlags::into_raw_value** (L240): `value |= self.present.into_raw_value();`
- **PageDirectoryEntryFlags::into_raw_value** (L241): `value |= self.read_write.into_raw_value();`
- **PageDirectoryEntryFlags::into_raw_value** (L242): `value |= self.user_supervisor.into_raw_value();`
- **PageDirectoryEntryFlags::into_raw_value** (L243): `value |= self.page_write_through.into_raw_value();`
- **PageDirectoryEntryFlags::into_raw_value** (L244): `value |= self.page_cache_disable.into_raw_value();`
- **PageDirectoryEntryFlags::into_raw_value** (L245): `value |= self.accessed.into_raw_value();`
- **PageDirectoryEntryFlags::into_raw_value** (L246): `value |= self.dirty.into_raw_value();`
- **PageDirectoryEntryFlags::into_raw_value** (L247): `value |= self.page_size.into_raw_value();`
- **PageDirectoryEntry::into_raw_value** (L325): `value |= self.flags.into_raw_value();`
- **PageDirectoryEntry::into_raw_value** (L326): `value |= (self.frame.into_raw_value() << crate::mem::FRAME_SHIFT) as PteWord;`

## Type References

### `PageDirectoryEntryFlags` [pub] — 0 external reference(s)

### `PageDirectoryEntry` [pub] — 0 external reference(s)

## ⚠️ Public Functions with No External Callers

These are public but have no call sites outside the module. They may be dead code or intended for future use.

- `new`
- `set_read_write`
- `is_writable`
- `is_present`
- `is_user`
- `set_user_supervisor`
- `is_large_page`
- `set_page_size`
- `from_raw`
- `raw`
- `into_raw_value`
- `from_raw_value`
- `is_present`
- `flags`
- `frame_number`
- `frame_address`
- `is_large_page`
- `set_page_size`
- `set_read_write`
- `set_user_supervisor`
- `new`

