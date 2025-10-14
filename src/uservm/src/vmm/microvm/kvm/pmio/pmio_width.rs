// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

/// Port-mapped I/O transfer widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmioWidth {
    /// Byte-sized (1 byte) access.
    Byte = 1,
    /// Word-sized (2 bytes) access.
    Word = 2,
    /// Doubleword-sized (4 bytes) access.
    Dword = 4,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl From<PmioWidth> for usize {
    fn from(value: PmioWidth) -> Self {
        value as usize
    }
}

impl From<&PmioWidth> for usize {
    fn from(value: &PmioWidth) -> Self {
        *value as usize
    }
}

impl TryFrom<usize> for PmioWidth {
    type Error = usize;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Byte),
            2 => Ok(Self::Word),
            4 => Ok(Self::Dword),
            invalid => Err(invalid),
        }
    }
}
