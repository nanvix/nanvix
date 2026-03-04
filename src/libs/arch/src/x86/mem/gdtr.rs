// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Global Descriptor Table Register (GDTR)
//==================================================================================================

#[cfg(target_arch = "x86")]
/// Global descriptor table register (GDTR).
#[derive(Default)]
#[repr(C, packed)]
pub struct Gdtr {
    limit: u16,
    base: u32,
}

#[cfg(target_arch = "x86")]
// `Gdtr` must be 6 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Gdtr, 6);

#[cfg(target_arch = "x86")]
impl Gdtr {
    pub fn new(base: u32, size: u16) -> Self {
        Self {
            base,
            limit: size - 1,
        }
    }
}

#[cfg(target_arch = "x86_64")]
/// Global descriptor table register (GDTR) for 64-bit x86_64.
#[derive(Default)]
#[repr(C, packed)]
pub struct Gdtr {
    limit: u16,
    base: u64,
}

#[cfg(target_arch = "x86_64")]
// `Gdtr` must be 10 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Gdtr, 10);

#[cfg(target_arch = "x86_64")]
impl Gdtr {
    pub fn new(base: u64, size: u16) -> Self {
        Self {
            base,
            limit: size - 1,
        }
    }
}
