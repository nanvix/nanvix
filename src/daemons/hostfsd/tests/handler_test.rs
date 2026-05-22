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

/// Builds an Open request payload for the given path and flags.
fn make_open_request(path: &str, flags: i32) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let path_bytes: &[u8] = path.as_bytes();
    let path_len: u16 = path_bytes.len().min(MAX_INLINE_PATH_LEN) as u16;
    let mut path_arr: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
    path_arr[..path_len as usize].copy_from_slice(&path_bytes[..path_len as usize]);

    let req: OpenRequest = OpenRequest {
        flags,
        path_len,
        path: path_arr,
    };
    set_header(&mut payload, SystemCallMessageHeader::HostFsOpenRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds a Close request payload.
fn make_close_request(fd: i32) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let req: CloseRequest = CloseRequest { fd };
    set_header(&mut payload, SystemCallMessageHeader::HostFsCloseRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds a Read request payload.
fn make_read_request(fd: i32, count: u32, offset: i64) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let req: ReadRequest = ReadRequest { fd, count, offset };
    set_header(&mut payload, SystemCallMessageHeader::HostFsReadRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds a Write request payload.
fn make_write_request(fd: i32, data: &[u8], offset: i64) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
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
    set_header(&mut payload, SystemCallMessageHeader::HostFsWriteRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds a Stat request payload.
fn make_stat_request(fd: i32) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let req: StatRequest = StatRequest { fd };
    set_header(&mut payload, SystemCallMessageHeader::HostFsStatRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds a Mkdir request payload.
fn make_mkdir_request(path: &str, mode: u32) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let path_bytes: &[u8] = path.as_bytes();
    let path_len: u16 = path_bytes.len().min(MAX_INLINE_PATH_LEN) as u16;
    let mut path_arr: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
    path_arr[..path_len as usize].copy_from_slice(&path_bytes[..path_len as usize]);

    let req: MkdirRequest = MkdirRequest {
        mode,
        path_len,
        path: path_arr,
    };
    set_header(&mut payload, SystemCallMessageHeader::HostFsMkdirRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds an Unlink request payload.
fn make_unlink_request(path: &str) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let path_bytes: &[u8] = path.as_bytes();
    let path_len: u16 = path_bytes.len().min(MAX_INLINE_PATH_LEN) as u16;
    let mut path_arr: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
    path_arr[..path_len as usize].copy_from_slice(&path_bytes[..path_len as usize]);

    let req: UnlinkRequest = UnlinkRequest {
        path_len,
        path: path_arr,
    };
    set_header(&mut payload, SystemCallMessageHeader::HostFsUnlinkRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds a Rmdir request payload.
fn make_rmdir_request(path: &str) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let path_bytes: &[u8] = path.as_bytes();
    let path_len: u16 = path_bytes.len().min(MAX_INLINE_PATH_LEN) as u16;
    let mut path_arr: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
    path_arr[..path_len as usize].copy_from_slice(&path_bytes[..path_len as usize]);

    let req: RmdirRequest = RmdirRequest {
        path_len,
        path: path_arr,
    };
    set_header(&mut payload, SystemCallMessageHeader::HostFsRmdirRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds a Rename request payload.
fn make_rename_request(old_path: &str, new_path: &str) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let old_bytes: &[u8] = old_path.as_bytes();
    let new_bytes: &[u8] = new_path.as_bytes();
    let old_len: u16 = old_bytes.len().min(MAX_INLINE_PATH_LEN) as u16;
    let new_len: u16 = new_bytes.len().min(MAX_INLINE_PATH_LEN - old_len as usize) as u16;

    let mut paths: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
    paths[..old_len as usize].copy_from_slice(&old_bytes[..old_len as usize]);
    paths[old_len as usize..old_len as usize + new_len as usize]
        .copy_from_slice(&new_bytes[..new_len as usize]);

    let req: RenameRequest = RenameRequest {
        old_path_len: old_len,
        new_path_len: new_len,
        paths,
    };
    set_header(&mut payload, SystemCallMessageHeader::HostFsRenameRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds an Lseek request payload.
fn make_lseek_request(fd: i32, offset: i64, whence: i32) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let req: LseekRequest = LseekRequest { fd, offset, whence };
    set_header(&mut payload, SystemCallMessageHeader::HostFsLseekRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds a Truncate request payload.
fn make_truncate_request(fd: i32, length: i64) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let req: TruncateRequest = TruncateRequest { fd, length };
    set_header(&mut payload, SystemCallMessageHeader::HostFsTruncateRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds a Flush request payload.
fn make_flush_request(fd: i32) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let req: FlushRequest = FlushRequest { fd };
    set_header(&mut payload, SystemCallMessageHeader::HostFsFlushRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Builds a ReadDir request payload.
fn make_readdir_request(fd: i32) -> [u8; Message::PAYLOAD_SIZE] {
    make_readdir_request_at(fd, 0)
}

/// Builds a ReadDir request payload with a specified offset.
fn make_readdir_request_at(fd: i32, offset: u32) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    let req: ReadDirRequest = ReadDirRequest {
        fd,
        _reserved: 0,
        offset,
    };
    set_header(&mut payload, SystemCallMessageHeader::HostFsReadDirRequest as u16);
    req.encode(&mut payload);
    payload
}

/// Opens a file via the handler and returns the remote FD.
fn open_file(handler: &mut HostFsHandler, path: &str, flags: i32) -> i32 {
    let payload: [u8; Message::PAYLOAD_SIZE] = make_open_request(path, flags);
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
    let mut payload: [u8; Message::PAYLOAD_SIZE] = make_open_request("opid.txt", O_RDONLY);
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

    let mut payload: [u8; Message::PAYLOAD_SIZE] = make_open_request("zero.txt", O_RDONLY);
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
    op_id: OperationId,
) -> Vec<[u8; Message::PAYLOAD_SIZE]> {
    let path_bytes: &[u8] = path.as_bytes();
    let path_len: u16 = path_bytes.len() as u16;

    // Serialize: [op_id:4][flags:4][path_len:2][path:N]
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&op_id.to_le_bytes());
    data.extend_from_slice(&flags.to_le_bytes());
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

    let parts = make_long_open_parts("short.txt", O_RDONLY, OperationId::from_raw(7));
    assert_eq!(parts.len(), 1, "short path should fit in one part");

    let response = handler.handle_request(&parts[0]);
    assert!(response.is_some(), "single-part request should produce a response");
    let response = response.unwrap();
    assert_eq!(get_op_id(&response), OperationId::from_raw(7));
    let resp: OpenResponse = OpenResponse::decode(&response);
    assert!(resp.fd > 0, "open should succeed");
}

#[test]
fn test_long_open_multi_part() {
    let (mut handler, tmp) = setup();
    // Create a deeply nested path that requires multiple parts.
    let long_name: String = "a".repeat(50);
    fs::create_dir(tmp.path().join(&long_name)).unwrap();
    let file_name: String = format!("{}/file.txt", long_name);
    fs::write(tmp.path().join(&file_name), b"hello").unwrap();

    let parts = make_long_open_parts(&file_name, O_RDONLY, OperationId::from_raw(42));
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

    let parts = make_long_open_parts("does-not-exist.txt", O_RDONLY, OperationId::from_raw(10));
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
