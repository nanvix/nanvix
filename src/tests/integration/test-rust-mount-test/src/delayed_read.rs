// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Delayed HostFS pull regression test.

use ::core::time::Duration;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        RequestToken,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    safe::{
        FileSystem,
        FileSystemPath,
        FileSystemPermissions,
        RegularFile,
        RegularFileOffset,
        RegularFileOpenFlags,
        RegularFileSeekWhence,
    },
    unistd::message::{
        ReadRequest,
        WriteRequest,
    },
};

/// Verifies that a read whose pull is registered late receives an empty transfer and an error.
pub fn test() -> Result<(), Error> {
    const DATA: &[u8] = b"delayed-read";
    const DELIVERY_TIMEOUT: Duration = Duration::from_secs(1);

    let pathname: FileSystemPath = FileSystemPath::new("/mnt/test-delayed-read.txt")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);
    {
        let mut file: RegularFile = FileSystem::create_regular_file(&pathname, Some(permissions))?;
        let written: usize = file.write(DATA)?;
        if written != DATA.len() {
            panic!("delayed read setup wrote {written} of {} bytes", DATA.len());
        }
    }

    {
        let file: RegularFile =
            FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None)?;
        let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
        let token: RequestToken = RequestToken::allocate(tid, ProcessIdentifier::VFSD)?;
        let mut request: Message = ReadRequest::build(
            tid,
            file.as_raw_fd(),
            1,
            ::syscall::VFS_DESTINATION,
            ::syscall::VFS_MESSAGE_TYPE,
        );
        token.identifier().write_to(&mut request);
        ::sys::kcall::ipc::__kcall_send(&request)?;

        ::sys::kcall::pm::__kcall_sleep(DELIVERY_TIMEOUT)?;

        let mut delayed_byte: [u8; 1] = [0u8; 1];
        let pulled: usize = ::sys::kcall::ipc::__kcall_pull_tagged(
            ::syscall::VFS_PUSH_PULL_PID,
            ::syscall::VFS_PUSH_PULL_TID,
            &mut delayed_byte,
            token.identifier(),
        )?;
        if pulled != 0 {
            panic!("delayed HostFS pull returned {pulled} bytes instead of an empty transfer");
        }

        let response: Message =
            token.receive_response_with(::sys::kcall::ipc::__kcall_recv, |_| {}, |_, _, _| {})?;
        let status: i32 = response.status;
        if status != i32::from(ErrorCode::OperationTimedOut) {
            panic!("delayed HostFS read returned status {status}");
        }

        let mut first_byte: [u8; 1] = [0u8; 1];
        let read: usize = file.read(&mut first_byte)?;
        if read != 1 || first_byte[0] != DATA[0] {
            panic!("timed-out HostFS read advanced the shared file offset");
        }
    }

    {
        let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
        let token: RequestToken = RequestToken::allocate(tid, ProcessIdentifier::VFSD)?;
        let mut request: Message = ReadRequest::build(
            tid,
            i32::MAX,
            1,
            ::syscall::VFS_DESTINATION,
            ::syscall::VFS_MESSAGE_TYPE,
        );
        token.identifier().write_to(&mut request);
        ::sys::kcall::ipc::__kcall_send(&request)?;

        ::sys::kcall::pm::__kcall_sleep(DELIVERY_TIMEOUT)?;

        let mut byte: [u8; 1] = [0u8; 1];
        let pulled: usize = ::sys::kcall::ipc::__kcall_pull_tagged(
            ::syscall::VFS_PUSH_PULL_PID,
            ::syscall::VFS_PUSH_PULL_TID,
            &mut byte,
            token.identifier(),
        )?;
        if pulled != 0 {
            panic!("invalid-fd read returned {pulled} bytes instead of an empty transfer");
        }

        let response: Message =
            token.receive_response_with(::sys::kcall::ipc::__kcall_recv, |_| {}, |_, _, _| {})?;
        let status: i32 = response.status;
        if status != i32::from(ErrorCode::BadFile) {
            panic!("delayed invalid-fd read returned status {status}");
        }
    }

    let write_path: FileSystemPath = FileSystemPath::new("/mnt/test-delayed-write.txt")?;
    {
        let mut file: RegularFile =
            FileSystem::create_regular_file(&write_path, Some(permissions))?;
        let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
        let token: RequestToken = RequestToken::allocate(tid, ProcessIdentifier::VFSD)?;
        let mut request: Message = WriteRequest::build(
            tid,
            file.as_raw_fd(),
            1,
            [0u8; WriteRequest::BUFFER_SIZE],
            ::syscall::VFS_DESTINATION,
            ::syscall::VFS_MESSAGE_TYPE,
        );
        token.identifier().write_to(&mut request);
        ::sys::kcall::ipc::__kcall_send(&request)?;

        ::sys::kcall::pm::__kcall_sleep(DELIVERY_TIMEOUT)?;

        let push_error: Error = ::sys::kcall::ipc::__kcall_push_tagged_timed(
            ::syscall::VFS_PUSH_PULL_PID,
            ::syscall::VFS_PUSH_PULL_TID,
            b"x",
            token.identifier(),
            Some(DELIVERY_TIMEOUT),
        )
        .expect_err("delayed HostFS write push should time out");
        if push_error.code != ErrorCode::OperationTimedOut {
            panic!("delayed HostFS write push returned {push_error:?}");
        }

        let end: RegularFileOffset =
            file.seek(RegularFileSeekWhence::End, RegularFileOffset::from(0i32))?;
        if end != RegularFileOffset::from(0i32) {
            panic!("timed-out HostFS write changed the file size");
        }
    }

    ::syscall::safe::fs::unlink(&pathname)?;
    ::syscall::safe::fs::unlink(&write_path)?;
    ::syslog::info!(
        "mount-test: [PASS] delayed read errors release pulls without advancing offsets"
    );
    Ok(())
}
