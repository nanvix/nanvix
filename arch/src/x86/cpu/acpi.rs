// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;
use ::core::ffi::CStr;

//==================================================================================================
// Structures
//==================================================================================================

#[repr(C, packed)]
/// System Description Table (SDT) header.
#[derive(Debug)]
pub struct AcpiSdtHeader {
    // ASCII string representation of the table ID.
    pub signature: [i8; 4],
    // Length of the table.
    pub length: u32,
    // Revision of the structure.
    pub revision: u8,
    // Checksum of the table.
    pub checksum: u8,
    // ASCII string that identifies the OEM.
    pub oem_id: [i8; 6],
    // ASCII string that identifies the OEM table.
    pub oem_table_id: [i8; 8],
    // OEM revision number.
    pub oem_revision: u32,
    // Vendor ID of the util that created the table.
    pub creator_id: u32,
    // Revision of the util that created the table.
    pub creator_rev: u32,
}

impl AcpiSdtHeader {
    const _SIZE: usize = 36;

    pub unsafe fn from_ptr(ptr: *const AcpiSdtHeader) -> Option<Self> {
        // Validate SDT.
        if !unsafe { do_checksum(ptr as *const u8, (*ptr).length as usize) } {
            return None;
        }

        let sdt: AcpiSdtHeader = Self {
            signature: [
                (*ptr).signature[0],
                (*ptr).signature[1],
                (*ptr).signature[2],
                (*ptr).signature[3],
            ],
            length: (*ptr).length,
            revision: (*ptr).revision,
            checksum: (*ptr).checksum,
            oem_id: [
                (*ptr).oem_id[0],
                (*ptr).oem_id[1],
                (*ptr).oem_id[2],
                (*ptr).oem_id[3],
                (*ptr).oem_id[4],
                (*ptr).oem_id[5],
            ],
            oem_table_id: [
                (*ptr).oem_table_id[0],
                (*ptr).oem_table_id[1],
                (*ptr).oem_table_id[2],
                (*ptr).oem_table_id[3],
                (*ptr).oem_table_id[4],
                (*ptr).oem_table_id[5],
                (*ptr).oem_table_id[6],
                (*ptr).oem_table_id[7],
            ],
            oem_revision: (*ptr).oem_revision,
            creator_id: (*ptr).creator_id,
            creator_rev: (*ptr).creator_rev,
        };

        Some(sdt)
    }

    pub fn signature(&self) -> Option<String> {
        let buf: [i8; 5] = [
            self.signature[0],
            self.signature[1],
            self.signature[2],
            self.signature[3],
            0,
        ];

        match unsafe { CStr::from_ptr(buf.as_ptr()).to_str() } {
            Ok(s) => Some(String::from(s)),
            Err(_) => None,
        }
    }

    pub fn oemid(&self) -> Option<String> {
        let buf: [i8; 7] = [
            self.oem_id[0],
            self.oem_id[1],
            self.oem_id[2],
            self.oem_id[3],
            self.oem_id[4],
            self.oem_id[5],
            0,
        ];

        match unsafe { CStr::from_ptr(buf.as_ptr()).to_str() } {
            Ok(s) => Some(String::from(s)),
            Err(_) => None,
        }
    }

    pub fn oem_table_id(&self) -> Option<String> {
        let buf: [i8; 8] = [
            self.oem_table_id[0],
            self.oem_table_id[1],
            self.oem_table_id[2],
            self.oem_table_id[3],
            self.oem_table_id[4],
            self.oem_table_id[5],
            self.oem_table_id[6],
            0,
        ];

        match unsafe { CStr::from_ptr(buf.as_ptr()).to_str() } {
            Ok(s) => Some(String::from(s)),
            Err(_) => None,
        }
    }
}

static_assert_size!(AcpiSdtHeader, AcpiSdtHeader::_SIZE);

#[derive(Debug)]
#[repr(C, packed)]
/// Root System Description Pointer (RSDP).
pub struct Rsdp {
    // ASCII string representation of the signature.
    pub signature: [i8; 8],
    // Checksum of the table.
    pub checksum: u8,
    // ASCII string that identifies the OEM.
    pub oemid: [i8; 6],
    // Revision of the structure.
    pub revision: u8,
    // Address of the RSDT.
    pub rsdt_addr: u32,
}

impl Rsdp {
    const _SIZE: usize = 20;

    pub unsafe fn from_ptr(ptr: *const u8) -> Option<Self> {
        // Check if checksum is invalid.
        if !unsafe { do_checksum(ptr, core::mem::size_of::<Rsdp>()) } {
            return None;
        }

        let ptr = ptr as *const Rsdp;

        // print signature.

        let rsdp: Self = Self {
            signature: [
                (*ptr).signature[0],
                (*ptr).signature[1],
                (*ptr).signature[2],
                (*ptr).signature[3],
                (*ptr).signature[4],
                (*ptr).signature[5],
                (*ptr).signature[6],
                (*ptr).signature[7],
            ],
            checksum: (*ptr).checksum,
            oemid: [
                (*ptr).oemid[0],
                (*ptr).oemid[1],
                (*ptr).oemid[2],
                (*ptr).oemid[3],
                (*ptr).oemid[4],
                (*ptr).oemid[5],
            ],
            revision: (*ptr).revision,
            rsdt_addr: (*ptr).rsdt_addr,
        };

        if rsdp.revision != 0 {
            return None;
        }

        Some(rsdp)
    }

    pub fn signature(&self) -> Option<String> {
        let buf: [i8; 8] = [
            self.signature[0],
            self.signature[1],
            self.signature[2],
            self.signature[3],
            self.signature[4],
            self.signature[5],
            self.signature[6],
            0,
        ];

        match unsafe { CStr::from_ptr(buf.as_ptr()).to_str() } {
            Ok(s) => Some(String::from(s)),
            Err(_) => None,
        }
    }

    pub fn oemid(&self) -> Option<String> {
        let buf: [i8; 6] = [
            self.oemid[0],
            self.oemid[1],
            self.oemid[2],
            self.oemid[3],
            self.oemid[4],
            0,
        ];

        match unsafe { CStr::from_ptr(buf.as_ptr()).to_str() } {
            Ok(s) => Some(String::from(s)),
            Err(_) => None,
        }
    }
}

static_assert_size!(Rsdp, Rsdp::_SIZE);

/// Checks if the checksum of a given ACPI table is valid.
///
/// # Arguments
///
/// * `start` - Start address of the ACPI table.
/// * `len` - Length of the ACPI table.
///
/// # Returns
///
/// Upon successful completion, `true` is returned. Upon failure, `false` is returned instead.
unsafe fn do_checksum(start: *const u8, len: usize) -> bool {
    let data = core::slice::from_raw_parts(start, len);
    data.iter().fold(0u8, |acc, &x| acc.wrapping_add(x)) == 0
}
