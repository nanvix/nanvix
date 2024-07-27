// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

///
/// # Description
///
/// The `ApicBase` register holds the APIC base address, permitting the relocation of the APIC
/// memory map. For more information, see Intel® 64 and IA-32 Architectures Software Developer
/// Manuals.
///
#[derive(Clone, Copy)]
pub struct ApicBase(u64);

impl ApicBase {
    /// MSR address.
    const ADDRESS: u32 = 0x1B;

    /// Shift to BSP flag.
    const BSP_SHIFT: u64 = 8;
    /// Mask for BSP flag.
    const BSP_MASK: u64 = 1 << Self::BSP_SHIFT;

    /// Shift to APIC global enable flag.
    const ENABLE_SHIFT: u64 = 11;
    /// Mask for APIC global enable flag.
    const ENABLE_MASK: u64 = 1 << Self::ENABLE_SHIFT;

    /// Shift to APIC base address.
    const BASE_SHIFT: u64 = 12;
    /// Mask for APIC base address.
    const BASE_MASK: u64 = 0x7fff_ffff_ffff << Self::BASE_SHIFT;

    ///
    /// # Description
    ///
    /// Creates a new `ApicBase` register.
    ///
    /// # Parameters
    ///
    /// * `base`: APIC base address.
    /// * `bsp`: BSP flag.
    /// * `enable`: APIC global enable flag.
    ///
    pub fn new(base: u64, bsp: bool, enable: bool) -> Self {
        let mut value: u64 = base << Self::BASE_SHIFT;

        if bsp {
            value |= Self::BSP_MASK;
        }

        if enable {
            value |= Self::ENABLE_MASK;
        }

        Self(value)
    }

    ///
    /// # Description
    ///
    /// Gets the APIC base address.
    ///
    /// # Return Value
    ///
    /// The APIC base address.
    ///
    pub fn base(&self) -> u64 {
        (self.0 & Self::BASE_MASK) >> Self::BASE_SHIFT
    }

    ///
    /// # Description
    ///
    /// Gets the BSP flag.
    ///
    /// # Return Value
    ///
    /// The BSP flag.
    ///
    pub fn bsp(&self) -> bool {
        (self.0 & Self::BSP_MASK) != 0
    }

    ///
    /// # Description
    ///
    /// Gets the APIC global enable flag.
    ///
    /// # Return Value
    ///
    /// The APIC global enable flag.
    ///
    pub fn enable(&self) -> bool {
        (self.0 & Self::ENABLE_MASK) != 0
    }

    ////
    /// # Description
    ///
    /// Reads the `ApicBase` register.
    ///
    /// # Return Value
    ///
    /// The contents of the `ApicBase` register.
    ///
    pub fn read() -> Self {
        Self(super::rdmsr(Self::ADDRESS))
    }

    ////
    ///
    /// # Description
    ///
    /// Writes to the `ApicBase` register.
    ///
    /// # Parameters
    ///
    /// * `value`: Value to write.
    ///
    pub fn write(&self) {
        super::wrmsr(Self::ADDRESS, self.0);
    }
}

impl ::core::fmt::Debug for ApicBase {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "{{ base: {:#010x}, bsp: {}, enable: {} }}",
            self.base(),
            self.bsp(),
            self.enable()
        )
    }
}
