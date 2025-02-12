// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    mm,
    mm::{
        VirtualAddress,
        PAGE_ALIGNMENT,
    },
    pm,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================///

/// # Description
///
/// The `sbrk()` increments the location of the program break by `size` bytes. The program break
/// defines the end of the process's data segment and is the address of the first location after the
/// end of the uninitialized data segment. Increasing the program break has the effect of allocating
/// memory to the process; decreasing the break deallocates memory.
///
/// # Parameters
///
/// - `size`: Number of bytes to increment the program break.
///
/// # Returns
///
/// Upon successful completion, the `sbrk()` function returns the address of the start of the newly
/// allocated memory. Otherwise, it returns an error code.
///
pub fn sbrk(size: isize) -> Result<*mut u8, Error> {
    ::nvx::trace!("sbrk(): size = {}", size);
    static mut END: *mut u8 = mm::BREAK_BASE_RAW as *mut u8;

    let old_end: *mut u8 = unsafe {
        let old_end: *mut u8 = END;
        let new_end: *mut u8 = END.offset(size);

        // Align the new end.
        let new_end: *mut u8 = mm::align_up(new_end as usize, PAGE_ALIGNMENT) as *mut u8;

        // Check for overflow.
        // TODO: remove this check and let the page fault handler run.
        if new_end >= (mm::BREAK_BASE_RAW + mm::C_HEAP_SIZE) as *mut u8 {
            return Err(Error::new(ErrorCode::OutOfMemory, "out of memory"));
        }

        let pid: ProcessIdentifier = pm::getpid()?;

        nvx::mm::heap::map_range(
            pid,
            VirtualAddress::from_raw_value(old_end as usize),
            VirtualAddress::from_raw_value(new_end as usize),
        )?;

        END = new_end;
        old_end
    };

    Ok(old_end)
}
