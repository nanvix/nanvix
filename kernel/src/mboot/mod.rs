// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod acpi;
pub mod info;
mod memory_map;
mod module;

//==================================================================================================
// Imports
//==================================================================================================

use self::{
    acpi::MbootAcpi,
    memory_map::MbootMemoryMap,
    module::MbootModule,
};
use crate::{
    arch::{
        self,
        cpu::{
            acpi::{
                AcpiSdtHeader,
                Rsdp,
            },
            madt::Madt,
        },
    },
    error::{
        Error,
        ErrorCode,
    },
    hal::{
        arch::x86::cpu::{
            self,
            madt::madt::MadtInfo,
        },
        mem::{
            AccessPermission,
            Address,
            MemoryRegion,
            MemoryRegionType,
            PageAligned,
            PhysicalAddress,
            TruncatedMemoryRegion,
            VirtualAddress,
        },
    },
    kmod::KernelModule,
    mboot::info::BootInfo,
};
use alloc::{
    collections::LinkedList,
    string::{
        String,
        ToString,
    },
};
use core::mem;

//==================================================================================================
// Constants
//==================================================================================================

/// Bootloader magic value.
const MBOOT_BOOTLOADER_MAGIC: u32 = 0x36d76289;

//==================================================================================================
// Multiboot Tag Type
//==================================================================================================

#[repr(u16)]
enum MbootTagType {
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

//==================================================================================================
// Multiboot Tag
//==================================================================================================

///
/// # Description
///
/// Multiboot header tag.
///
#[repr(C, align(8))]
struct MbootTag {
    /// Type.
    typ: u16,
    /// Flags
    flags: u16,
    /// Size.
    size: u32,
}

// `MbootTag` must be 8 bytes long. This must match the multiboot specification.
static_assert_size!(MbootTag, 8);

// `MbootTag` must be 8-byte aligned. This must match the multiboot specification.
static_assert_alignment!(MbootTag, 8);

//==================================================================================================
// Implementations
//==================================================================================================

impl MbootTag {
    ///
    /// # Description
    ///
    /// Constructs a multiboot tag from a raw pointer.
    ///
    /// # Parameters
    ///
    /// - `ptr`: Pointer to the multiboot tag.
    ///
    /// # Returns
    ///
    /// Upon success, returns a reference to the multiboot tag and the announced size of the multiboot
    /// structure. Upon failure, an error is returned.
    ///
    /// # Safety
    ///
    /// This function is unsafe for the following reasons:
    /// - The caller must ensure that the `ptr` points to a valid multiboot tag.
    ///
    unsafe fn from_ptr<'a>(mut ptr: *const u8) -> Result<(&'a MbootTag, u64), Error> {
        // Ensure that `ptr` is not null.
        if ptr.is_null() {
            return Err(Error::new(ErrorCode::InvalidArgument, "null pointer"));
        }

        // Check if multiboot tag is misaligned.
        if !ptr.is_aligned_to(mem::size_of::<u64>()) {
            return Err(Error::new(ErrorCode::BadAddress, "unaligned pointer"));
        }

        let mbi_size: u64 = *(ptr as *const u64);

        // Check if pointer arithmetic wraps around the address space.
        if ptr.wrapping_add(mem::size_of::<u64>()) < ptr {
            return Err(Error::new(ErrorCode::BadAddress, "pointer arithmetic wraps around"));
        }

        ptr = ptr.add(mem::size_of::<u64>());

        // Check if `ptr` is misaligned.
        if !ptr.is_aligned_to(mem::align_of::<MbootTag>()) {
            return Err(Error::new(ErrorCode::BadAddress, "unaligned pointer"));
        }

        // Cast pointer to multiboot tag.
        Ok((&*(ptr as *const MbootTag), mbi_size))
    }

    ///
    /// # Description
    ///
    /// Returns the next multiboot tag.
    ///
    /// # Returns
    ///
    /// Upon success, returns a reference to the next multiboot tag. Upon failure, an error is returned.
    ///
    fn next_tag(&self) -> Result<&MbootTag, Error> {
        let mut ptr: *const u8 = self as *const MbootTag as *const u8;

        // Check if pointer arithmetic overflows `isize`.
        if self.size as usize > isize::MAX as usize {
            return Err(Error::new(ErrorCode::BadAddress, "pointer arithmetic overflows"));
        }

        // Check if pointer arithmetic wraps around the address space.
        if ptr.wrapping_add(self.size as usize) < ptr {
            return Err(Error::new(ErrorCode::BadAddress, "pointer arithmetic wraps around"));
        }

        // Skip current tag.
        ptr = unsafe {
            // Safety: `ptr` points to a valid multiboot tag.
            ptr.add(self.size as usize)
        };

        // Check if pointer arithmetic wraps around the address space.
        if ptr.wrapping_add(mem::size_of::<u64>()) < ptr {
            return Err(Error::new(ErrorCode::BadAddress, "pointer arithmetic wraps around"));
        }

        // Align pointer.
        ptr = unsafe {
            // Safety: `ptr` points to a valid multiboot tag.
            ptr.add(ptr.align_offset(mem::size_of::<u64>()))
        };

        // NOTE: no need to check if `ptr` is misaligned, as we just aligned it.

        // Safety: `ptr` points to a valid multiboot tag.
        unsafe { Ok(&*(ptr as *const MbootTag)) }
    }
}

impl core::fmt::Debug for MbootTag {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "typ={:?}, size={}", self.typ, self.size)
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Parse modules from Multiboot tag.
///
/// # Parameters
///
/// - `tag`: Mboot tag for parse.
/// - `kernel_modules`: List of kernel modules.
///
/// # Returns
///
/// Upon success, returns kernel modules list. Otherwise, it returns an error.
///
fn parse_module(
    tag: &MbootTag,
    mut kernel_modules: LinkedList<KernelModule>,
) -> Result<LinkedList<KernelModule>, Error> {
    let module: MbootModule = unsafe {
        // Safety: `MbootModule` is a prefix of `MbootTag`.
        let ptr: *const MbootTag = tag as *const MbootTag;
        // Safety: `ptr` points to a valid `MbootModule`.
        MbootModule::from_ptr(ptr as *const u8)?
    };
    module.display();

    // Add kernel module to the list of kernel modules.
    let start: PhysicalAddress = match module.start().try_into() {
        Ok(raw_addr) => PhysicalAddress::from_raw_value(raw_addr)?,
        Err(_) => {
            let reason: &'static str = "invalid kernel module start address";
            error!("parse_module(): {}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        },
    };

    let cmdline: String = module.cmdline().to_string();

    let module: KernelModule = KernelModule::new(start, module.size(), cmdline);
    kernel_modules.push_back(module);

    Ok(kernel_modules)
}

///
/// # Description
///
/// Parse Memory Regions from Multiboot tag.
///
/// # Parameters
///
/// - `tag`: Mboot tag for parse.
/// - `memory_regions`: List of memory regions.
///
/// # Returns
///
/// Upon success, returns memory region. Otherwise, it returns an error.
///
fn parse_mmap(
    tag: &MbootTag,
    mut memory_regions: LinkedList<MemoryRegion<VirtualAddress>>,
) -> Result<LinkedList<MemoryRegion<VirtualAddress>>, Error> {
    let mmap: MbootMemoryMap = unsafe {
        // Safety: `MbootMemoryMap` is a prefix of `MbootTag`.
        let ptr: *const MbootTag = tag as *const MbootTag;
        // Safety: `ptr` points to a valid `MbootMemoryMap`.
        MbootMemoryMap::from_ptr(ptr as *const u8)?
    };
    mmap.display();

    // Add memory regions to the list of memory regions.
    for entry in mmap {
        let typ: MemoryRegionType = entry.typ().into();
        let start: VirtualAddress = match entry.addr().try_into() {
            Ok(raw_addr) => VirtualAddress::from_raw_value(raw_addr)?,
            Err(_) => {
                let reason: &'static str = "invalid memory region address";
                error!("parse_mmap(): {}", reason);
                return Err(Error::new(ErrorCode::BadAddress, reason));
            },
        };
        let size: usize = match entry.len().try_into() {
            Ok(len) => len,
            Err(_) => {
                let reason: &'static str = "invalid memory region size";
                error!("parse_mmap(): {}", reason);
                return Err(Error::new(ErrorCode::BadAddress, reason));
            },
        };
        let perm: AccessPermission = if typ != MemoryRegionType::Bad {
            AccessPermission::new(
                crate::hal::mem::ReadPermission::Allow,
                crate::hal::mem::WritePermission::Allow,
                crate::hal::mem::ExecutePermission::Deny,
            )
        } else {
            AccessPermission::default()
        };
        let name: &str = match typ {
            MemoryRegionType::Usable => "usable",
            MemoryRegionType::Reserved => "reserved",
            MemoryRegionType::Mmio => "mmio",
            MemoryRegionType::Bad => "bad",
        };
        let region: MemoryRegion<VirtualAddress> = MemoryRegion::new(name, start, size, typ, perm)?;
        memory_regions.push_back(region);
    }

    Ok(memory_regions)
}

///
/// # Description
///
/// Parse Old Root System Description Pointer (RSDP) from Multiboot tag.
///
/// # Parameters
///
/// - `tag`: Mboot tag for parse.
/// - `mmio_regions`: List of mapped I/O memory regions.
///
/// # Returns
///
/// Upon success, returns information about the machine. Otherwise, it returns an error.
///
fn parse_acpiold(
    tag: &MbootTag,
    mmio_regions: &mut LinkedList<TruncatedMemoryRegion<VirtualAddress>>,
) -> Result<Option<MadtInfo>, Error> {
    let acpi: MbootAcpi = unsafe {
        // Safety: `MbootAcpi` is a prefix of `MbootTag`.
        let ptr: *const MbootTag = tag as *const MbootTag;
        // Safety: `ptr` points to a valid `MbootAcpi`.
        MbootAcpi::from_raw(ptr as *const u8)?
    };
    acpi.display();
    let rsdp: Rsdp = unsafe { Rsdp::from_ptr(acpi.rsdp())? };
    rsdp.display();
    let rsdt: *const AcpiSdtHeader = rsdp.rsdt_addr as *const AcpiSdtHeader;
    let rsdt: AcpiSdtHeader = unsafe { AcpiSdtHeader::from_ptr(rsdt) }?;
    rsdt.display();

    let ptr: *const AcpiSdtHeader =
        unsafe { cpu::acpi::find_table_by_sig(rsdp.rsdt_addr as *const AcpiSdtHeader, "APIC")? };
    rsdp.display();
    let madt: Option<MadtInfo> = match unsafe { Madt::parse(ptr as *const Madt) } {
        Ok(madt) => {
            madt.display();

            // If I/O APIC is present, book corresponding memory.
            if let Some(ioapic) = madt.get_ioapic_info() {
                let addr: PageAligned<VirtualAddress> =
                    PageAligned::from_raw_value(ioapic.io_apic_addr as usize)?;
                let region: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new(
                    "ioapic",
                    addr,
                    arch::mem::PAGE_SIZE,
                    MemoryRegionType::Mmio,
                    AccessPermission::RDWR,
                )?;
                mmio_regions.push_back(region);
            }

            // If local APIC is present, book corresponding memory.
            if madt.get_lapic_info().is_some() {
                let addr: PageAligned<VirtualAddress> =
                    PageAligned::from_raw_value(madt.local_apic_addr as usize)?;
                let region: TruncatedMemoryRegion<VirtualAddress> = TruncatedMemoryRegion::new(
                    "local_apic",
                    addr,
                    arch::mem::PAGE_SIZE,
                    MemoryRegionType::Mmio,
                    AccessPermission::RDWR,
                )?;
                mmio_regions.push_back(region);
            }

            Some(madt)
        },
        Err(err) => {
            warn!("failed to parse madt: {:?}", err);
            None
        },
    };

    Ok(madt)
}

///
/// # Description
///
/// Parse New Root System Description Pointer (RSDP) from Multiboot tag.
///
/// # Parameters
///
/// - `tag`: Mboot tag for parse.
///
/// # Returns
///
/// Upon success, returns RSDP structure. Otherwise, it returns an error.
///
fn parse_acpinew(tag: &MbootTag) -> Result<MbootAcpi, Error> {
    let acpi: MbootAcpi = unsafe {
        // Safety: `MbootAcpi` is a prefix of `MbootTag`.
        let ptr: *const MbootTag = tag as *const MbootTag;
        // Safety: `ptr` points to a valid `MbootAcpi`.
        MbootAcpi::from_raw(ptr as *const u8)?
    };

    Ok(acpi)
}

///
/// # Description
///
/// Parse Multiboot tags.
///
/// # Parameters
///
/// - `bootloader_magic`: Multiboot magic number.
/// - `addr`: Multiboot header address.
///
/// # Returns
///
/// Upon success, returns boot informations. Otherwise, it returns an error.
///
pub fn parse(bootloader_magic: u32, addr: usize) -> Result<BootInfo, Error> {
    // Check if bootloader magic value mismatches what we expect.
    if bootloader_magic != MBOOT_BOOTLOADER_MAGIC {
        return Err(Error::new(ErrorCode::InvalidArgument, "invalid bootloader magic value"));
    }

    let (mut tag, mbi_size): (&MbootTag, u64) = unsafe { MbootTag::from_ptr(addr as *const u8)? };

    info!("announced mboot size: {}", mbi_size);

    // List of memory regions.
    let mut memory_regions: LinkedList<MemoryRegion<VirtualAddress>> = LinkedList::new();
    // List of kernel modules.
    let mut kernel_modules: LinkedList<KernelModule> = LinkedList::new();
    // List of memory-mapped I/O regions.
    let mut mmio_regions: LinkedList<TruncatedMemoryRegion<VirtualAddress>> = LinkedList::new();
    // Machine information.
    let mut madt: Option<MadtInfo> = None;

    while tag.typ != MbootTagType::End as u16 {
        match tag.typ.into() {
            MbootTagType::Cmdline => {
                info!("command_line: {:?}", tag);
            },
            MbootTagType::BootLoaderName => {
                info!("bootloader_name: {:?}", tag);
            },
            MbootTagType::Module => {
                kernel_modules = parse_module(tag, kernel_modules)?;
            },
            MbootTagType::BasicMeminfo => {
                info!("basic_mem_info: {:?}", tag);
            },
            MbootTagType::Bootdev => {
                info!("boot_device: {:?}", tag);
            },
            MbootTagType::Mmap => {
                memory_regions = parse_mmap(tag, memory_regions)?;
            },
            MbootTagType::Vbe => {
                info!("vbe: {:?}", tag);
            },
            MbootTagType::Framebuffer => {
                info!("frame_buffer: {:?}", tag);
            },
            MbootTagType::ElfSections => {
                info!("elf_sections: {:?}", tag);
            },
            MbootTagType::Apm => {
                info!("apm: {:?}", tag);
            },
            MbootTagType::Efi32 => {
                info!("efi32: {:?}", tag);
            },
            MbootTagType::Efi64 => {
                info!("efi64: {:?}", tag);
            },
            MbootTagType::Smbios => {
                info!("smbios: {:?}", tag);
            },
            MbootTagType::AcpiOld => {
                madt = parse_acpiold(tag, &mut mmio_regions)?;
            },
            MbootTagType::AcpiNew => {
                let acpinew = parse_acpinew(tag)?;
                acpinew.display();
            },
            MbootTagType::Network => {
                info!("network: {:?}", tag);
            },
            MbootTagType::EfiMmap => {
                info!("efi_mmap: {:?}", tag);
            },
            MbootTagType::EfiBs => {
                info!("efi_bs: {:?}", tag);
            },
            MbootTagType::Efi32Ih => {
                info!("efi32_ih: {:?}", tag);
            },
            MbootTagType::Efi64Ih => {
                info!("efi64_ih: {:?}", tag);
            },
            MbootTagType::LoadBaseAddr => {
                info!("load_base_addr: {:?}", tag);
            },
            MbootTagType::End => {
                info!("end: {:?}", tag);

                // Check if the size of end tag mismatches what we expect.
                if tag.size != mem::size_of::<MbootTag>() as u32 {
                    return Err(Error::new(ErrorCode::BadAddress, "invalid end tag size"));
                }

                // Do not process further tags.
                break;
            },
        }

        tag = tag.next_tag()?;
    }

    // Compute total size of the multiboot structure.
    // NOTE: we intentionally add the size of the end tag here.
    let total_mbi_size: usize =
        { (tag as *const MbootTag as usize + mem::size_of::<MbootTag>()) - addr };

    info!("total mboot size: {}", total_mbi_size);

    // Check if total size mismatches what we expect.
    if total_mbi_size as u64 != mbi_size {
        return Err(Error::new(ErrorCode::BadAddress, "invalid multiboot size"));
    }

    Ok(BootInfo::new(madt, memory_regions, mmio_regions, kernel_modules))
}
