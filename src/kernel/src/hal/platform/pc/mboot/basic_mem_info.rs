// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::platform::mboot::MbootTagType;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Multiboot Basic Mememory information Tag
//==================================================================================================

///
/// # Description
///
/// Multiboot Basic Memory information tag.
///
#[repr(C, align(8))]
pub struct MbootBasicMeminfoTag {
    /// Type.
    typ: MbootTagType,
    /// Size.
    size: u32,
}

// `MbootBasicMeminfoTag` must be 16 bytes long. This must match the multiboot specification.
sys::static_assert_size!(MbootBasicMeminfoTag, 8);

// `MbootBasicMeminfoTag` must be 8-byte aligned. This must match the multiboot specification.
sys::static_assert_alignment!(MbootBasicMeminfoTag, 8);

impl core::fmt::Debug for MbootBasicMeminfoTag {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "MbootBasicMeminfoTag: typ={:?}, size={}", self.typ, self.size)
    }
}

//==================================================================================================
// Multiboot Basic Memory information
//==================================================================================================

///
/// # Description
///
/// Multiboot Basic Memory information
///
/// tag: Mboot Header
///
/// mem_lower: Size of available memory below 1MB starting from 0x0.
///
/// mem_upper: Size of the first contiguos memory region above 1MB.
///
pub struct MbootBasicMeminfo<'a> {
    /// Tag.
    tag: &'a MbootBasicMeminfoTag,
    /// Lower memory size in kilobytes.
    mem_lower: &'a usize,
    /// Upper memory size in kilobytes.
    mem_upper: &'a usize,
}

impl MbootBasicMeminfo<'_> {
    pub unsafe fn from_raw(ptr: *const u8) -> Result<Self, Error> {
        // Ensure that `ptr` is not null.
        if ptr.is_null() {
            let reason: &str = "null pointer";
            error!("from_raw(): {:?}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Check if `ptr` is misaligned.
        if !ptr.is_aligned_to(core::mem::align_of::<MbootBasicMeminfo>()) {
            let reason: &str = "unaligned pointer";
            error!("from_raw(): {:?}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Cast pointer to multiboot basic memory information tag.
        let tag: &MbootBasicMeminfoTag = &*(ptr as *const MbootBasicMeminfoTag);

        // Check if pointer arithmetic wraps around the address space.
        if ptr.wrapping_add(tag.size as usize) < ptr {
            let reason: &str = "pointer arithmetic wraps around the address space";
            error!("from_raw(): {:?}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        let ptr: *const u8 = ptr.add(core::mem::size_of::<MbootBasicMeminfoTag>());

        // Cast pointer to lower memory address.
        let mem_lower: &usize = &*(ptr as *const usize);

        // Check if pointer arithmetic wraps around the address space.
        if ptr.wrapping_add(core::mem::size_of::<u32>()) < ptr {
            let reason: &str = "pointer arithmetic wraps around the address space";
            error!("from_raw(): {:?}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        let ptr: *const u8 = ptr.add(core::mem::size_of::<MbootBasicMeminfoTag>());

        // Cast pointer to upper memory address.
        let mem_upper: &usize = &*(ptr as *const usize);

        Ok(Self {
            tag,
            mem_lower,
            mem_upper,
        })
    }

    ///
    /// # Description
    ///
    /// Displays information about the target multiboot Basic Memory information.
    ///
    pub fn display(&self) {
        info!("{:?}", self.tag);
        info!("Available Lower memory={:?}", self.mem_lower);
        info!("Available Upper memory={:?}", self.mem_upper);
    }

    ///
    /// # Description
    ///
    /// Converts lower memory from kilobytes to bytes and return it.
    ///
    pub fn mem_lower_tobytes(&self) -> Result<usize, Error> {
        Ok(*self.mem_lower * 1024)
    }
}
