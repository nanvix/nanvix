// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use alloc::string::String;

use crate::{
    arch::cpu::acpi::AcpiSdtHeader,
    error::{
        Error,
        ErrorCode,
    },
    hal::arch::x86::cpu::acpi,
};
use core::ffi::CStr;

//==================================================================================================
// Implementations
//==================================================================================================

impl AcpiSdtHeader {
    pub unsafe fn from_ptr(ptr: *const AcpiSdtHeader) -> Result<Self, Error> {
        // Validate SDT.
        if !unsafe { acpi::do_checksum(ptr as *const u8, (*ptr).length as usize) } {
            let reason: &str = "invalid sdt checksum";
            error!("from_raw_ptr(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
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

        Ok(sdt)
    }

    pub fn signature(&self) -> Result<String, Error> {
        let buf: [i8; 5] = [
            self.signature[0],
            self.signature[1],
            self.signature[2],
            self.signature[3],
            0,
        ];

        match unsafe { CStr::from_ptr(buf.as_ptr()).to_str() } {
            Ok(s) => Ok(String::from(s)),
            Err(_) => Err(Error::new(ErrorCode::InvalidArgument, "invalid signature")),
        }
    }

    pub fn oemid(&self) -> Result<&str, Error> {
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
            Ok(s) => Ok(s),
            Err(_) => Err(Error::new(ErrorCode::InvalidArgument, "invalid oemid")),
        }
    }

    pub fn oem_table_id(&self) -> Result<String, Error> {
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
            Ok(s) => Ok(String::from(s)),
            Err(_) => Err(Error::new(ErrorCode::InvalidArgument, "invalid oem_table_id")),
        }
    }

    pub fn display(&self) {
        info!("ACPI Header:");
        if let Ok(s) = self.signature() {
            info!("  Signature: {}", s);
        }
        let header_length = self.length;
        info!("  Length: {}", header_length);
        info!("  Revision: {}", self.revision);
        info!("  Checksum: {}", self.checksum);
        if let Ok(s) = self.oemid() {
            info!("  OEM ID: {}", s);
        }
        if let Ok(s) = self.oem_table_id() {
            info!("  OEM Table ID: {}", s);
        }
        let oem_revision = self.oem_revision;
        info!("  OEM Revision: {}", oem_revision);
        let creator_id = self.creator_id;
        info!("  Creator ID: {}", creator_id);
        let creator_rev = self.creator_rev;
        info!("  Creator Revision: {}", creator_rev);
    }
}
