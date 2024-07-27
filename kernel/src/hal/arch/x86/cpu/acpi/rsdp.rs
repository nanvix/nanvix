// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use alloc::string::String;

use crate::{
    arch::cpu::acpi::Rsdp,
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

impl Rsdp {
    pub unsafe fn from_ptr(ptr: *const u8) -> Result<Self, Error> {
        // Check if checksum is invalid.
        if !unsafe { acpi::do_checksum(ptr, core::mem::size_of::<Rsdp>()) } {
            let reason: &str = "invalid rsdp checksum";
            error!("from_raw_ptr(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
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
            let reason: &str = "acpi version >= 2.0 not supported";
            error!("from_raw_ptr(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        Ok(rsdp)
    }

    pub fn signature(&self) -> Result<String, Error> {
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

        unsafe {
            info!("Signature: {:?}", core::ffi::CStr::from_ptr(buf.as_ptr()).to_str().unwrap())
        };

        match unsafe { CStr::from_ptr(buf.as_ptr()).to_str() } {
            Ok(s) => Ok(String::from(s)),
            Err(_) => Err(Error::new(ErrorCode::InvalidArgument, "invalid signature")),
        }
    }

    pub fn oemid(&self) -> Result<String, Error> {
        let buf: [i8; 6] = [
            self.oemid[0],
            self.oemid[1],
            self.oemid[2],
            self.oemid[3],
            self.oemid[4],
            0,
        ];

        match unsafe { CStr::from_ptr(buf.as_ptr()).to_str() } {
            Ok(s) => Ok(String::from(s)),
            Err(_) => Err(Error::new(ErrorCode::InvalidArgument, "invalid oemid")),
        }
    }

    pub fn display(&self) {
        info!("RSDP:");
        if let Ok(signature) = self.signature() {
            info!("  Signature: {}", signature);
        }
        let checksum: u8 = self.checksum;
        info!("  Checksum: 0x{:x}", checksum);
        if let Ok(oemid) = self.oemid() {
            info!("  OEM ID: {}", oemid);
        }
        let revision: u8 = self.revision;
        info!("  Revision: 0x{:x}", revision);
        let rsdt_addr: u32 = self.rsdt_addr;
        info!("  RSDT Address: 0x{:x}", rsdt_addr);
    }
}
