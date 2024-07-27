// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Multiboot ACPI Tag
//==================================================================================================

///
/// # Description
///
/// Multiboot ACPI tag.
///
#[repr(C, align(8))]
pub struct MbootAcpiTag {
    /// Type.
    typ: u32,
    /// Size.
    size: u32,
}

// `MbootAcpiTag` must be 16 bytes long. This must match the multiboot specification.
static_assert_size!(MbootAcpiTag, 8);

// `MbootAcpiTag` must be 8-byte aligned. This must match the multiboot specification.
static_assert_alignment!(MbootAcpiTag, 8);

impl core::fmt::Debug for MbootAcpiTag {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "mboot_acpi_tag: typ={:?}, size={}", self.typ, self.size)
    }
}

//==================================================================================================
// Multiboot ACPI
//==================================================================================================

///
/// # Description
///
/// Multiboot ACPI.
///
pub struct MbootAcpi<'a> {
    /// Tag.
    tag: &'a MbootAcpiTag,
    /// Pointer to ACPI table.
    rsdp: *const u8,
}

impl<'a> MbootAcpi<'a> {
    ///
    /// # Description
    ///
    /// Constructs a multiboot ACPI from a raw pointer.
    ///
    /// # Parameters
    ///
    /// - `ptr`: Pointer to the multiboot ACPI.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the multiboot ACPI. Otherwise, it returns an error.
    ///
    /// # Safety
    ///
    /// This function is unsafe for the following reasons:
    /// - The caller must ensure that `ptr` points to a is a valid a multiboot ACPI tag.
    ///
    pub unsafe fn from_raw(ptr: *const u8) -> Result<Self, Error> {
        // Ensure that `ptr` is not null.
        if ptr.is_null() {
            return Err(Error::new(ErrorCode::BadAddress, "null pointer"));
        }

        // Check if `ptr` is misaligned.
        if !ptr.is_aligned_to(core::mem::align_of::<MbootAcpiTag>()) {
            return Err(Error::new(ErrorCode::BadAddress, "unaligned pointer"));
        }

        // Cast pointer to multiboot ACPI tag.
        let tag: &MbootAcpiTag = &*(ptr as *const MbootAcpiTag);

        // Check if pointer arithmetic wraps around the address space.
        if ptr.wrapping_add(tag.size as usize) < ptr {
            return Err(Error::new(
                ErrorCode::BadAddress,
                "pointer arithmetic wraps around the address space",
            ));
        }

        let ptr: *const u8 = ptr.add(core::mem::size_of::<MbootAcpiTag>());

        // NOTE: no need to check if `ptr` is misaligned, as this is the start of the ACPI table.

        Ok(Self { tag, rsdp: ptr })
    }

    ///
    /// # Description
    ///
    /// Displays information about the target multiboot ACPI.
    ///
    pub fn display(&self) {
        info!("{:?}", self.tag);
        info!("rsdp={:?}", self.rsdp);
    }

    ///
    /// # Description
    ///
    /// Returns the pointer to the ACPI table.
    ///
    /// # Return Values
    ///
    /// Returns the pointer to the ACPI table.
    ///
    pub fn rsdp(&self) -> *const u8 {
        self.rsdp
    }
}
