// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Integration tests for the hostfsd handler.
//!
//! These tests exercise the full request/response cycle of [`HostFsHandler`] by encoding
//! wire-format request payloads, dispatching them through `handle_request()`, and decoding
//! the response payloads. A temporary directory is used as the sandbox root so tests are
//! hermetic and do not depend on external state.

use hostfs_api::{
    OperationId,
    *,
};
use hostfsd::HostFsHandler;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    path::PathBuf,
};
use sys::ipc::Message;
use sysapi::{
    fcntl::{
        file_access_mode::{
            O_RDONLY,
            O_RDWR,
            O_WRONLY,
        },
        file_creation_flags::{
            O_CREAT,
            O_DIRECTORY,
            O_EXCL,
            O_TRUNC,
        },
    },
    unistd::file_seek::{
        SEEK_END,
        SEEK_SET,
    },
};
use syscall::SystemCallMessageHeader;
use tempfile::TempDir;

//==================================================================================================
// Test Helpers
//==================================================================================================

/// Creates a handler rooted at a fresh temporary directory and returns both.
fn setup() -> (HostFsHandler, TempDir) {
    let tmp: TempDir = TempDir::new().expect("failed to create temp dir");
    let handler: HostFsHandler =
        HostFsHandler::new(tmp.path().to_path_buf()).expect("failed to create handler");
    (handler, tmp)
}

/// Builds an Open request payload for the given path, flags, and mode.
fn make_open_request(path: &str, flags: i32, mode: u32) -> [u8; Message::PAYLOAD_SIZE] {
    let req: OpenRequest = OpenRequest::from_path(flags, mode, path.as_bytes())
        .expect("test path fits in MAX_INLINE_PATH_LEN");
    req.serialize(
        SystemCallMessageHeader::HostFsOpenRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds a Close request payload.
fn make_close_request(fd: i32) -> [u8; Message::PAYLOAD_SIZE] {
    let req: CloseRequest = CloseRequest { fd };
    req.serialize(
        SystemCallMessageHeader::HostFsCloseRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds a Read request payload.
fn make_read_request(fd: i32, count: u32, offset: i64) -> [u8; Message::PAYLOAD_SIZE] {
    let req: ReadRequest = ReadRequest { fd, count, offset };
    req.serialize(
        SystemCallMessageHeader::HostFsReadRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds a Write request payload.
fn make_write_request(fd: i32, data: &[u8], offset: i64) -> [u8; Message::PAYLOAD_SIZE] {
    let data_len: u16 = data.len().min(MAX_INLINE_WRITE_DATA) as u16;
    let mut data_arr: [u8; MAX_INLINE_WRITE_DATA] = [0u8; MAX_INLINE_WRITE_DATA];
    data_arr[..data_len as usize].copy_from_slice(&data[..data_len as usize]);

    let req: WriteRequest = WriteRequest {
        fd,
        count: data_len as u32,
        offset,
        data_len,
        data: data_arr,
    };
    req.serialize(
        SystemCallMessageHeader::HostFsWriteRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds a Stat request payload.
fn make_stat_request(fd: i32) -> [u8; Message::PAYLOAD_SIZE] {
    let req: StatRequest = StatRequest { fd };
    req.serialize(
        SystemCallMessageHeader::HostFsStatRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds a Mkdir request payload.
fn make_mkdir_request(path: &str, mode: u32) -> [u8; Message::PAYLOAD_SIZE] {
    let req: MkdirRequest = MkdirRequest::from_path(mode, path.as_bytes())
        .expect("test path fits in MAX_INLINE_PATH_LEN");
    req.serialize(
        SystemCallMessageHeader::HostFsMkdirRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds an Unlink request payload.
fn make_unlink_request(path: &str) -> [u8; Message::PAYLOAD_SIZE] {
    let req: UnlinkRequest =
        UnlinkRequest::from_path(path.as_bytes()).expect("test path fits in MAX_INLINE_PATH_LEN");
    req.serialize(
        SystemCallMessageHeader::HostFsUnlinkRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds a Rmdir request payload.
fn make_rmdir_request(path: &str) -> [u8; Message::PAYLOAD_SIZE] {
    let req: RmdirRequest =
        RmdirRequest::from_path(path.as_bytes()).expect("test path fits in MAX_INLINE_PATH_LEN");
    req.serialize(
        SystemCallMessageHeader::HostFsRmdirRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds a Rename request payload.
fn make_rename_request(old_path: &str, new_path: &str) -> [u8; Message::PAYLOAD_SIZE] {
    let req: RenameRequest = RenameRequest::from_paths(old_path.as_bytes(), new_path.as_bytes())
        .expect("test paths fit in MAX_INLINE_PATH_LEN");
    req.serialize(
        SystemCallMessageHeader::HostFsRenameRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds an Lseek request payload.
fn make_lseek_request(fd: i32, offset: i64, whence: i32) -> [u8; Message::PAYLOAD_SIZE] {
    let req: LseekRequest = LseekRequest { fd, offset, whence };
    req.serialize(
        SystemCallMessageHeader::HostFsLseekRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds a Truncate request payload.
fn make_truncate_request(fd: i32, length: i64) -> [u8; Message::PAYLOAD_SIZE] {
    let req: TruncateRequest = TruncateRequest { fd, length };
    req.serialize(
        SystemCallMessageHeader::HostFsTruncateRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds a Flush request payload.
fn make_flush_request(fd: i32) -> [u8; Message::PAYLOAD_SIZE] {
    let req: FlushRequest = FlushRequest { fd };
    req.serialize(
        SystemCallMessageHeader::HostFsFlushRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds a ReadDir request payload.
fn make_readdir_request(fd: i32) -> [u8; Message::PAYLOAD_SIZE] {
    make_readdir_request_at(fd, 0)
}

/// Builds a ReadDir request payload with a specified offset.
fn make_readdir_request_at(fd: i32, offset: u32) -> [u8; Message::PAYLOAD_SIZE] {
    let req: ReadDirRequest = ReadDirRequest {
        fd,
        _reserved: 0,
        offset,
    };
    req.serialize(
        SystemCallMessageHeader::HostFsReadDirRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Opens a file via the handler and returns the remote FD.
fn open_file(handler: &mut HostFsHandler, path: &str, flags: i32) -> i32 {
    let payload: [u8; Message::PAYLOAD_SIZE] = make_open_request(path, flags, 0o666);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: OpenResponse = OpenResponse::decode(&response);
    resp.fd
}

//==================================================================================================
// Tests: File Open/Close
//==================================================================================================

#[test]
fn test_open_and_close_existing_file() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("hello.txt"), b"world").unwrap();

    // Open for reading.
    let fd: i32 = open_file(&mut handler, "hello.txt", O_RDONLY);
    assert!(fd > 0, "expected positive fd, got {fd}");

    // Close.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_close_request(fd);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    // Status starts at HOSTFS_DATA_START (after header[2] + op_id[4]).
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "close should succeed");
}

#[test]
fn test_open_nonexistent_file_fails() {
    let (mut handler, _tmp) = setup();

    let fd: i32 = open_file(&mut handler, "does-not-exist.txt", O_RDONLY);
    assert!(fd < 0, "expected negative fd for nonexistent file, got {fd}");
}

#[test]
fn test_open_create_flag() {
    let (mut handler, tmp) = setup();

    let fd: i32 = open_file(&mut handler, "new-file.txt", O_WRONLY | O_CREAT | O_TRUNC);
    assert!(fd > 0, "expected positive fd for created file, got {fd}");

    // Verify file was actually created on disk.
    assert!(tmp.path().join("new-file.txt").exists());
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(tmp.path().join("new-file.txt"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o666,
        "open should apply the requested creation mode"
    );

    let exclusive_fd: i32 = open_file(&mut handler, "new-file.txt", O_WRONLY | O_CREAT | O_EXCL);
    assert!(exclusive_fd < 0, "exclusive creation of an existing file should fail");

    #[cfg(unix)]
    {
        let payload: [u8; Message::PAYLOAD_SIZE] =
            make_open_request("new-file.txt", O_WRONLY | O_CREAT, 0o600);
        let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
        let existing_fd: i32 = OpenResponse::decode(&response).fd;
        assert!(existing_fd > 0, "opening an existing file with O_CREAT should succeed");
        assert_eq!(
            fs::metadata(tmp.path().join("new-file.txt"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o666,
            "open should preserve the mode of an existing file"
        );
        handler
            .handle_request(&make_close_request(existing_fd))
            .unwrap();
    }

    // Close.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_close_request(fd);
    handler.handle_request(&payload).unwrap();
}

#[test]
fn test_close_invalid_fd_fails() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_close_request(999);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, HOSTFS_ERR_IO, "close of invalid fd should fail");
}

//==================================================================================================
// Tests: Read/Write
//==================================================================================================

#[test]
fn test_read_file_contents() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("data.txt"), b"hello hostfsd").unwrap();

    let fd: i32 = open_file(&mut handler, "data.txt", O_RDONLY);
    assert!(fd > 0);

    let payload: [u8; Message::PAYLOAD_SIZE] = make_read_request(fd, 42, -1);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: ReadResponse = ReadResponse::decode(&response);
    assert_eq!(resp.bytes_read, 13);
    assert_eq!(&resp.data[..13], b"hello hostfsd");
}

#[test]
fn test_write_and_read_back() {
    let (mut handler, tmp) = setup();

    // Create and open for write.
    let fd: i32 = open_file(&mut handler, "output.txt", O_WRONLY | O_CREAT | O_TRUNC);
    assert!(fd > 0);

    // Write data.
    let write_data: &[u8] = b"test-data-12345";
    let payload: [u8; Message::PAYLOAD_SIZE] = make_write_request(fd, write_data, -1);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: WriteResponse = WriteResponse::decode(&response);
    assert_eq!(resp.bytes_written, write_data.len() as i32);

    // Close.
    handler.handle_request(&make_close_request(fd)).unwrap();

    // Verify on disk.
    let contents: Vec<u8> = fs::read(tmp.path().join("output.txt")).unwrap();
    assert_eq!(contents, write_data);
}

#[test]
fn test_read_at_offset() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("offset.txt"), b"abcdefghij").unwrap();

    let fd: i32 = open_file(&mut handler, "offset.txt", O_RDONLY);
    assert!(fd > 0);

    // Read 5 bytes starting at offset 3.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_read_request(fd, 5, 3);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: ReadResponse = ReadResponse::decode(&response);
    assert_eq!(resp.bytes_read, 5);
    assert_eq!(&resp.data[..5], b"defgh");
}

#[test]
fn test_read_invalid_fd() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_read_request(999, 10, -1);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: ReadResponse = ReadResponse::decode(&response);
    assert_eq!(resp.bytes_read, -1);
}

/// Verifies the file-offset sharing contract that makes `fork()` work over hostfs.
///
/// After `fork()`, the guest's parent and child share a single remote descriptor in hostfsd's
/// (global) FD table, backed by one host `File`. Because `vfsd` forwards ordinary reads with a
/// "use current position" offset (`-1`), the file position lives in that one `File` and advances on
/// every read regardless of which guest process issued it. This test models that contract at the
/// daemon boundary: three sequential current-position reads on one descriptor walk three disjoint
/// windows of a deterministic ramp.
#[test]
fn test_fork_shared_offset_across_descriptor() {
    let (mut handler, tmp) = setup();

    // Deterministic ramp: byte i holds value i.
    let ramp: Vec<u8> = (0u8..64).collect();
    fs::write(tmp.path().join("forkfd.dat"), &ramp).unwrap();

    let fd: i32 = open_file(&mut handler, "forkfd.dat", O_RDONLY);
    assert!(fd > 0, "expected positive fd, got {fd}");

    // Window 1 (parent prelude): bytes 0..16, advancing the shared offset to 16.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_read_request(fd, 16, -1);
    let resp: ReadResponse = ReadResponse::decode(&handler.handle_request(&payload).unwrap());
    assert_eq!(resp.bytes_read, 16);
    assert_eq!(&resp.data[..16], &ramp[0..16]);

    // Window 2 (child chunk): bytes 16..48 — continues from the offset the prelude advanced.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_read_request(fd, 32, -1);
    let resp: ReadResponse = ReadResponse::decode(&handler.handle_request(&payload).unwrap());
    assert_eq!(resp.bytes_read, 32);
    assert_eq!(&resp.data[..32], &ramp[16..48]);

    // Window 3 (parent tail): bytes 48..64 — continues from where the child chunk left off.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_read_request(fd, 16, -1);
    let resp: ReadResponse = ReadResponse::decode(&handler.handle_request(&payload).unwrap());
    assert_eq!(resp.bytes_read, 16);
    assert_eq!(&resp.data[..16], &ramp[48..64]);
}

//==================================================================================================
// Tests: Stat
//==================================================================================================

#[test]
fn test_stat_file() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("stat-me.txt"), b"0123456789").unwrap();

    let fd: i32 = open_file(&mut handler, "stat-me.txt", O_RDONLY);
    assert!(fd > 0);

    let payload: [u8; Message::PAYLOAD_SIZE] = make_stat_request(fd);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: StatResponse = StatResponse::decode(&response);
    assert_eq!(resp.status, 0);
    assert_eq!(resp.size, 10);
    assert_eq!(resp.is_dir, 0);
}

#[test]
fn test_stat_directory() {
    let (mut handler, tmp) = setup();
    fs::create_dir(tmp.path().join("mydir")).unwrap();

    // Open directory.
    let fd: i32 = open_file(&mut handler, "mydir", O_RDONLY | O_DIRECTORY);
    assert!(fd > 0);

    let payload: [u8; Message::PAYLOAD_SIZE] = make_stat_request(fd);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: StatResponse = StatResponse::decode(&response);
    assert_eq!(resp.status, 0);
    assert_eq!(resp.is_dir, 1);
}

#[test]
fn test_stat_invalid_fd() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_stat_request(999);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: StatResponse = StatResponse::decode(&response);
    assert!(resp.status < 0, "stat of invalid fd should fail");
}

//==================================================================================================
// Tests: Mkdir / Rmdir
//==================================================================================================

#[test]
fn test_mkdir_and_rmdir() {
    let (mut handler, tmp) = setup();

    // Create directory.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_mkdir_request("newdir", 0o755);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "mkdir should succeed");
    assert!(tmp.path().join("newdir").is_dir());

    // Remove directory.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_rmdir_request("newdir");
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "rmdir should succeed");
    assert!(!tmp.path().join("newdir").exists());
}

#[test]
fn test_rmdir_nonexistent_fails() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_rmdir_request("ghost");
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert!(status < 0, "rmdir of nonexistent dir should return negative error, got {status}");
}

//==================================================================================================
// Tests: Unlink
//==================================================================================================

#[test]
fn test_unlink_file() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("remove-me.txt"), b"bye").unwrap();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_unlink_request("remove-me.txt");
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0);
    assert!(!tmp.path().join("remove-me.txt").exists());
}

#[test]
fn test_unlink_nonexistent_fails() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_unlink_request("nope.txt");
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert!(status < 0, "unlink of nonexistent file should return negative error, got {status}");
}

//==================================================================================================
// Tests: Rename
//==================================================================================================

#[test]
fn test_rename_file() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("old.txt"), b"data").unwrap();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_rename_request("old.txt", "new.txt");
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0);
    assert!(!tmp.path().join("old.txt").exists());
    assert!(tmp.path().join("new.txt").exists());
    assert_eq!(fs::read(tmp.path().join("new.txt")).unwrap(), b"data");
}

//==================================================================================================
// Tests: Lseek
//==================================================================================================

#[test]
fn test_lseek_set_and_read() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("seek.txt"), b"abcdefghij").unwrap();

    // Open for reading.
    let fd: i32 = open_file(&mut handler, "seek.txt", O_RDONLY);
    assert!(fd > 0);

    // Seek to offset 5.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_lseek_request(fd, 5, SEEK_SET);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: LseekResponse = LseekResponse::decode(&response);
    assert_eq!(resp.offset, 5);

    // Read from current position (offset = -1 means current pos).
    let payload: [u8; Message::PAYLOAD_SIZE] = make_read_request(fd, 5, -1);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: ReadResponse = ReadResponse::decode(&response);
    assert_eq!(resp.bytes_read, 5);
    assert_eq!(&resp.data[..5], b"fghij");
}

#[test]
fn test_lseek_end() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("end.txt"), b"12345").unwrap();

    let fd: i32 = open_file(&mut handler, "end.txt", O_RDONLY);
    assert!(fd > 0);

    // Seek to end.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_lseek_request(fd, 0, SEEK_END);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: LseekResponse = LseekResponse::decode(&response);
    assert_eq!(resp.offset, 5);
}

#[test]
fn test_lseek_set_negative_offset_fails() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("neg.txt"), b"12345").unwrap();

    let fd: i32 = open_file(&mut handler, "neg.txt", O_RDONLY);
    assert!(fd > 0);

    // SEEK_SET with negative offset should fail.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_lseek_request(fd, -1, SEEK_SET);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let resp: LseekResponse = LseekResponse::decode(&response);
    assert_eq!(
        resp.offset, HOSTFS_ERR_INVALID as i64,
        "SEEK_SET with negative offset should fail with EINVAL"
    );
}

//==================================================================================================
// Tests: Truncate
//==================================================================================================

#[test]
fn test_truncate_file() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("trunc.txt"), b"hello world").unwrap();

    // Open for writing.
    let fd: i32 = open_file(&mut handler, "trunc.txt", O_RDWR);
    assert!(fd > 0);

    // Truncate to 5 bytes.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_truncate_request(fd, 5);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0);

    // Close and verify on disk.
    handler.handle_request(&make_close_request(fd)).unwrap();
    let contents: Vec<u8> = fs::read(tmp.path().join("trunc.txt")).unwrap();
    assert_eq!(contents, b"hello");
}

#[test]
fn test_truncate_negative_length_fails() {
    let (mut handler, _tmp) = setup();

    let fd: i32 = open_file(&mut handler, "neg-trunc.txt", O_RDWR | O_CREAT | O_TRUNC);
    assert!(fd > 0);

    // Truncate with a negative length should fail.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_truncate_request(fd, -1);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, HOSTFS_ERR_INVALID, "truncate with negative length should fail with EINVAL");
}

//==================================================================================================
// Tests: Flush
//==================================================================================================

#[test]
fn test_flush_succeeds() {
    let (mut handler, tmp) = setup();

    let fd: i32 = open_file(&mut handler, "flush.txt", O_WRONLY | O_CREAT | O_TRUNC);
    assert!(fd > 0);

    // Write something.
    handler
        .handle_request(&make_write_request(fd, b"data", -1))
        .unwrap();

    // Flush.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_flush_request(fd);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0);

    // Verify data persisted.
    let _ = tmp; // keep tmp alive
    handler.handle_request(&make_close_request(fd)).unwrap();
    assert_eq!(fs::read(tmp.path().join("flush.txt")).unwrap(), b"data");
}

#[test]
fn test_flush_invalid_fd() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_flush_request(999);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, HOSTFS_ERR_IO);
}

//==================================================================================================
// Tests: ReadDir
//==================================================================================================

#[test]
fn test_readdir_lists_entry() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("file1.txt"), b"a").unwrap();
    fs::write(tmp.path().join("file2.txt"), b"bb").unwrap();

    // Open directory.
    let fd: i32 = open_file(&mut handler, "/", O_RDONLY | O_DIRECTORY);
    assert!(fd > 0);

    // Read one entry.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_readdir_request(fd);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let entry: ReadDirEntry = ReadDirEntry::decode(&response);
    assert!(entry.name_len > 0, "expected at least one directory entry");

    let name: &str =
        core::str::from_utf8(&entry.name[..entry.name_len as usize]).expect("valid utf8");
    // The entry should be one of the files we created.
    assert!(name == "file1.txt" || name == "file2.txt", "unexpected entry name: {name}");
}

#[test]
fn test_readdir_iterates_all_entries() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("alpha.txt"), b"a").unwrap();
    fs::write(tmp.path().join("beta.txt"), b"bb").unwrap();
    fs::write(tmp.path().join("gamma.txt"), b"ccc").unwrap();

    // Open directory.
    let fd: i32 = open_file(&mut handler, "/", O_RDONLY | O_DIRECTORY);
    assert!(fd > 0);

    // Iterate through all entries using increasing offsets.
    let mut names: Vec<String> = Vec::new();
    for offset in 0u32..10 {
        let payload: [u8; Message::PAYLOAD_SIZE] = make_readdir_request_at(fd, offset);
        let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
        let entry: ReadDirEntry = ReadDirEntry::decode(&response);
        if entry.name_len == 0 {
            break;
        }
        let name: String = core::str::from_utf8(&entry.name[..entry.name_len as usize])
            .expect("valid utf8")
            .to_string();
        names.push(name);
    }

    // We should have gotten exactly 3 entries.
    assert_eq!(names.len(), 3, "expected 3 entries, got {:?}", names);
    names.sort();
    assert_eq!(names, vec!["alpha.txt", "beta.txt", "gamma.txt"]);
}

#[test]
fn test_readdir_past_end_returns_empty() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("only.txt"), b"x").unwrap();

    let fd: i32 = open_file(&mut handler, "/", O_RDONLY | O_DIRECTORY);
    assert!(fd > 0);

    // Offset 0 should return the entry.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_readdir_request_at(fd, 0);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let entry: ReadDirEntry = ReadDirEntry::decode(&response);
    assert!(entry.name_len > 0);

    // Offset 1 should return empty (end of directory).
    let payload: [u8; Message::PAYLOAD_SIZE] = make_readdir_request_at(fd, 1);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let entry: ReadDirEntry = ReadDirEntry::decode(&response);
    assert_eq!(entry.name_len, 0, "expected end-of-directory signal");
}

#[test]
fn test_readdir_reports_directory_flag_and_size() {
    let (mut handler, tmp) = setup();
    fs::create_dir(tmp.path().join("subdir")).unwrap();
    fs::write(tmp.path().join("data.bin"), b"hello world").unwrap();

    let fd: i32 = open_file(&mut handler, "/", O_RDONLY | O_DIRECTORY);
    assert!(fd > 0);

    // Collect every entry, recording the is_dir flag and size reported by hostfsd.
    // vfsd's getdents conversion (step_getdents) consumes only is_dir to set d_type;
    // size is exercised here to confirm hostfsd reports it correctly for files.
    let mut seen: Vec<(String, bool, u64)> = Vec::new();
    for offset in 0u32..10 {
        let payload: [u8; Message::PAYLOAD_SIZE] = make_readdir_request_at(fd, offset);
        let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
        let entry: ReadDirEntry = ReadDirEntry::decode(&response);
        if entry.name_len == 0 {
            break;
        }
        let name: String = core::str::from_utf8(&entry.name[..entry.name_len as usize])
            .expect("valid utf8")
            .to_string();
        seen.push((name, entry.is_dir != 0, entry.size));
    }

    let subdir: &(String, bool, u64) = seen
        .iter()
        .find(|(n, _, _)| n == "subdir")
        .expect("subdir listed");
    assert!(subdir.1, "subdir should be flagged as a directory");

    let data: &(String, bool, u64) = seen
        .iter()
        .find(|(n, _, _)| n == "data.bin")
        .expect("data.bin listed");
    assert!(!data.1, "data.bin should not be flagged as a directory");
    assert_eq!(data.2, 11, "data.bin size should be reported");
}

#[test]
fn test_readdir_long_name_multipart_response() {
    let (mut handler, tmp) = setup();
    // A name longer than the inline `ReadDirEntry` capacity forces the multi-part
    // response path. Use a name that comfortably exceeds MAX_DIR_ENTRY_NAME_LEN.
    let long_name: String = "l".repeat(180) + ".txt";
    assert!(long_name.len() > MAX_DIR_ENTRY_NAME_LEN, "test name must exceed the inline cap");
    fs::write(tmp.path().join(&long_name), b"payload!!").unwrap();

    let fd: i32 = open_file(&mut handler, "/", O_RDONLY | O_DIRECTORY);
    assert!(fd > 0);

    let payload: [u8; Message::PAYLOAD_SIZE] = make_readdir_request_at(fd, 0);
    let first: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();

    // The entry must be returned as a `HostFsReadDirResponsePart` stream.
    let header_raw: u16 = u16::from_ne_bytes([first[0], first[1]]);
    assert_eq!(
        header_raw,
        SystemCallMessageHeader::HostFsReadDirResponsePart as u16,
        "long-name readdir must use the multi-part response header"
    );

    let body: Vec<u8> = drain_multipart_response(
        &mut handler,
        first,
        SystemCallMessageHeader::HostFsReadDirResponsePart,
    );
    let resp = hostfs_api::long_msg::deserialize_long_readdir_response(&body)
        .expect("readdir long response must deserialize");
    assert!(!resp.is_dir, "regular file should not be flagged as a directory");
    assert_eq!(resp.size, 9, "file size should be reported");
    assert_eq!(resp.name, long_name.as_bytes(), "full name must round-trip");
}

//==================================================================================================
// Tests: Sandbox Security
//==================================================================================================

#[test]
fn test_path_traversal_rejected() {
    let (mut handler, tmp) = setup();
    // Write a file outside the sandbox (in the parent of tmp).
    let outside_path: PathBuf = tmp.path().parent().unwrap().join("secret.txt");
    fs::write(&outside_path, b"secret").unwrap();

    // Try to open with path traversal.
    let fd: i32 = open_file(&mut handler, "../secret.txt", O_RDONLY);
    assert!(fd < 0, "path traversal should be rejected, got fd={fd}");

    // Clean up.
    let _ = fs::remove_file(&outside_path);
}

#[test]
fn test_absolute_path_normalized() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("abs.txt"), b"content").unwrap();

    // Leading slash should be stripped and resolved within sandbox.
    let fd: i32 = open_file(&mut handler, "/abs.txt", O_RDONLY);
    assert!(fd > 0, "absolute path should be resolved within sandbox");
}

#[test]
fn test_mkdir_path_traversal_rejected() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_mkdir_request("../../escape", 0o755);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, HOSTFS_ERR_PERMISSION, "mkdir with path traversal should fail");
}

//==================================================================================================
// Tests: Multiple Operations (workflow)
//==================================================================================================

#[test]
fn test_create_write_close_reopen_read() {
    let (mut handler, _tmp) = setup();

    // Create, write, close.
    let fd: i32 = open_file(&mut handler, "roundtrip.txt", O_WRONLY | O_CREAT | O_TRUNC);
    assert!(fd > 0);
    let resp: [u8; Message::PAYLOAD_SIZE] = handler
        .handle_request(&make_write_request(fd, b"round-trip-data", -1))
        .unwrap();
    let wr: WriteResponse = WriteResponse::decode(&resp);
    assert_eq!(wr.bytes_written, 15);
    handler.handle_request(&make_close_request(fd)).unwrap();

    // Reopen for reading.
    let fd2: i32 = open_file(&mut handler, "roundtrip.txt", O_RDONLY);
    assert!(fd2 > 0);

    // Read back.
    let resp: [u8; Message::PAYLOAD_SIZE] = handler
        .handle_request(&make_read_request(fd2, 42, -1))
        .unwrap();
    let rd: ReadResponse = ReadResponse::decode(&resp);
    assert_eq!(rd.bytes_read, 15);
    assert_eq!(&rd.data[..15], b"round-trip-data");
}

#[test]
fn test_nested_directory_workflow() {
    let (mut handler, tmp) = setup();

    // Create nested directories.
    handler
        .handle_request(&make_mkdir_request("a", 0o755))
        .unwrap();
    handler
        .handle_request(&make_mkdir_request("a/b", 0o755))
        .unwrap();
    assert!(tmp.path().join("a/b").is_dir());

    // Create a file inside nested dir.
    let fd: i32 = open_file(&mut handler, "a/b/deep.txt", O_WRONLY | O_CREAT | O_TRUNC);
    assert!(fd > 0);
    handler
        .handle_request(&make_write_request(fd, b"deep", -1))
        .unwrap();
    handler.handle_request(&make_close_request(fd)).unwrap();

    // Verify.
    assert_eq!(fs::read(tmp.path().join("a/b/deep.txt")).unwrap(), b"deep");

    // Clean up: unlink file, rmdir b, rmdir a.
    let resp: [u8; Message::PAYLOAD_SIZE] = handler
        .handle_request(&make_unlink_request("a/b/deep.txt"))
        .unwrap();
    let ds: usize = HOSTFS_DATA_START;
    assert_eq!(i32::from_le_bytes(resp[ds..ds + 4].try_into().unwrap()), 0);
    let resp: [u8; Message::PAYLOAD_SIZE] =
        handler.handle_request(&make_rmdir_request("a/b")).unwrap();
    assert_eq!(i32::from_le_bytes(resp[ds..ds + 4].try_into().unwrap()), 0);
    let resp: [u8; Message::PAYLOAD_SIZE] =
        handler.handle_request(&make_rmdir_request("a")).unwrap();
    assert_eq!(i32::from_le_bytes(resp[ds..ds + 4].try_into().unwrap()), 0);
    assert!(!tmp.path().join("a").exists());
}

//==================================================================================================
// Tests: Operation ID echoing
//==================================================================================================

#[test]
fn test_op_id_echoed_in_response() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("opid.txt"), b"test").unwrap();

    // Build a request with a specific op_id.
    let mut payload: [u8; Message::PAYLOAD_SIZE] = make_open_request("opid.txt", O_RDONLY, 0);
    set_op_id(&mut payload, OperationId::from_raw(42));

    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    assert_eq!(
        get_op_id(&response),
        OperationId::from_raw(42),
        "op_id should be echoed in response"
    );

    let resp: OpenResponse = OpenResponse::decode(&response);
    assert!(resp.fd > 0, "open should succeed");

    // Close with a different op_id.
    let mut close_payload: [u8; Message::PAYLOAD_SIZE] = make_close_request(resp.fd);
    set_op_id(&mut close_payload, OperationId::from_raw(99));
    let close_response: [u8; Message::PAYLOAD_SIZE] =
        handler.handle_request(&close_payload).unwrap();
    assert_eq!(
        get_op_id(&close_response),
        OperationId::from_raw(99),
        "op_id should be echoed for close"
    );
}

#[test]
fn test_op_id_zero_is_valid() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("zero.txt"), b"z").unwrap();

    let mut payload: [u8; Message::PAYLOAD_SIZE] = make_open_request("zero.txt", O_RDONLY, 0);
    set_op_id(&mut payload, OperationId::from_raw(0));

    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload).unwrap();
    assert_eq!(get_op_id(&response), OperationId::from_raw(0), "op_id 0 should be echoed");
}

//==================================================================================================
// Multi-part Request Tests
//==================================================================================================

use syscall::message::SystemCallMessagePart;

/// Builds a raw IKC payload representing a `SystemCallMessagePart` with the given header.
fn make_part_payload(
    header: SystemCallMessageHeader,
    total_parts: u16,
    part_number: u16,
    payload_size: u8,
    part_payload: &[u8],
) -> [u8; Message::PAYLOAD_SIZE] {
    let mut buf: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    // Bytes [0..2]: header.
    let h: u16 = header as u16;
    buf[0] = h as u8;
    buf[1] = (h >> 8) as u8;
    // Bytes [2..4]: total_parts.
    buf[2] = total_parts as u8;
    buf[3] = (total_parts >> 8) as u8;
    // Bytes [4..6]: part_number.
    buf[4] = part_number as u8;
    buf[5] = (part_number >> 8) as u8;
    // Byte [6]: payload_size.
    buf[6] = payload_size;
    // Bytes [7..]: payload.
    let copy_len: usize = part_payload.len().min(SystemCallMessagePart::PAYLOAD_SIZE);
    buf[7..7 + copy_len].copy_from_slice(&part_payload[..copy_len]);
    buf
}

/// Serializes a long OPEN request into multi-part IKC payloads.
fn make_long_open_parts(
    path: &str,
    flags: i32,
    mode: u32,
    op_id: OperationId,
) -> Vec<[u8; Message::PAYLOAD_SIZE]> {
    let path_bytes: &[u8] = path.as_bytes();
    let path_len: u16 = path_bytes.len() as u16;

    // Serialize: [op_id:4][flags:4][mode:4][path_len:2][path:N]
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&op_id.to_le_bytes());
    data.extend_from_slice(&flags.to_le_bytes());
    data.extend_from_slice(&mode.to_le_bytes());
    data.extend_from_slice(&path_len.to_le_bytes());
    data.extend_from_slice(path_bytes);

    let chunk_size: usize = SystemCallMessagePart::PAYLOAD_SIZE;
    let num_parts: u16 = data.len().div_ceil(chunk_size) as u16;
    let header: SystemCallMessageHeader = SystemCallMessageHeader::HostFsOpenRequestPart;

    let mut parts: Vec<[u8; Message::PAYLOAD_SIZE]> = Vec::new();
    for (i, chunk) in data.chunks(chunk_size).enumerate() {
        parts.push(make_part_payload(header, num_parts, i as u16, chunk.len() as u8, chunk));
    }
    parts
}

#[test]
fn test_long_open_single_part() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("short.txt"), b"data").unwrap();

    let parts = make_long_open_parts("short.txt", O_RDONLY, 0, OperationId::from_raw(7));
    assert_eq!(parts.len(), 1, "short path should fit in one part");

    let response = handler.handle_request(&parts[0]);
    assert!(response.is_some(), "single-part request should produce a response");
    let response = response.unwrap();
    assert_eq!(get_op_id(&response), OperationId::from_raw(7));
    let resp: OpenResponse = OpenResponse::decode(&response);
    assert!(resp.fd > 0, "open should succeed");
}

#[cfg(unix)]
#[test]
fn test_long_open_create_preserves_existing_mode() {
    let (mut handler, tmp) = setup();
    let host_path: PathBuf = tmp.path().join("existing.txt");
    fs::write(&host_path, b"data").unwrap();
    fs::set_permissions(&host_path, fs::Permissions::from_mode(0o640)).unwrap();

    let parts: Vec<[u8; Message::PAYLOAD_SIZE]> =
        make_long_open_parts("existing.txt", O_WRONLY | O_CREAT, 0o600, OperationId::from_raw(8));
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&parts[0]).unwrap();
    let resp: OpenResponse = OpenResponse::decode(&response);
    assert!(resp.fd > 0, "opening an existing file with O_CREAT should succeed");
    assert_eq!(
        fs::metadata(&host_path).unwrap().permissions().mode() & 0o777,
        0o640,
        "long open should preserve the mode of an existing file"
    );
}

#[test]
fn test_long_open_multi_part() {
    let (mut handler, tmp) = setup();
    // Create a deeply nested path that requires multiple parts.
    let long_name: String = "a".repeat(50);
    fs::create_dir(tmp.path().join(&long_name)).unwrap();
    let file_name: String = format!("{}/file.txt", long_name);
    fs::write(tmp.path().join(&file_name), b"hello").unwrap();

    let parts = make_long_open_parts(&file_name, O_RDONLY, 0, OperationId::from_raw(42));
    assert!(parts.len() >= 2, "long path should require multiple parts, got {}", parts.len());

    // Feed intermediate parts — should return None.
    for part in &parts[..parts.len() - 1] {
        let result = handler.handle_request(part);
        assert!(result.is_none(), "intermediate part should return None");
    }

    // Feed final part — should return the response.
    let response = handler.handle_request(parts.last().unwrap());
    assert!(response.is_some(), "final part should produce a response");
    let response = response.unwrap();
    assert_eq!(get_op_id(&response), OperationId::from_raw(42));
    let resp: OpenResponse = OpenResponse::decode(&response);
    assert!(resp.fd > 0, "open should succeed for long path");
}

#[test]
fn test_long_open_nonexistent_file() {
    let (mut handler, _tmp) = setup();

    let parts = make_long_open_parts("does-not-exist.txt", O_RDONLY, 0, OperationId::from_raw(10));
    let response = handler.handle_request(&parts[0]).unwrap();
    assert_eq!(get_op_id(&response), OperationId::from_raw(10));
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert!(status < 0, "open of nonexistent file should fail");
}

#[test]
fn test_assembler_rejects_zero_total_parts() {
    let (mut handler, _tmp) = setup();

    let payload = make_part_payload(
        SystemCallMessageHeader::HostFsOpenRequestPart,
        0, // total_parts = 0
        0,
        10,
        &[0u8; 10],
    );
    let response = handler.handle_request(&payload);
    // Should return an error response (not None).
    assert!(response.is_some(), "zero total_parts should produce error response");
    let response = response.unwrap();
    assert_eq!(get_op_id(&response), OperationId::INVALID);
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, HOSTFS_ERR_INVALID);
}

#[test]
fn test_assembler_rejects_out_of_order_parts() {
    let (mut handler, _tmp) = setup();

    // Send a valid part 0 of a 3-part request. The first 4 bytes of the
    // assembled wire format are the op_id; embed a non-zero value so we can
    // verify it is echoed back in the assembler error response.
    let op_id_bytes: [u8; 4] = OperationId::from_raw(77).to_le_bytes();
    let mut part0_payload: [u8; 10] = [0u8; 10];
    part0_payload[..4].copy_from_slice(&op_id_bytes);
    let payload0 =
        make_part_payload(SystemCallMessageHeader::HostFsOpenRequestPart, 3, 0, 10, &part0_payload);
    let result = handler.handle_request(&payload0);
    assert!(result.is_none(), "first part should return None");

    // Send part 2 (skipping part 1) — out-of-order should produce error.
    let payload2 =
        make_part_payload(SystemCallMessageHeader::HostFsOpenRequestPart, 3, 2, 10, &[0u8; 10]);
    let response = handler.handle_request(&payload2);
    assert!(response.is_some(), "out-of-order part should produce error response");
    let response = response.unwrap();
    // The error response must echo the in-flight op_id recovered from part 0,
    // so vfsd can match it against its pending-op table and unblock the caller.
    assert_eq!(get_op_id(&response), OperationId::from_raw(77));
}

#[test]
fn test_assembler_rejects_header_mismatch() {
    let (mut handler, _tmp) = setup();

    // Send part 0 with OpenRequestPart header, embedding a non-zero op_id in
    // the first 4 bytes so we can verify it is echoed back on the error path.
    let op_id_bytes: [u8; 4] = OperationId::from_raw(123).to_le_bytes();
    let mut part0_payload: [u8; 10] = [0u8; 10];
    part0_payload[..4].copy_from_slice(&op_id_bytes);
    let payload0 =
        make_part_payload(SystemCallMessageHeader::HostFsOpenRequestPart, 2, 0, 10, &part0_payload);
    let result = handler.handle_request(&payload0);
    assert!(result.is_none(), "first part should return None");

    // Send part 1 with a DIFFERENT header (MkdirRequestPart) — should produce error.
    let payload1 =
        make_part_payload(SystemCallMessageHeader::HostFsMkdirRequestPart, 2, 1, 10, &[0u8; 10]);
    let response = handler.handle_request(&payload1);
    assert!(response.is_some(), "header mismatch should produce error response");
    let response = response.unwrap();
    // The error response must echo the in-flight op_id recovered from part 0
    // (recorded under the original header), not the mismatching part's header.
    assert_eq!(get_op_id(&response), OperationId::from_raw(123));
}

#[test]
fn test_assembler_rejects_stray_part_zero_mid_stream() {
    // A new part_number == 0 arriving while a multi-part stream is still in flight
    // must surface an error so the in-flight op_id can be reported back to vfsd,
    // rather than silently discarding the buffered bytes and orphaning the
    // pending op.
    let (mut handler, _tmp) = setup();

    // Begin a 3-part stream with a recoverable op_id in part 0.
    let op_id_bytes: [u8; 4] = OperationId::from_raw(99).to_le_bytes();
    let mut part0_payload: [u8; 10] = [0u8; 10];
    part0_payload[..4].copy_from_slice(&op_id_bytes);
    let payload0 =
        make_part_payload(SystemCallMessageHeader::HostFsOpenRequestPart, 3, 0, 10, &part0_payload);
    let result = handler.handle_request(&payload0);
    assert!(result.is_none(), "first part should return None");

    // Send a stray part 0 (start of a "new" stream) — should error.
    let stray_payload =
        make_part_payload(SystemCallMessageHeader::HostFsOpenRequestPart, 2, 0, 10, &[0u8; 10]);
    let response = handler.handle_request(&stray_payload);
    assert!(response.is_some(), "stray part 0 should produce error response");
    let response = response.unwrap();
    // The error response must echo the original in-flight op_id, not the
    // stray new stream's bytes.
    assert_eq!(get_op_id(&response), OperationId::from_raw(99));
}

//==================================================================================================
// Multi-part Request Tests: Unlink / Rmdir / Mkdir / Rename
//==================================================================================================

/// Splits a serialized long-message wire-format buffer into IKC parts using the given header.
fn split_into_parts(
    header: SystemCallMessageHeader,
    data: &[u8],
) -> Vec<[u8; Message::PAYLOAD_SIZE]> {
    let chunk_size: usize = SystemCallMessagePart::PAYLOAD_SIZE;
    let num_parts: u16 = data.len().div_ceil(chunk_size).max(1) as u16;
    let mut parts: Vec<[u8; Message::PAYLOAD_SIZE]> = Vec::new();
    for (i, chunk) in data.chunks(chunk_size).enumerate() {
        parts.push(make_part_payload(header, num_parts, i as u16, chunk.len() as u8, chunk));
    }
    parts
}

/// Serializes a long UNLINK request: `[op_id:4][path_len:2][path:N]`.
fn make_long_unlink_parts(path: &str, op_id: OperationId) -> Vec<[u8; Message::PAYLOAD_SIZE]> {
    let path_bytes: &[u8] = path.as_bytes();
    let path_len: u16 = path_bytes.len() as u16;
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&op_id.to_le_bytes());
    data.extend_from_slice(&path_len.to_le_bytes());
    data.extend_from_slice(path_bytes);
    split_into_parts(SystemCallMessageHeader::HostFsUnlinkRequestPart, &data)
}

/// Serializes a long RMDIR request: `[op_id:4][path_len:2][path:N]`.
fn make_long_rmdir_parts(path: &str, op_id: OperationId) -> Vec<[u8; Message::PAYLOAD_SIZE]> {
    let path_bytes: &[u8] = path.as_bytes();
    let path_len: u16 = path_bytes.len() as u16;
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&op_id.to_le_bytes());
    data.extend_from_slice(&path_len.to_le_bytes());
    data.extend_from_slice(path_bytes);
    split_into_parts(SystemCallMessageHeader::HostFsRmdirRequestPart, &data)
}

/// Serializes a long MKDIR request: `[op_id:4][mode:4][path_len:2][path:N]`.
fn make_long_mkdir_parts(
    path: &str,
    mode: u32,
    op_id: OperationId,
) -> Vec<[u8; Message::PAYLOAD_SIZE]> {
    let path_bytes: &[u8] = path.as_bytes();
    let path_len: u16 = path_bytes.len() as u16;
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&op_id.to_le_bytes());
    data.extend_from_slice(&mode.to_le_bytes());
    data.extend_from_slice(&path_len.to_le_bytes());
    data.extend_from_slice(path_bytes);
    split_into_parts(SystemCallMessageHeader::HostFsMkdirRequestPart, &data)
}

/// Serializes a long RENAME request: `[op_id:4][old_path_len:2][new_path_len:2][old_path][new_path]`.
fn make_long_rename_parts(
    old_path: &str,
    new_path: &str,
    op_id: OperationId,
) -> Vec<[u8; Message::PAYLOAD_SIZE]> {
    let old_bytes: &[u8] = old_path.as_bytes();
    let new_bytes: &[u8] = new_path.as_bytes();
    let old_len: u16 = old_bytes.len() as u16;
    let new_len: u16 = new_bytes.len() as u16;
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&op_id.to_le_bytes());
    data.extend_from_slice(&old_len.to_le_bytes());
    data.extend_from_slice(&new_len.to_le_bytes());
    data.extend_from_slice(old_bytes);
    data.extend_from_slice(new_bytes);
    split_into_parts(SystemCallMessageHeader::HostFsRenameRequestPart, &data)
}

/// Feeds all parts of a multi-part request through the handler and returns the final response.
fn feed_parts(
    handler: &mut HostFsHandler,
    parts: &[[u8; Message::PAYLOAD_SIZE]],
) -> [u8; Message::PAYLOAD_SIZE] {
    assert!(!parts.is_empty(), "must have at least one part");
    for part in &parts[..parts.len() - 1] {
        let result = handler.handle_request(part);
        assert!(result.is_none(), "intermediate part should return None");
    }
    handler
        .handle_request(parts.last().unwrap())
        .expect("final part should produce a response")
}

#[test]
fn test_long_unlink_multi_part() {
    let (mut handler, tmp) = setup();
    // Use a path long enough to require multiple parts.
    let long_dir: String = "u".repeat(60);
    fs::create_dir(tmp.path().join(&long_dir)).unwrap();
    let path: String = format!("{}/victim.txt", long_dir);
    fs::write(tmp.path().join(&path), b"x").unwrap();

    let parts = make_long_unlink_parts(&path, OperationId::from_raw(11));
    assert!(parts.len() >= 2, "long unlink should require multiple parts, got {}", parts.len());

    let response = feed_parts(&mut handler, &parts);
    assert_eq!(get_op_id(&response), OperationId::from_raw(11));
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "long unlink should succeed");
    assert!(!tmp.path().join(&path).exists());
}

/// Ensures that unlinking a symbolic link via the multi-part long-request path
/// removes the link itself and leaves the target intact. Exercises the long-unlink
/// decoding code path, which is distinct from the inline one covered by
/// `test_unlink_removes_symlink_not_target`.
#[test]
fn test_long_unlink_symlink_removes_link_not_target() {
    let (mut handler, tmp) = setup();
    // Use a long parent dir so the encoded path exceeds the inline limit and the
    // request must be split across multiple parts.
    let long_dir: String = "u".repeat(60);
    fs::create_dir(tmp.path().join(&long_dir)).unwrap();
    let target_rel: String = format!("{}/target.txt", long_dir);
    let link_rel: String = format!("{}/link", long_dir);
    fs::write(tmp.path().join(&target_rel), b"keep me").unwrap();
    // Use a relative target so the link is portable within the sandbox.
    if let Err(e) = host_symlink(std::path::Path::new("target.txt"), &tmp.path().join(&link_rel)) {
        if is_privilege_error(&e) {
            println!("skipping: host cannot create symlinks ({})", e);
            return;
        }
        panic!("host_symlink failed: {}", e);
    }

    let parts = make_long_unlink_parts(&link_rel, OperationId::from_raw(13));
    assert!(parts.len() >= 2, "long unlink should require multiple parts, got {}", parts.len());

    let response = feed_parts(&mut handler, &parts);
    assert_eq!(get_op_id(&response), OperationId::from_raw(13));
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "long unlink of symlink should succeed");
    assert!(tmp.path().join(&link_rel).symlink_metadata().is_err(), "symlink should be removed");
    assert!(tmp.path().join(&target_rel).exists(), "symlink target must not be removed");
}

#[test]
fn test_long_rmdir_multi_part() {
    let (mut handler, tmp) = setup();
    let long_parent: String = "r".repeat(60);
    let inner: String = format!("{}/inner", long_parent);
    fs::create_dir(tmp.path().join(&long_parent)).unwrap();
    fs::create_dir(tmp.path().join(&inner)).unwrap();

    let parts = make_long_rmdir_parts(&inner, OperationId::from_raw(12));
    assert!(parts.len() >= 2, "long rmdir should require multiple parts, got {}", parts.len());

    let response = feed_parts(&mut handler, &parts);
    assert_eq!(get_op_id(&response), OperationId::from_raw(12));
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "long rmdir should succeed");
    assert!(!tmp.path().join(&inner).exists());
}

#[test]
fn test_long_mkdir_multi_part() {
    let (mut handler, tmp) = setup();
    let long_parent: String = "m".repeat(60);
    fs::create_dir(tmp.path().join(&long_parent)).unwrap();
    let new_dir: String = format!("{}/child", long_parent);

    let parts = make_long_mkdir_parts(&new_dir, 0o755, OperationId::from_raw(13));
    assert!(parts.len() >= 2, "long mkdir should require multiple parts, got {}", parts.len());

    let response = feed_parts(&mut handler, &parts);
    assert_eq!(get_op_id(&response), OperationId::from_raw(13));
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "long mkdir should succeed");
    assert!(tmp.path().join(&new_dir).is_dir());
}

#[test]
fn test_long_rename_multi_part() {
    let (mut handler, tmp) = setup();
    // Use two long paths so the wire format spans multiple parts.
    let long_dir: String = "n".repeat(50);
    fs::create_dir(tmp.path().join(&long_dir)).unwrap();
    let old_path: String = format!("{}/before.txt", long_dir);
    let new_path: String = format!("{}/after.txt", long_dir);
    fs::write(tmp.path().join(&old_path), b"payload").unwrap();

    let parts = make_long_rename_parts(&old_path, &new_path, OperationId::from_raw(14));
    assert!(parts.len() >= 2, "long rename should require multiple parts, got {}", parts.len());

    let response = feed_parts(&mut handler, &parts);
    assert_eq!(get_op_id(&response), OperationId::from_raw(14));
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "long rename should succeed");
    assert!(!tmp.path().join(&old_path).exists());
    assert_eq!(fs::read(tmp.path().join(&new_path)).unwrap(), b"payload");
}

//==================================================================================================
// Symlink Tests (lstat / readlink / symlink)
//==================================================================================================

/// Builds an inline Lstat request payload.
fn make_lstat_request(path: &str) -> [u8; Message::PAYLOAD_SIZE] {
    let req: LstatRequest =
        LstatRequest::from_path(path.as_bytes()).expect("test path fits in MAX_INLINE_PATH_LEN");
    req.serialize(
        SystemCallMessageHeader::HostFsLstatRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Builds an inline path-based following-stat request payload.
fn make_pathstat_request(path: &str) -> [u8; Message::PAYLOAD_SIZE] {
    let req: LstatRequest =
        LstatRequest::from_path(path.as_bytes()).expect("test path fits in MAX_INLINE_PATH_LEN");
    req.serialize(
        SystemCallMessageHeader::HostFsPathStatRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Serializes a long PATHSTAT (following stat) request: `[op_id:4][path_len:2][path:N]`.
///
/// Reuses the lstat wire body but tags the parts with the PathStat part header so the
/// handler dispatches to `handle_long_pathstat` (the multi-part following-stat path).
fn make_long_pathstat_parts(path: &str, op_id: OperationId) -> Vec<[u8; Message::PAYLOAD_SIZE]> {
    let path_bytes: &[u8] = path.as_bytes();
    let path_len: u16 = path_bytes.len() as u16;
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&op_id.to_le_bytes());
    data.extend_from_slice(&path_len.to_le_bytes());
    data.extend_from_slice(path_bytes);
    split_into_parts(SystemCallMessageHeader::HostFsPathStatRequestPart, &data)
}

/// Builds an inline Readlink request payload.
fn make_readlink_request(path: &str) -> [u8; Message::PAYLOAD_SIZE] {
    let req: ReadlinkRequest =
        ReadlinkRequest::from_path(path.as_bytes()).expect("test path fits in MAX_INLINE_PATH_LEN");
    req.serialize(
        SystemCallMessageHeader::HostFsReadlinkRequest as u16,
        OperationId::from_le_bytes([0; 4]),
    )
}

/// Serializes a long Symlink request:
/// `[op_id:4][target_len:2][linkpath_len:2][target:N][linkpath:M]`.
fn make_long_symlink_parts(
    target: &str,
    linkpath: &str,
    op_id: OperationId,
) -> Vec<[u8; Message::PAYLOAD_SIZE]> {
    let t: &[u8] = target.as_bytes();
    let l: &[u8] = linkpath.as_bytes();
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&op_id.to_le_bytes());
    data.extend_from_slice(&(t.len() as u16).to_le_bytes());
    data.extend_from_slice(&(l.len() as u16).to_le_bytes());
    data.extend_from_slice(t);
    data.extend_from_slice(l);
    split_into_parts(SystemCallMessageHeader::HostFsSymlinkRequestPart, &data)
}

/// Creates a symbolic link on the host. Returns `Ok(())` on success or an error.
///
/// On Windows this requires Developer Mode (or admin); if not enabled, returns
/// `Err(_)` with `ErrorKind::PermissionDenied`. Tests that depend on host symlink
/// creation should skip gracefully in that case.
fn host_symlink(target: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        // Use symlink_file by default; tests that need dir links create them explicitly.
        std::os::windows::fs::symlink_file(target, link)
    }
}

/// Creates a symbolic link to a directory on the host.
///
/// On Windows, directory symlinks are a distinct object type from file symlinks
/// (`symlink_dir` vs `symlink_file`); using the file variant for a directory target
/// produces a link the OS will not traverse. On Unix there is a single `symlink(2)`.
fn host_symlink_dir(target: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link)
    }
}

/// Returns true if the OS refused to create the symlink due to insufficient privileges.
/// On Windows non-Developer-Mode setups, symlink creation fails with
/// `ERROR_PRIVILEGE_NOT_HELD` (1314); tests should skip in that case. On Unix, host
/// `symlink(2)` does not require special privileges, so this always returns `false`.
fn is_privilege_error(err: &std::io::Error) -> bool {
    #[cfg(windows)]
    {
        // ERROR_PRIVILEGE_NOT_HELD = 1314
        err.raw_os_error() == Some(1314)
    }
    #[cfg(not(windows))]
    {
        let _ = err;
        false
    }
}

#[test]
fn test_lstat_regular_file() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("file.txt"), b"hello").unwrap();
    let req = make_lstat_request("file.txt");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = LstatResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, 0, "lstat should succeed");
    assert_eq!(r.kind, file_kind::REGULAR);
    assert_eq!(r.size, 5);
}

#[test]
fn test_lstat_directory() {
    let (mut handler, tmp) = setup();
    fs::create_dir(tmp.path().join("dir")).unwrap();
    let req = make_lstat_request("dir");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = LstatResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, 0, "lstat should succeed");
    assert_eq!(r.kind, file_kind::DIRECTORY);
}

#[test]
fn test_lstat_nonexistent_fails() {
    let (mut handler, _tmp) = setup();
    let req = make_lstat_request("missing");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = LstatResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, HOSTFS_ERR_NOT_FOUND);
}

#[test]
fn test_lstat_symlink_does_not_follow() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("target.txt"), b"contents").unwrap();
    if let Err(e) = host_symlink(std::path::Path::new("target.txt"), &tmp.path().join("link")) {
        if is_privilege_error(&e) {
            println!("skipping: host cannot create symlinks ({})", e);
            return;
        }
        panic!("host_symlink failed: {}", e);
    }
    let req = make_lstat_request("link");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = LstatResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, 0, "lstat should succeed");
    assert_eq!(r.kind, file_kind::SYMLINK, "lstat must NOT follow the symlink");
}

#[test]
fn test_pathstat_regular_file() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("file.txt"), b"hello").unwrap();
    let req = make_pathstat_request("file.txt");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = LstatResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, 0, "pathstat should succeed");
    assert_eq!(r.kind, file_kind::REGULAR);
    assert_eq!(r.size, 5);
}

#[test]
fn test_pathstat_directory() {
    let (mut handler, tmp) = setup();
    fs::create_dir(tmp.path().join("dir")).unwrap();
    let req = make_pathstat_request("dir");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = LstatResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, 0, "pathstat should succeed");
    assert_eq!(r.kind, file_kind::DIRECTORY);
}

#[test]
fn test_pathstat_nonexistent_fails() {
    let (mut handler, _tmp) = setup();
    let req = make_pathstat_request("missing");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = LstatResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, HOSTFS_ERR_NOT_FOUND);
}

#[test]
fn test_pathstat_symlink_follows_to_target() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("target.txt"), b"contents").unwrap();
    if let Err(e) = host_symlink(std::path::Path::new("target.txt"), &tmp.path().join("link")) {
        if is_privilege_error(&e) {
            println!("skipping: host cannot create symlinks ({})", e);
            return;
        }
        panic!("host_symlink failed: {}", e);
    }
    let req = make_pathstat_request("link");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = LstatResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, 0, "pathstat should succeed");
    assert_eq!(
        r.kind,
        file_kind::REGULAR,
        "pathstat must FOLLOW the symlink and report the target's kind"
    );
    assert_eq!(r.size, 8, "size should be the target file's size");
}

#[test]
fn test_pathstat_symlink_to_directory_follows() {
    let (mut handler, tmp) = setup();
    fs::create_dir(tmp.path().join("realdir")).unwrap();
    if let Err(e) = host_symlink_dir(std::path::Path::new("realdir"), &tmp.path().join("dlink")) {
        if is_privilege_error(&e) {
            println!("skipping: host cannot create symlinks ({})", e);
            return;
        }
        panic!("host_symlink failed: {}", e);
    }
    let req = make_pathstat_request("dlink");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = LstatResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, 0, "pathstat should succeed");
    assert_eq!(
        r.kind,
        file_kind::DIRECTORY,
        "pathstat must follow a symlink that points at a directory"
    );
}

#[test]
fn test_pathstat_dangling_symlink_fails() {
    let (mut handler, tmp) = setup();
    if let Err(e) =
        host_symlink(std::path::Path::new("does_not_exist"), &tmp.path().join("dangling"))
    {
        if is_privilege_error(&e) {
            println!("skipping: host cannot create symlinks ({})", e);
            return;
        }
        panic!("host_symlink failed: {}", e);
    }
    let req = make_pathstat_request("dangling");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = LstatResponse::decode(&resp).expect("decode");
    assert_eq!(
        r.status, HOSTFS_ERR_NOT_FOUND,
        "following a dangling symlink must surface the target's ENOENT"
    );
}

#[test]
fn test_pathstat_long_path_multipart_follows_symlink() {
    let (mut handler, tmp) = setup();
    // Use a link name long enough that the request body exceeds the inline limit,
    // forcing the multi-part assembler / `handle_long_pathstat` dispatch path.
    let link_name: &str = "long-pathstat-link-name-padding-AAAAAAAAAAAA";
    assert!(
        link_name.len() > MAX_INLINE_PATH_LEN,
        "link name must exceed the inline path limit to exercise the multi-part path"
    );
    fs::write(tmp.path().join("target.txt"), b"contents").unwrap();
    if let Err(e) = host_symlink(std::path::Path::new("target.txt"), &tmp.path().join(link_name)) {
        if is_privilege_error(&e) {
            println!("skipping: host cannot create symlinks ({})", e);
            return;
        }
        panic!("host_symlink failed: {}", e);
    }
    let parts = make_long_pathstat_parts(link_name, OperationId::from_raw(77));
    assert!(parts.len() >= 2, "long pathstat should require multiple parts, got {}", parts.len());
    let response = feed_parts(&mut handler, &parts);
    assert_eq!(get_op_id(&response), OperationId::from_raw(77));
    let r = LstatResponse::decode(&response).expect("decode");
    assert_eq!(r.status, 0, "long pathstat should succeed");
    assert_eq!(
        r.kind,
        file_kind::REGULAR,
        "multi-part pathstat must FOLLOW the symlink and report the target's kind"
    );
    assert_eq!(r.size, 8, "size should be the target file's size");
}

#[test]
fn test_readlink_returns_target() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("target.txt"), b"x").unwrap();
    let target_str = "target.txt";
    if let Err(e) = host_symlink(std::path::Path::new(target_str), &tmp.path().join("link")) {
        if is_privilege_error(&e) {
            println!("skipping: host cannot create symlinks ({})", e);
            return;
        }
        panic!("host_symlink failed: {}", e);
    }
    let req = make_readlink_request("link");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = ReadlinkResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, 0, "readlink should succeed");
    let got = &r.target[..r.target_len as usize];
    assert_eq!(got, target_str.as_bytes());
}

#[test]
fn test_readlink_regular_file_fails() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("file.txt"), b"x").unwrap();
    let req = make_readlink_request("file.txt");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = ReadlinkResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, HOSTFS_ERR_INVALID, "readlink on non-symlink must fail with EINVAL");
}

#[test]
fn test_readlink_nonexistent_fails() {
    let (mut handler, _tmp) = setup();
    let req = make_readlink_request("missing");
    let resp = handler.handle_request(&req).expect("response expected");
    let r = ReadlinkResponse::decode(&resp).expect("decode");
    assert_eq!(r.status, HOSTFS_ERR_NOT_FOUND);
}

#[test]
fn test_unlink_removes_symlink_not_target() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("target.txt"), b"preserve me").unwrap();
    if let Err(e) = host_symlink(std::path::Path::new("target.txt"), &tmp.path().join("link")) {
        if is_privilege_error(&e) {
            println!("skipping: host cannot create symlinks ({})", e);
            return;
        }
        panic!("host_symlink failed: {}", e);
    }
    let req = make_unlink_request("link");
    let resp = handler.handle_request(&req).expect("response expected");
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(resp[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "unlink of symlink should succeed");
    // Target file must survive.
    assert_eq!(fs::read(tmp.path().join("target.txt")).unwrap(), b"preserve me");
    // Symlink itself must be gone.
    assert!(
        !tmp.path().join("link").exists() && fs::symlink_metadata(tmp.path().join("link")).is_err()
    );
}

#[test]
fn test_symlink_creates_link() {
    let (mut handler, tmp) = setup();
    // Build multi-part symlink request.
    let parts = make_long_symlink_parts("target.txt", "link", OperationId::from_raw(101));
    let response = feed_parts(&mut handler, &parts);
    assert_eq!(get_op_id(&response), OperationId::from_raw(101));
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    if status == HOSTFS_ERR_NOT_SUPPORTED {
        println!("skipping: host cannot create symlinks (ENOTSUP)");
        return;
    }
    assert_eq!(status, 0, "symlink should succeed, got {}", status);
    // Verify the link exists and points to the intended target.
    let meta = fs::symlink_metadata(tmp.path().join("link")).expect("link should exist");
    assert!(meta.file_type().is_symlink(), "created entry must be a symlink");
    let target = fs::read_link(tmp.path().join("link")).expect("readlink");
    assert_eq!(target, std::path::PathBuf::from("target.txt"));
}

#[test]
fn test_symlink_rejects_empty_target() {
    let (mut handler, _tmp) = setup();
    let parts = make_long_symlink_parts("", "link", OperationId::from_raw(102));
    let response = feed_parts(&mut handler, &parts);
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, HOSTFS_ERR_INVALID);
}

#[test]
fn test_symlink_rejects_nul_in_target() {
    let (mut handler, _tmp) = setup();
    let parts = make_long_symlink_parts("a\0b", "link", OperationId::from_raw(103));
    let response = feed_parts(&mut handler, &parts);
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, HOSTFS_ERR_INVALID);
}

#[test]
fn test_symlink_linkpath_escapes_sandbox() {
    let (mut handler, _tmp) = setup();
    let parts = make_long_symlink_parts("target.txt", "../escape", OperationId::from_raw(104));
    let response = feed_parts(&mut handler, &parts);
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, HOSTFS_ERR_PERMISSION);
}

/// Reassembles a sequence of multi-part response messages into the raw body.
///
/// Each part has the wire layout:
/// `[header:2][total_parts:2 LE][part_number:2 LE][payload_size:1][payload:N]`
/// (see `build_long_readlink_response_part` in handler.rs).
fn drain_multipart_response(
    handler: &mut HostFsHandler,
    first: [u8; Message::PAYLOAD_SIZE],
    expected_header: SystemCallMessageHeader,
) -> Vec<u8> {
    fn read_part(
        payload: &[u8; Message::PAYLOAD_SIZE],
        expected_header: SystemCallMessageHeader,
    ) -> (u16, u16, Vec<u8>) {
        let header_raw: u16 = u16::from_ne_bytes([payload[0], payload[1]]);
        assert_eq!(header_raw, expected_header as u16, "unexpected response header");
        let total_parts: u16 = u16::from_le_bytes([payload[2], payload[3]]);
        let part_number: u16 = u16::from_le_bytes([payload[4], payload[5]]);
        let payload_size: usize = payload[6] as usize;
        let chunk: Vec<u8> = payload[7..7 + payload_size].to_vec();
        (total_parts, part_number, chunk)
    }

    let (total_parts, part_number, chunk0) = read_part(&first, expected_header);
    assert_eq!(part_number, 0, "first response part must have part_number == 0");

    let mut body: Vec<u8> = chunk0;
    for expected_pn in 1..total_parts {
        let next: [u8; Message::PAYLOAD_SIZE] = handler
            .take_next_response_part()
            .expect("expected another response part");
        let (tp, pn, chunk) = read_part(&next, expected_header);
        assert_eq!(tp, total_parts, "total_parts mismatch across parts");
        assert_eq!(pn, expected_pn, "out-of-order response part");
        body.extend_from_slice(&chunk);
    }
    assert!(handler.take_next_response_part().is_none(), "unexpected extra response parts queued");
    body
}

#[test]
fn test_readlink_long_target_multipart_response() {
    let (mut handler, tmp) = setup();
    fs::write(tmp.path().join("target.txt"), b"x").unwrap();

    // Construct a target longer than the inline cap so the handler is forced into
    // the multi-part response path.
    let target_str: String = "a/".repeat(120) + "tail.txt";
    assert!(target_str.len() > MAX_INLINE_READLINK_TARGET, "test target must exceed inline cap");
    assert!(target_str.len() <= ::sysapi::limits::PATH_MAX, "test target must fit long cap");

    if let Err(e) = host_symlink(std::path::Path::new(&target_str), &tmp.path().join("link")) {
        if is_privilege_error(&e) {
            println!("skipping: host cannot create symlinks ({})", e);
            return;
        }
        panic!("host_symlink failed: {}", e);
    }

    let req = make_readlink_request("link");
    let first = handler.handle_request(&req).expect("response expected");

    let body = drain_multipart_response(
        &mut handler,
        first,
        SystemCallMessageHeader::HostFsReadlinkResponsePart,
    );
    assert!(body.len() >= hostfs_api::long_msg::READLINK_RESPONSE_HEADER_SIZE, "body too short");
    let resp = hostfs_api::long_msg::deserialize_long_readlink_response(&body)
        .expect("readlink long response must deserialize");
    assert_eq!(resp.status, 0, "readlink should succeed");
    assert_eq!(resp.target.len(), target_str.len());
    assert_eq!(resp.target, target_str.as_bytes());
}
