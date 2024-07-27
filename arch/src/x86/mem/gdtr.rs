// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Global Descriptor Table Register (GDTR)
//==================================================================================================

/// Global descriptor table register (GDTR).
#[derive(Default)]
#[repr(C, packed)]
pub struct Gdtr {
    limit: u16,
    base: u32,
}

// `Gdtptr` must be 6 bytes long. This must match the hardware specification.
static_assert_size!(Gdtr, 6);

impl Gdtr {
    pub fn new(base: u32, size: u16) -> Self {
        Self {
            base,
            limit: size - 1,
        }
    }
}
