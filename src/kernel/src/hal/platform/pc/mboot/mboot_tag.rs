// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Multiboot Tag Type
//==================================================================================================

#[repr(u16)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum MbootTagType {
    End = 0,
    Cmdline = 1,
    BootLoaderName = 2,
    Module = 3,
    BasicMeminfo = 4,
    Bootdev = 5,
    Mmap = 6,
    Vbe = 7,
    Framebuffer = 8,
    ElfSections = 9,
    Apm = 10,
    Efi32 = 11,
    Efi64 = 12,
    Smbios = 13,
    AcpiOld = 14,
    AcpiNew = 15,
    Network = 16,
    EfiMmap = 17,
    EfiBs = 18,
    Efi32Ih = 19,
    Efi64Ih = 20,
    LoadBaseAddr = 21,
}

impl From<u16> for MbootTagType {
    fn from(value: u16) -> Self {
        match value {
            0 => MbootTagType::End,
            1 => MbootTagType::Cmdline,
            2 => MbootTagType::BootLoaderName,
            3 => MbootTagType::Module,
            4 => MbootTagType::BasicMeminfo,
            5 => MbootTagType::Bootdev,
            6 => MbootTagType::Mmap,
            7 => MbootTagType::Vbe,
            8 => MbootTagType::Framebuffer,
            9 => MbootTagType::ElfSections,
            10 => MbootTagType::Apm,
            11 => MbootTagType::Efi32,
            12 => MbootTagType::Efi64,
            13 => MbootTagType::Smbios,
            14 => MbootTagType::AcpiOld,
            15 => MbootTagType::AcpiNew,
            16 => MbootTagType::Network,
            17 => MbootTagType::EfiMmap,
            18 => MbootTagType::EfiBs,
            19 => MbootTagType::Efi32Ih,
            20 => MbootTagType::Efi64Ih,
            21 => MbootTagType::LoadBaseAddr,
            _ => panic!("invalid multiboot tag type {}", value),
        }
    }
}
