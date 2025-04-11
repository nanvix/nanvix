// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::VirtualAddress,
    kcall::{
        KcallArgs,
        KcallResult,
    },
    pm::ProcessManager,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_debug(buf: &[u8]) -> Result<(), Error> {
    let message: &str = match core::str::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => {
            let reason: &str = "invalid UTF-8";
            error!("debug(): {} (error={:?})", reason, e);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        },
    };
    unsafe { crate::klog::puts(message) }

    Ok(())
}

pub fn debug(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    // Buffer size in bytes.
    // NOTE: This value was chosen to be smaller than a page, but big enough for along messages.
    const BUFFER_SIZE: usize = 256;
    let user_buffer: usize = args.arg0 as usize;
    let size: usize = args.arg1 as usize;

    // skip zero-length messages
    if size == 0 {
        return KcallResult::ok();
    }

    // Sanity check message size.
    if size > BUFFER_SIZE {
        let reason: &str = "message too large";
        error!("debug() {}", reason);
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    let mut kernel_buffer: [u8; BUFFER_SIZE + 1] = [0; BUFFER_SIZE + 1];

    let src: VirtualAddress = VirtualAddress::new(user_buffer);
    let dst: VirtualAddress = VirtualAddress::new(kernel_buffer.as_mut_ptr() as usize);

    if let Err(e) = pm.vmcopy_from_user(args.pid, dst, src, size) {
        return KcallResult::Error(e.code.into());
    }

    let buf: &[u8] = unsafe { core::slice::from_raw_parts(kernel_buffer.as_ptr(), size) };

    match do_debug(buf) {
        Ok(()) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
