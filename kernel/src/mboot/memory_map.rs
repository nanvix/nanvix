// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::MemoryRegionType;
use ::core::mem;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Multiboot Memory Type
//==================================================================================================

///
/// # Description
///
/// Multiboot memory type.
///
pub enum MbootMemoryType {
    /// Available.
    Available = 1,
    /// Reserved.
    Reserved = 2,
    /// ACPI reclaimable.
    AcpiReclaimable = 3,
    /// NVS.
    Nvs = 4,
    /// Bad RAM.
    BadRam = 5,
}

impl core::fmt::Debug for MbootMemoryType {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            MbootMemoryType::Available => write!(f, "available"),
            MbootMemoryType::Reserved => write!(f, "reserved"),
            MbootMemoryType::AcpiReclaimable => write!(f, "acpi reclaimable"),
            MbootMemoryType::Nvs => write!(f, "nvs"),
            MbootMemoryType::BadRam => write!(f, "bad ram"),
        }
    }
}

impl From<u32> for MbootMemoryType {
    fn from(value: u32) -> Self {
        match value {
            1 => MbootMemoryType::Available,
            2 => MbootMemoryType::Reserved,
            3 => MbootMemoryType::AcpiReclaimable,
            4 => MbootMemoryType::Nvs,
            5 => MbootMemoryType::BadRam,
            _ => unreachable!("invalid mboot memory type {}", value),
        }
    }
}

impl From<MbootMemoryType> for MemoryRegionType {
    fn from(val: MbootMemoryType) -> Self {
        match val {
            MbootMemoryType::Available => MemoryRegionType::Usable,
            MbootMemoryType::Reserved => MemoryRegionType::Reserved,
            MbootMemoryType::AcpiReclaimable => MemoryRegionType::Reserved,
            MbootMemoryType::Nvs => MemoryRegionType::Reserved,
            MbootMemoryType::BadRam => MemoryRegionType::Bad,
        }
    }
}

//==================================================================================================
// Multiboot Memory Map Entry
//==================================================================================================

///
/// # Description
///
/// Multiboot memory map entry.
///
#[repr(C, align(8))]
pub struct MbootMemoryMapEntry {
    /// Address.
    addr: u64,
    /// Length.
    len: u64,
    /// Type.
    typ: u32,
    /// Reserved.
    reserved: u32,
}

// `MbootMemoryMapEntry` must be 24 bytes long. This must match the multiboot specification.
static_assert_size!(MbootMemoryMapEntry, 24);

// `MbootMemoryMapEntry` must be 8-byte aligned. This must match the multiboot specification.
static_assert_alignment!(MbootMemoryMapEntry, 8);

impl MbootMemoryMapEntry {
    ///
    /// # Description
    ///
    /// Returns the address of the memory map entry.
    ///
    pub fn addr(&self) -> u64 {
        self.addr
    }

    ///
    /// # Description
    ///
    /// Returns the length of the memory map entry.
    ///
    pub fn len(&self) -> u64 {
        self.len
    }

    ///
    /// # Description
    ///
    /// Returns the type of the memory map entry.
    ///
    pub fn typ(&self) -> MbootMemoryType {
        Into::<MbootMemoryType>::into(self.typ)
    }
}

impl core::fmt::Debug for MbootMemoryMapEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "mboot_memory_map_entry: addr={:#018x}, len={:#018x}, typ={:?}",
            self.addr,
            self.len,
            Into::<MbootMemoryType>::into(self.typ)
        )
    }
}

//==================================================================================================
// Multiboot Memory Map Tag
//==================================================================================================

///
/// # Description
///
/// Multiboot memory map tag.
///
#[repr(C, align(8))]
struct MbootMemoryMapTag {
    /// Type.
    typ: u32,
    /// Total size including the header.
    size: u32,
    /// Size of each entry.
    entry_size: u32,
    /// Version of the entry format.
    entry_version: u32,
}

// `MbootMemoryMapTag` must be 16 bytes long. This must match the multiboot specification.
static_assert_size!(MbootMemoryMapTag, 16);

// `MbootMemoryMapTag` must be 8-byte aligned. This must match the multiboot specification.
static_assert_alignment!(MbootMemoryMapTag, 8);

impl core::fmt::Debug for MbootMemoryMapTag {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "mboot_memory_map_tag: typ={:?}, size={}, entry_size={}, entry_version={}",
            self.typ, self.size, self.entry_size, self.entry_version
        )
    }
}

//==================================================================================================
// Multiboot Memory Map
//==================================================================================================

///
/// # Description
///
/// Multiboot memory map.
///
pub struct MbootMemoryMap<'a> {
    tag: &'a MbootMemoryMapTag,
    entries: &'a [MbootMemoryMapEntry],
}

impl<'a> MbootMemoryMap<'a> {
    ///
    /// # Description
    ///
    /// Constructs a multiboot memory map from a raw pointer.
    ///
    /// # Parameters
    ///
    /// - `ptr`: Pointer to the multiboot memory map.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the multiboot memory map. Otherwise, it returns an error.
    ///
    /// # Safety
    ///
    /// This function is unsafe for the following reasons:
    /// - The caller must ensure that `ptr` points to a valid multiboot memory map.
    ///
    pub unsafe fn from_ptr(ptr: *const u8) -> Result<Self, Error> {
        // Ensure that `ptr` is not null.
        if ptr.is_null() {
            return Err(Error::new(ErrorCode::InvalidArgument, "null pointer"));
        }

        // Check if `ptr` is misaligned.
        if !ptr.is_aligned_to(mem::align_of::<MbootMemoryMapTag>()) {
            return Err(Error::new(ErrorCode::BadAddress, "unaligned pointer"));
        }

        // Cast pointer to memory map tag.
        let tag: &MbootMemoryMapTag = &*(ptr as *const MbootMemoryMapTag);

        // Cast memory map entries.
        let entries: &[MbootMemoryMapEntry] = {
            // Check if pointer arithmetic wraps around the address space.
            if ptr.wrapping_add(mem::size_of::<MbootMemoryMapTag>()) < ptr {
                return Err(Error::new(ErrorCode::BadAddress, "pointer arithmetic wraps around"));
            }

            let ptr: *const u8 = ptr.add(mem::size_of::<MbootMemoryMapTag>());

            // Check if `ptr` is misaligned.
            if !ptr.is_aligned_to(mem::align_of::<MbootMemoryMapEntry>()) {
                return Err(Error::new(ErrorCode::BadAddress, "unaligned pointer"));
            }

            // Check if `tag.size` is not big enough.
            if tag.size < mem::size_of::<MbootMemoryMapTag>() as u32 {
                return Err(Error::new(ErrorCode::BadAddress, "invalid tag size"));
            }

            let entries_size: usize = tag.size as usize - mem::size_of::<MbootMemoryMapTag>();

            // Check if `entries_size` is not a multiple of `tag.entry_size`.
            if entries_size % tag.entry_size as usize != 0 {
                return Err(Error::new(ErrorCode::BadAddress, "invalid size of entries"));
            }

            let len: usize = entries_size / tag.entry_size as usize;
            core::slice::from_raw_parts(ptr as *const MbootMemoryMapEntry, len)
        };

        Ok(Self { tag, entries })
    }

    ///
    /// # Description
    ///
    /// Displays the target multiboot memory map.
    ///
    pub fn display(&self) {
        info!("{:?}", self.tag);
        for entry in self.entries {
            info!("{:?}", entry);
        }
    }
}

impl<'a> Iterator for MbootMemoryMap<'a> {
    type Item = &'a MbootMemoryMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.entries.is_empty() {
            return None;
        }

        let entry = &self.entries[0];
        self.entries = &self.entries[1..];

        Some(entry)
    }
}
