// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::PhysicalAddress,
    kmod::KernelModule,
};
use ::alloc::vec::Vec;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::Address,
};

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Parses a multibinary image and returns a list of kernel modules plus the shared kernel
/// arguments.
///
/// Each entry in the multibinary image becomes a [`KernelModule`] whose physical address is
/// computed as `initrd_base + entry.offset`. Kernel arguments are stored once in the image
/// header and apply to the entire image rather than to individual modules.
///
/// **Note:** Kernel arguments from the multibinary header are stored verbatim and do not
/// undergo `\;` unescape processing. The `\;` escape convention applies only to the packed
/// command-line string format used in the single-binary initrd path. Because multibinary
/// kernel args are structurally separated by the image header, semicolon escaping is not
/// needed.
///
/// # Parameters
///
/// - `image_data`: Raw bytes of the multibinary image.
/// - `initrd_base`: Physical base address where the image is loaded in guest memory.
///
/// # Returns
///
/// A tuple `(modules, kernel_args)` on success, or an [`Error`] if the image is malformed.
///
pub fn parse(
    image_data: &'static [u8],
    initrd_base: usize,
) -> Result<(Vec<KernelModule>, &'static str), Error> {
    let parsed: multibin::ParseResult = multibin::parse(image_data).map_err(|e| {
        error!("parse(): failed to parse multibinary image: {:?}", e);
        e
    })?;

    info!(
        "multibinary image: base={:#010x}, size={:#010x}, entries={}",
        initrd_base,
        image_data.len(),
        parsed.count()
    );

    // Extract kernel arguments from the image header.
    let kernel_args: &str = if parsed.kernel_args_size() > 0 {
        let ka_bytes: &[u8] = &image_data
            [parsed.kernel_args_offset()..parsed.kernel_args_offset() + parsed.kernel_args_size()];
        match core::str::from_utf8(ka_bytes) {
            Ok(s) => s,
            Err(_) => {
                let reason: &str = "invalid UTF-8 in multibinary kernel args";
                error!("parse(): {}", reason);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        }
    } else {
        ""
    };

    let mut modules: Vec<KernelModule> = Vec::new();

    for entry in parsed.iter() {
        let entry_phys_addr: usize = match initrd_base.checked_add(entry.offset) {
            Some(addr) => addr,
            None => {
                let reason: &str = "multibinary entry offset overflows physical address";
                error!("parse(): {}", reason);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        let cmdline_bytes: &[u8] =
            &image_data[entry.cmdline_offset..entry.cmdline_offset + entry.cmdline_size];
        let cmdline: &str = match core::str::from_utf8(cmdline_bytes) {
            Ok(s) => s,
            Err(_) => {
                let reason: &str = "invalid UTF-8 in multibinary entry cmdline";
                error!("parse(): {}", reason);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        info!(
            "  entry: addr={:#010x}, size={:#010x}, cmdline={:?}",
            entry_phys_addr, entry.size, cmdline
        );

        // Each module's mapped region covers the entire multibinary image so that
        // the header pages (containing cmdline strings) remain accessible after
        // the kernel switches to the new page table.
        let module: KernelModule = KernelModule::new_with_region(
            PhysicalAddress::from_raw_value(entry_phys_addr)?,
            entry.size,
            PhysicalAddress::from_raw_value(initrd_base)?,
            image_data.len(),
            cmdline,
        );
        modules.push(module);
    }

    Ok((modules, kernel_args))
}
