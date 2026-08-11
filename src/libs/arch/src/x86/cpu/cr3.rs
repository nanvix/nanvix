// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

// Error
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
compile_error!("Unsupported architecture");

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch::asm;

include!("cr3.spec.rs");

//==================================================================================================
// Page-Level Write-Through Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the page-level write-through flag in the CR3 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLevelWriteThroughFlag {
    /// Page-level write-through is disabled (write-back caching).
    Disabled = 0,
    /// Page-level write-through is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl PageLevelWriteThroughFlag {
    /// Bit shift of the page-level write-through flag.
    const SHIFT: u32 = 3;
    /// Bit mask of the page-level write-through flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a page-level write-through flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The page-level write-through flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => PageLevelWriteThroughFlag::Disabled,
            _ => PageLevelWriteThroughFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the page-level write-through flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the page-level write-through flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Page-Level Cache Disable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the page-level cache disable flag in the CR3 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLevelCacheDisableFlag {
    /// Page-level caching is enabled.
    Enabled = 0,
    /// Page-level caching is disabled.
    Disabled = (1 << Self::SHIFT),
}

impl PageLevelCacheDisableFlag {
    /// Bit shift of the page-level cache disable flag.
    const SHIFT: u32 = 4;
    /// Bit mask of the page-level cache disable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a page-level cache disable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The page-level cache disable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => PageLevelCacheDisableFlag::Enabled,
            _ => PageLevelCacheDisableFlag::Disabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the page-level cache disable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the page-level cache disable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Paging Structure Base Address
//==================================================================================================

///
/// # Description
///
/// A type that represents the base address of the paging structure referenced by the CR3
/// register.
/// This is a 4 KB-aligned physical address encoded in CR3.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagingStructureBaseAddress(u32);

/// Backward-compatible alias for the legacy 32-bit naming.
pub type PageDirectoryBaseAddress = PagingStructureBaseAddress;

impl PagingStructureBaseAddress {
    /// Bit shift of the paging structure base address.
    const SHIFT: u32 = 12;
    /// Bit mask of the paging structure base address.
    const MASK: u32 = !((1 << Self::SHIFT) - 1);

    ///
    /// # Description
    ///
    /// Attempts to create a paging structure base address from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the address.
    ///
    /// # Return Value
    ///
    /// The paging structure base address extracted from the value, or `None` if the lower 12 bits
    /// are not zero.
    ///
    fn try_from_u32(value: u32) -> Option<Self> {
        if value & !Self::MASK != 0 {
            return None;
        }
        Some(Self(value))
    }

    ///
    /// # Description
    ///
    /// Creates a paging structure base address from a raw 32-bit value without validation.
    /// The lower 12 bits are masked out.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the address.
    ///
    /// # Return Value
    ///
    /// The paging structure base address extracted from the value.
    ///
    fn from_u32_unchecked(value: u32) -> Self {
        Self(value & Self::MASK)
    }

    ///
    /// # Description
    ///
    /// Creates a paging structure base address from a physical address.
    ///
    /// # Parameters
    ///
    /// - `address`: The physical address of the paging structure. Must be 4 KB aligned.
    ///
    /// # Return Value
    ///
    /// The paging structure base address, or `None` if the address is not 4 KB aligned.
    ///
    pub fn new(address: u32) -> Option<Self> {
        if address & !Self::MASK != 0 {
            return None;
        }
        Some(Self(address))
    }

    ///
    /// # Description
    ///
    /// Returns the physical address of the paging structure.
    ///
    /// # Return Value
    ///
    /// The physical address stored in the paging structure base address field.
    ///
    pub fn address(self) -> u32 {
        self.0
    }

    ///
    /// # Description
    ///
    /// Converts the paging structure base address to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the paging structure base address.
    ///
    fn into_u32(self) -> u32 {
        self.0
    }
}

//==================================================================================================
// Control Register Three (CR3)
//==================================================================================================

///
/// # Description
///
/// A type that represents the CR3 register. The CR3 register holds the physical address
/// of the current paging structure and two flags that control page-level caching.
///
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cr3Register {
    /// Page-level write-through flag.
    pub page_level_write_through: PageLevelWriteThroughFlag,
    /// Page-level cache disable flag.
    pub page_level_cache_disable: PageLevelCacheDisableFlag,
    /// Paging structure base address.
    pub paging_structure_base_address: PagingStructureBaseAddress,
}

impl Cr3Register {
    /// Mask of all reserved bits in the CR3 register.
    const RESERVED_MASK: u32 = !(PageLevelWriteThroughFlag::MASK
        | PageLevelCacheDisableFlag::MASK
        | PagingStructureBaseAddress::MASK);

    ///
    /// # Description
    ///
    /// Attempts to create a CR3 register from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the register state.
    ///
    /// # Return Value
    ///
    /// The CR3 register with all fields extracted from the value, or `None` if the
    /// value contains non-zero reserved bits (bits 0-2 or 5-11).
    ///
    pub fn try_from_u32(value: u32) -> Option<Self> {
        if value & Self::RESERVED_MASK != 0 {
            return None;
        }

        let cr3: Self = Self {
            page_level_write_through: PageLevelWriteThroughFlag::from_u32(value),
            page_level_cache_disable: PageLevelCacheDisableFlag::from_u32(value),
            paging_structure_base_address: PagingStructureBaseAddress::try_from_u32(
                value & PagingStructureBaseAddress::MASK,
            )?,
        };

        Some(cr3)
    }

    ///
    /// # Description
    ///
    /// Creates a CR3 register from a raw 32-bit value without validation.
    /// Reserved bits are ignored; the base address field is masked.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the register state.
    ///
    /// # Return Value
    ///
    /// The CR3 register with all fields extracted from the value.
    ///
    /// # Safety
    ///
    /// It is unsafe to call this function because no validation is performed.
    ///
    /// It is safe to call this function if the following conditions are met:
    /// - The reserved bits (0-2 and 5-11) in `value` are zero or have been masked out by the
    ///   caller.
    ///
    pub unsafe fn from_u32_unchecked(value: u32) -> Self {
        Self {
            page_level_write_through: PageLevelWriteThroughFlag::from_u32(value),
            page_level_cache_disable: PageLevelCacheDisableFlag::from_u32(value),
            paging_structure_base_address: PagingStructureBaseAddress::from_u32_unchecked(value),
        }
    }

    ///
    /// # Description
    ///
    /// Converts the CR3 register to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the CR3 register with all fields combined.
    ///
    pub fn into_u32(self) -> u32 {
        let mut value: u32 = 0;

        value |= self.page_level_write_through.into_u32();
        value |= self.page_level_cache_disable.into_u32();
        value |= self.paging_structure_base_address.into_u32();

        value
    }

    ///
    /// # Description
    ///
    /// Reads the value of the CR3 register.
    ///
    /// # Return Value
    ///
    /// The value of the CR3 register.
    ///
    /// # Safety
    ///
    /// It is unsafe to call this function because it executes privileged instructions.
    ///
    /// It is safe to call this function if the following conditions are met:
    /// - The caller runs at processor privilege level 0.
    ///
    pub unsafe fn read() -> Self {
        #[cfg(target_arch = "x86")]
        {
            // let value: u32;
            // asm!("mov {0:e}, cr3", out(reg) value);
            let value: u32 = unsafe { env_interaction_read_cr3() };
            Self::from_u32_unchecked(value & !Self::RESERVED_MASK)
        }

        #[cfg(target_arch = "x86_64")]
        {
            // let value: u64;
            // asm!("mov {0:r}, cr3", out(reg) value);
            let value: u64 = unsafe { env_interaction_read_cr3() };
            let value32: u32 = (value & u32::MAX as u64) as u32;
            Self::from_u32_unchecked(value32 & !Self::RESERVED_MASK)
        }
    }

    ///
    /// # Description
    ///
    /// Writes a value to the CR3 register.
    ///
    /// # Safety
    ///
    /// It is unsafe to call this function because it executes privileged instructions.
    ///
    /// It is safe to call this function if the following conditions are met:
    /// - The caller runs at processor privilege level 0.
    ///
    pub unsafe fn write(&self) {
        #[cfg(target_arch = "x86")]
        {
            let value: u32 = self.into_u32();
            // asm!("mov cr3, {0:e}", in(reg) value);
            unsafe {
                env_interaction_write_cr3(value);
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            let value64: u64 = self.into_u32() as u64;
            // asm!("mov cr3, {0:r}", in(reg) value64);
            unsafe {
                env_interaction_write_cr3(value64);
            }
        }
    }
}

impl Default for Cr3Register {
    fn default() -> Self {
        Self {
            page_level_write_through: PageLevelWriteThroughFlag::Disabled,
            page_level_cache_disable: PageLevelCacheDisableFlag::Enabled,
            paging_structure_base_address: PagingStructureBaseAddress(0),
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Tests if page-level write-through flag works.
fn test_page_level_write_through_flag() -> bool {
    let value: u32 = 0x00000008;

    if PageLevelWriteThroughFlag::from_u32(value) != PageLevelWriteThroughFlag::Enabled {
        return false;
    }

    let cr3: Cr3Register = Cr3Register {
        page_level_write_through: PageLevelWriteThroughFlag::Enabled,
        ..Cr3Register::default()
    };

    if Cr3Register::try_from_u32(value) != Some(cr3) {
        return false;
    }

    if cr3.into_u32() != value {
        return false;
    }

    cr3.into_u32() == PageLevelWriteThroughFlag::Enabled.into_u32()
}

/// Tests if page-level cache disable flag works.
fn test_page_level_cache_disable_flag() -> bool {
    let value: u32 = 0x00000010;

    if PageLevelCacheDisableFlag::from_u32(value) != PageLevelCacheDisableFlag::Disabled {
        return false;
    }

    let cr3: Cr3Register = Cr3Register {
        page_level_cache_disable: PageLevelCacheDisableFlag::Disabled,
        ..Cr3Register::default()
    };

    if Cr3Register::try_from_u32(value) != Some(cr3) {
        return false;
    }

    if cr3.into_u32() != value {
        return false;
    }

    cr3.into_u32() == PageLevelCacheDisableFlag::Disabled.into_u32()
}

/// Tests if paging structure base address works.
fn test_paging_structure_base_address() -> bool {
    let value: u32 = 0x12345000;

    let base: PagingStructureBaseAddress = match PagingStructureBaseAddress::try_from_u32(value) {
        Some(b) => b,
        None => return false,
    };
    if base.address() != 0x12345000 {
        return false;
    }

    let cr3: Cr3Register = Cr3Register {
        paging_structure_base_address: match PagingStructureBaseAddress::new(0x12345000) {
            Some(b) => b,
            None => return false,
        },
        ..Cr3Register::default()
    };

    if Cr3Register::try_from_u32(value) != Some(cr3) {
        return false;
    }

    if cr3.into_u32() != value {
        return false;
    }

    cr3.paging_structure_base_address.address() == 0x12345000
}

/// Tests if paging structure base address alignment works.
fn test_paging_structure_base_address_alignment() -> bool {
    // Non-aligned address must be rejected.
    PagingStructureBaseAddress::new(0x12345678).is_none()
}

/// Tests if combined CR3 fields work.
fn test_combined_fields() -> bool {
    let value: u32 = 0xabcde018;

    let cr3: Cr3Register = match Cr3Register::try_from_u32(value) {
        Some(cr3) => cr3,
        None => return false,
    };

    if cr3.page_level_write_through != PageLevelWriteThroughFlag::Enabled {
        return false;
    }

    if cr3.page_level_cache_disable != PageLevelCacheDisableFlag::Disabled {
        return false;
    }

    if cr3.paging_structure_base_address.address() != 0xabcde000 {
        return false;
    }

    cr3.into_u32() == value
}

/// Tests if reserved bits are rejected.
fn test_reserved_bits_rejected() -> bool {
    // Bit 0 set (reserved).
    if Cr3Register::try_from_u32(0x00001001).is_some() {
        return false;
    }
    // Bit 2 set (reserved).
    if Cr3Register::try_from_u32(0x00001004).is_some() {
        return false;
    }
    // Bit 5 set (reserved).
    if Cr3Register::try_from_u32(0x00001020).is_some() {
        return false;
    }
    // Bit 11 set (reserved).
    if Cr3Register::try_from_u32(0x00001800).is_some() {
        return false;
    }
    // Valid value should succeed.
    Cr3Register::try_from_u32(0x00001018).is_some()
}

// Runs all tests for this module.
pub fn test() -> bool {
    let mut passed: bool = true;

    passed &= test_page_level_write_through_flag();
    passed &= test_page_level_cache_disable_flag();
    passed &= test_paging_structure_base_address();
    passed &= test_paging_structure_base_address_alignment();
    passed &= test_combined_fields();
    passed &= test_reserved_bits_rejected();

    passed
}
