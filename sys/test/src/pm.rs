/*
 * Copyright(c) 2011-2024 The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==============================================================================
// Imports
//==============================================================================

use core::ffi;

use nanvix::pm::{
    self,
    Pid,
    ProcessInfo,
    Tid,
};

//==============================================================================
// Constants
//==============================================================================

const THREAD_ARG_VAL: u32 = 0xdab;
const THREAD_RET_VAL: u32 = 0x86;

//==============================================================================
// Private Standalone Functions
//==============================================================================

/// Checks if sizes are conformant.
fn check_sizes() -> bool {
    if core::mem::size_of::<Pid>() != 4 {
        nanvix::log!("unexpected size for Pid");
        return false;
    }
    if core::mem::size_of::<ProcessInfo>() != 12 {
        nanvix::log!("unexpected size for ProcessInfo");
        return false;
    }

    true
}

/// Attempts to get information on the calling process.
fn get_process_info() -> bool {
    // Attempt to get information on the calling process.
    let mut pinfo: ProcessInfo = ProcessInfo::default();
    let result: i32 = pm::pinfo(pm::PID_SELF, &mut pinfo);

    // Check if we failed to get information on the calling process.
    if result != 0 {
        nanvix::log!("failed to get information on the calling process");
        return false;
    }

    // Assert process information.
    if pinfo.pid == 1 {
        nanvix::log!("unexpected PID");
        return false;
    }
    if pinfo.tid == 1 {
        nanvix::log!("unexpected TID");
        return false;
    }
    if pinfo.vmem == 1 {
        nanvix::log!("unexpected virtual memory space");
        return false;
    }

    true
}

/// Attempts to get information on an invalid process.
fn get_process_info_invalid_pid() -> bool {
    // Attempt to get information on an invalid process.
    let mut pinfo: ProcessInfo = ProcessInfo::default();
    let result: i32 = pm::pinfo(i32::MAX, &mut pinfo);

    // Check if we have succeeded.
    if result == 0 {
        // We should have failed.
        nanvix::log!("succeded to get information on an invalid process");
        return false;
    }

    true
}

/// Attempts to get information on an invalid process, but with an invalid buffer.
fn get_process_info_invalid_buf() -> bool {
    // Attempt to get information on an valid process, but with an invalid buffer.
    let result: i32 = pm::pinfo(pm::PID_SELF, core::ptr::null_mut());

    // Check if we have succeeded.
    if result == 0 {
        // We should have failed.
        nanvix::log!(
            "succeded to get information on a valid process with an invalid \
             buffer"
        );
        return false;
    }

    true
}

/// Attempts to get information on an invalid process, but with a bad buffer.
fn get_process_info_bad_buf() -> bool {
    // Attempt to get information on an valid process, but with a bad buffer.
    let pinfo: *mut ProcessInfo = 0x02000000 as *mut ProcessInfo;
    let result: i32 = pm::pinfo(pm::PID_SELF, pinfo);

    // Check if we have succeeded.
    if result == 0 {
        // We should have failed.
        nanvix::log!(
            "succeded to get information on a valid process with a bad buffer"
        );
        return false;
    }

    true
}

fn test_thread_getid() -> bool {
    let result: Tid = pm::thread_getid();
    if result < 0 {
        return false;
    }

    true
}

fn thread_multijoin_test(arg: *mut ffi::c_void) -> *mut ffi::c_void {
    let mut retval: *mut ffi::c_void = core::ptr::null_mut();
    let tid = pm::thread_getid();
    pm::thread_detach(tid);
    pm::thread_join(arg as Tid, &mut retval);
    if (retval as u32) == THREAD_RET_VAL {
        nanvix::log!("Tid {} succeeded to retrive thread return value", tid);
    } else {
        nanvix::log!("Tid {} failed to retrive thread return value", tid);
    }
    core::ptr::null_mut()
}

fn thread_func_test(arg: *mut ffi::c_void) -> *mut ffi::c_void {
    let tid = pm::thread_getid();
    pm::thread_create(
        thread_multijoin_test,
        pm::thread_getid() as *mut ffi::c_void,
    );
    if (arg as u32) == THREAD_ARG_VAL {
        nanvix::log!("Tid {} succeeded to retrive thread argument", tid);
    } else {
        nanvix::log!("Tid {} failed to retrive stack argument", tid);
    }
    THREAD_RET_VAL as *mut ffi::c_void
}

fn test_thread_create() -> bool {
    let tid: Tid =
        pm::thread_create(thread_func_test, THREAD_ARG_VAL as *mut ffi::c_void);
    if tid < 0 {
        return false;
    }

    let mut retval: *mut ffi::c_void = core::ptr::null_mut();
    pm::thread_join(tid, &mut retval);
    if (retval as u32) == THREAD_RET_VAL {
        nanvix::log!(
            "Tid {} succeeded to retrive thread return value",
            pm::thread_getid()
        );
    } else {
        nanvix::log!(
            "Tid {} failed to retrive thread return value",
            pm::thread_getid()
        );
    }

    true
}

//==============================================================================
// Public Standalone Functions
//==============================================================================

///
/// **Description**
///
/// Tests kernel calls in the process management facility.
///
pub fn test() {
    crate::test!(check_sizes());
    crate::test!(get_process_info());
    crate::test!(get_process_info_invalid_pid());
    crate::test!(get_process_info_invalid_buf());
    crate::test!(get_process_info_bad_buf());
    crate::test!(test_thread_getid());
    crate::test!(test_thread_create());
}
