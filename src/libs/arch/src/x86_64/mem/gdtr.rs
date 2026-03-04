// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Global Descriptor Table Register (GDTR)
//==================================================================================================

/// Global descriptor table register (GDTR) for x86_64.
/// In long mode, the base address is 64 bits.
#[derive(Default)]
#[repr(C, packed)]
pub struct Gdtr {
    limit: u16,
    base: u64,
}

// `Gdtr` must be 10 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Gdtr, 10);

impl Gdtr {
    pub fn new(base: u64, size: u16) -> Self {
        Self {
            base,
            limit: size - 1,
        }
    }
}
