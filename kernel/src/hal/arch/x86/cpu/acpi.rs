// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::arch::cpu::acpi::AcpiSdtHeader;
use ::core::ffi::CStr;

//==================================================================================================
// Imports
//==================================================================================================

///
/// Finds an APIC table by its signature.
///
/// # Arguments
///
/// * `rsdt` - Root System Description Table.
/// * `sig` - Signature of the table.
///
/// # Returns
///
/// Upon successful completion, a pointer to the table is returned. Upon failure, `None` is returned instead.
pub unsafe fn find_table_by_sig(
    rsdt: *const AcpiSdtHeader,
    sig: &str,
) -> Result<*const AcpiSdtHeader, Error> {
    let entries = ((*rsdt).length as usize - core::mem::size_of::<AcpiSdtHeader>())
        / core::mem::size_of::<u32>();

    info!("looking for table: {:?} in {:?} entries", sig, entries);

    let ptr: *const u32 = rsdt.offset(1) as *const u32;

    for i in 0..entries {
        info!("ptr: {:p}", ptr);
        let ptr = ptr.add(i);
        info!("ptr: {:p}", ptr);

        let table = (ptr.read_unaligned()) as *const AcpiSdtHeader;

        let buf: [i8; 5] = [
            (*table).signature[0],
            (*table).signature[1],
            (*table).signature[2],
            (*table).signature[3],
            0,
        ];

        let signature = match CStr::from_ptr(buf.as_ptr()).to_str() {
            Ok(sig) => sig,
            Err(_) => {
                let reason: &str = "invalid signature";
                warn!("find_table_by_sig(): {}", reason);
                continue;
            },
        };

        // Print signature.
        info!("Signature Found: {:?}", signature);

        // Check signature.
        if signature == sig {
            if AcpiSdtHeader::from_ptr(table).is_none() {
                continue;
            }

            return Ok(table);
        }
    }

    // Table not found.
    let reason: &str = "table not found";
    error!("find_table_by_sig(): {}", reason);
    Err(Error::new(ErrorCode::NoSuchEntry, reason))
}
