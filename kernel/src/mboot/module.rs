// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    ffi::CStr,
    mem,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Multiboot Module Tag
//==================================================================================================

///
/// # Description
///
/// Multiboot module tag
///
#[repr(C, align(8))]
pub struct MbootModuleTag {
    /// Type.
    typ: u32,
    /// Size.
    size: u32,
    /// Start.
    start: u32,
    /// End.
    end: u32,
}

// `MbootModuleTag` must be 16 bytes long. This must match the multiboot specification.
static_assert_size!(MbootModuleTag, 16);

// `MbootModuleTag` must be 8-byte aligned. This must match the multiboot specification.
static_assert_alignment!(MbootModuleTag, 8);

impl core::fmt::Debug for MbootModuleTag {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "mboot_module_tag: typ={:#010x}, size={}, start={:#010x}, end={:#010x}",
            self.typ, self.size, self.start, self.end
        )
    }
}

//==================================================================================================
// Multiboot Module
//==================================================================================================

///
/// # Description
///
/// Multiboot module.
///
pub struct MbootModule<'a> {
    /// Tag.
    tag: &'a MbootModuleTag,
    /// Command line.
    cmdline: &'a str,
}

impl<'a> MbootModule<'a> {
    ///
    /// # Description
    ///
    /// Constructs a  multiboot module from a raw pointer.
    ///
    /// # Parameters
    ///
    /// - `ptr`: Pointer to the multiboot module tag.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the multiboot module. Upon failure,
    /// an error is returned instead.
    ///
    /// # Safety
    ///
    /// The function is unsafe for the following reasons:
    ///  - The caller must ensure that `ptr` points to a valid multiboot module tag.
    ///
    pub unsafe fn from_ptr(ptr: *const u8) -> Result<Self, Error> {
        // Ensure that `ptr` is not null.
        if ptr.is_null() {
            return Err(Error::new(ErrorCode::BadAddress, "null pointer"));
        }

        // Check if `ptr` is misaligned.
        if !ptr.is_aligned_to(mem::align_of::<MbootModuleTag>()) {
            return Err(Error::new(ErrorCode::BadAddress, "unaligned pointer"));
        }

        // Cast pointer to multiboot module tag.
        let tag: &MbootModuleTag = &*(ptr as *const MbootModuleTag);

        // Check if pointer arithmetic wraps around the address space.
        if ptr.wrapping_add(mem::size_of::<MbootModuleTag>()) < ptr {
            error!("pointer arithmetic wrap around the address space");
            return Err(Error::new(ErrorCode::BadAddress, "pointer arithmetic wraps around"));
        }

        let ptr: *const u8 = ptr.add(mem::size_of::<MbootModuleTag>());

        // NOTE: no need to check if `ptr` is misaligned, as we will cast a string.

        // Cast pointer to command line.
        let cmdline: &str = {
            let cstr: &CStr = core::ffi::CStr::from_ptr(ptr as *const i8);
            match cstr.to_str() {
                Ok(s) => s,
                Err(_) => {
                    return Err(Error::new(ErrorCode::BadAddress, "invalid command line"));
                },
            }
        };

        Ok(Self { tag, cmdline })
    }

    ///
    /// # Description
    ///
    /// Gets the start address of the module.
    ///
    /// # Returns
    ///
    /// The start address of the module.
    pub fn start(&self) -> u32 {
        self.tag.start
    }

    ///
    /// # Description
    ///
    /// Gets the size of the module.
    ///
    /// # Returns
    ///
    /// The size of the module.
    ///
    pub fn size(&self) -> usize {
        (self.tag.end - self.tag.start) as usize
    }

    ///
    /// # Description
    ///
    /// Gets the command line of the module.
    ///
    /// # Returns
    ///
    /// The command line of the module.
    ///
    pub fn cmdline(&self) -> &str {
        self.cmdline
    }

    ///
    /// # Description
    ///
    /// Displays information about the target multiboot module.
    ///
    pub fn display(&self) {
        info!("{:?}", self.tag);
        info!("cmdline: {}", self.cmdline);
    }
}
