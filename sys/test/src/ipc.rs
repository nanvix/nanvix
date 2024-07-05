/*
 * Copyright(c) 2011-2024 The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==============================================================================
// Imports
//==============================================================================

use nanvix::{
    ipc,
    memory::{
        self,
        PageInfo,
        VirtualAddress,
        VirtualMemory,
    },
    misc,
    pm::{
        self,
        ProcessInfo,
    },
    security::AccessMode,
};

//==============================================================================
// Private Standalone Functions
//==============================================================================

/// Checks if sizes are conformant.
fn check_sizes() -> bool {
    if core::mem::size_of::<AccessMode>() != 4 {
        nanvix::log!("unexpected size for AccessMode");
        return false;
    }
    if core::mem::size_of::<VirtualMemory>() != 4 {
        nanvix::log!("unexpected size for VirtualMemory");
        return false;
    }
    if core::mem::size_of::<VirtualAddress>() != 4 {
        nanvix::log!("unexpected size for VirtualAddress");
        return false;
    }
    if core::mem::size_of::<memory::PhysicalAddress>() != 4 {
        nanvix::log!("unexpected size for PhysicalAddress");
        return false;
    }
    if core::mem::size_of::<memory::FrameNumber>() != 4 {
        nanvix::log!("unexpected size for FrameNumber");
        return false;
    }
    if core::mem::size_of::<PageInfo>() != 8 {
        nanvix::log!("unexpected size for PageInfo");
        return false;
    }
    if core::mem::size_of::<misc::KernelModule>() != 72 {
        nanvix::log!("unexpected size for KernelModule");
        return false;
    }

    if core::mem::size_of::<ProcessInfo>() != 12 {
        nanvix::log!("unexpected size for ProcessInfo");
        return false;
    }

    true
}

/// Test if Semaphore Get kernel call is working.
fn test_semget_call() -> bool {
    let key: u32 = 2012;
    let semid: i32 = pm::semget(key);

    if semid == -1 {
        return false;
    }

    true
}

/// Test if Semaphore Controler kernel call is working.
fn test_semctl_call() -> bool {
    let key: u32 = 2012;
    let semid: i32 = pm::semget(key);

    if semid == -1 {
        return false;
    }

    let val: u32 = 1;

    let setcountid: u32 = 1;
    let result: i32 = pm::semctl(semid as u32, setcountid, val);

    if result == -1 {
        return false;
    }

    // Test command 0 Get count id.
    let getcountid: u32 = 0;
    let result: i32 = pm::semctl(semid as u32, getcountid, val);

    if result as u32 != val {
        return false;
    }

    // Test command 2 Delete Semaphore.
    let deletesem: u32 = 2;
    let result: i32 = pm::semctl(semid as u32, deletesem, val);

    if result == -1 {
        return false;
    }

    true
}

/// Test if Semaphore Handler kernel call is working.
fn test_semop_call() -> bool {
    // Get semaphore and set a count value.
    let setcountid: u32 = 1;
    let key: u32 = 2012;
    let semid: i32 = pm::semget(key);
    let result = pm::semctl(semid as u32, setcountid, 0);

    if result != 0 {
        return false;
    }

    // Constants.
    let semaphore_up: u32 = 0;
    let semaphore_down: u32 = 1;
    let semaphore_trylock: u32 = 2;

    // Semaphore Up
    let result: i32 = pm::semop(semid as u32, semaphore_up);
    if result != 0 {
        return false;
    }

    // Semaphore Down
    let result: i32 = pm::semop(semid as u32, semaphore_down);
    if result != 0 {
        return false;
    }

    // Semaphore Trylock
    let eaddrinuse = 3;
    let result: i32 = pm::semop(semid as u32, semaphore_trylock);
    if result != (-eaddrinuse) {
        return false;
    }

    true
}

/// Test creation of a mailbox
fn do_mailbox_create_happy_path_testing() -> bool {
    let owner: u32 = 2;
    let mut tag: u32 = 1;
    let mut ombxid: i32;

    //Creating mailboxes
    for i in 0..ipc::MAILBOX_OPEN_MAX {
        ombxid = ipc::mailbox_create(owner, tag);
        tag += 1;
        if ombxid as u32 != i {
            nanvix::log!("Failed to create mailbox!");
            return false;
        }
    }

    //unlinking mailboxes
    for i in 0..ipc::MAILBOX_OPEN_MAX {
        if ipc::mailbox_unlink(i) != 0 {
            nanvix::log!("Failed to unlink mailbox!");
            return false;
        }
    }
    true
}

/// Test opening and closing of a mailbox
fn do_mailbox_open_close_happy_path_testing() -> bool {
    let owner: u32 = 2;
    let mut tag: u32 = 1;
    let mut ombxid: i32;

    //Creating mailboxes
    for i in 0..ipc::MAILBOX_OPEN_MAX {
        ombxid = ipc::mailbox_create(owner, tag);
        tag += 1;
        if ombxid as u32 != i {
            nanvix::log!("Failed to create mailbox!");
            return false;
        }
    }

    tag = 1;
    //Opening mailboxes
    for i in 0..ipc::MAILBOX_OPEN_MAX {
        ombxid = ipc::mailbox_open(owner, tag);
        tag += 1;
        if ombxid as u32 != i {
            nanvix::log!("Failed to open mailbox!");
            return false;
        }
    }

    //Closing mailboxes
    for i in 0..ipc::MAILBOX_OPEN_MAX {
        if ipc::mailbox_close(i) != 0 {
            nanvix::log!("Failed to close mailbox!");
            return false;
        }
    }

    true
}

/// Test writing and reading to/from mailboxes
fn do_mailbox_write_read_happy_path_testing() -> bool {
    let owner: u32 = 2;
    let mut tag: u32 = 1;
    let msg: [u32; 16] =
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut msg_buffer = &msg as *const u32;
    let mut ombxid: i32;

    //Writing
    for _ in 0..ipc::MAILBOX_OPEN_MAX {
        ombxid = ipc::mailbox_create(owner, tag);
        if ombxid < 0 {
            nanvix::log!("Failed to create mailbox!");
            return false;
        } else if ipc::mailbox_write(ombxid as u32, msg_buffer, 4) != 0 {
            nanvix::log!("Failed to write to mailbox!");
            return false;
        }
        unsafe {
            msg_buffer = msg_buffer.offset(1);
        }
        tag += 1;
    }

    //Reading
    let reader_msg: u32 = 0;
    let reader_msg_buffer: *const u32 = &reader_msg;
    for ombxid in (0..ipc::MAILBOX_OPEN_MAX).rev() {
        if ipc::mailbox_read(ombxid as u32, reader_msg_buffer, 4) != 0 {
            nanvix::log!("Failed to read from mailbox!");
            return false;
        }
    }

    //unlinking mailboxes
    for i in 0..ipc::MAILBOX_OPEN_MAX {
        if ipc::mailbox_unlink(i) != 0 {
            nanvix::log!("Failed to unlink mailbox!");
            return false;
        }
    }

    true
}

//Fault injection testing
fn do_mailbox_create_open_invalid_owner() -> bool {
    let mut owner: u32 = 2;
    let tag: u32 = 1;
    let ombxid: i32 = ipc::mailbox_create(owner, tag);

    //Creating mailbox
    if ombxid < 0 {
        nanvix::log!("Failed to create mailbox!");
        return false;
    }

    owner = 20; //invalid owner
                //Opening mailbox
                //it is supposed to fail since it was created a mailbox with owner = 2
    if (ipc::mailbox_open(owner, tag)) < 0 {
        nanvix::log!("Failed to open mailbox!");

        if ipc::mailbox_unlink(ombxid as u32) != 0 {
            nanvix::log!("Failed to remove mailbox!");
            return false;
        }
        return true;
    }

    false
}

//Fault injection testing
fn do_mailbox_write_invalid_pointer() -> bool {
    let owner: u32 = 2;
    let tag: u32 = 1;
    let msg: [u32; 16] =
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let msg_buffer = 0xFBADBEEF as *const u32;
    let ombxid: u32 = ipc::mailbox_create(owner, tag) as u32;

    //it is supposed to fail since we are assigning an invalid pointer to the msg_buffer
    if ipc::mailbox_write(
        ombxid,
        msg_buffer,
        core::mem::size_of_val(&msg) as u64,
    ) < 0
    {
        nanvix::log!("Failed to write to a mailbox!");

        if ipc::mailbox_unlink(ombxid) != 0 {
            nanvix::log!("Failed to remove mailbox!");
            return false;
        }
        return true;
    }

    nanvix::log!("Error: Memory Violation in Writing Operation!");
    false
}

//Fault injection testing
fn do_mailbox_read_invalid_pointer() -> bool {
    let owner: u32 = 2;
    let tag: u32 = 1;
    let msg: [u32; 16] =
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut msg_buffer = &msg as *const u32;
    let ombxid: u32 = ipc::mailbox_create(owner, tag) as u32;

    if ipc::mailbox_write(
        ombxid,
        msg_buffer,
        core::mem::size_of_val(&msg) as u64,
    ) < 0
    {
        nanvix::log!("Failed to write to a mailbox!");
        return true;
    }

    msg_buffer = 0xFBADBEEF as *const u32;
    //it is supposed to fail since we altered the msg_buffer pointer previously
    if ipc::mailbox_read(
        ombxid,
        msg_buffer,
        core::mem::size_of_val(&msg) as u64,
    ) < 0
    {
        nanvix::log!("Failed to read from a mailbox!");

        if ipc::mailbox_unlink(ombxid) != 0 {
            nanvix::log!("Failed to remove mailbox!");
            return false;
        }
        return true;
    }

    nanvix::log!("Error: Memory Violation in Reading Operation!");
    false
}

//Fault injection testing
fn do_mailbox_write_invalid_size() -> bool {
    let owner: u32 = 2;
    let tag: u32 = 1;
    let msg: [u32; 2] = [1, 2];
    let msg_buffer = &msg as *const u32;
    let sz: u32 = ipc::MAILBOX_MESSAGE_SIZE + 1;
    let ombxid: u32 = ipc::mailbox_create(owner, tag) as u32;

    //it is supposed to fail since the parameter size will be greater than the MAXIMUM message size
    // correct: sz = 8 bytes
    if ipc::mailbox_write(ombxid, msg_buffer, sz as u64) < 0 {
        nanvix::log!("Failed to write to a mailbox!");

        if ipc::mailbox_unlink(ombxid) != 0 {
            nanvix::log!("Failed to remove mailbox!");
            return false;
        }
        return true;
    }

    nanvix::log!("Error: Memory Violation in Writing Operation!");
    false
}

//==============================================================================
// Public Standalone Functions
//==============================================================================

///
/// **Description**
///
/// Tests kernel calls in the inter-process communication facility.
///
pub fn test() {
    crate::test!(check_sizes());
    crate::test!(test_semget_call());
    crate::test!(test_semctl_call());
    crate::test!(test_semop_call());
    crate::test!(do_mailbox_open_close_happy_path_testing());
    crate::test!(do_mailbox_create_happy_path_testing());
    crate::test!(do_mailbox_write_read_happy_path_testing());
    crate::test!(do_mailbox_create_open_invalid_owner());
    crate::test!(do_mailbox_write_invalid_pointer());
    crate::test!(do_mailbox_read_invalid_pointer());
    crate::test!(do_mailbox_write_invalid_size());
}
