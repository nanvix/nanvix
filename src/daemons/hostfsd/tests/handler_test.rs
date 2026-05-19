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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    handler.handle_request(&payload);
}

#[test]
fn test_close_invalid_fd_fails() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_close_request(999);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    let resp: WriteResponse = WriteResponse::decode(&response);
    assert_eq!(resp.bytes_written, write_data.len() as i32);

    // Close.
    handler.handle_request(&make_close_request(fd));

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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    let resp: ReadResponse = ReadResponse::decode(&response);
    assert_eq!(resp.bytes_read, 5);
    assert_eq!(&resp.data[..5], b"defgh");
}

#[test]
fn test_read_invalid_fd() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_read_request(999, 10, -1);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    let resp: StatResponse = StatResponse::decode(&response);
    assert_eq!(resp.status, 0);
    assert_eq!(resp.is_dir, 1);
}

#[test]
fn test_stat_invalid_fd() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_stat_request(999);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "mkdir should succeed");
    assert!(tmp.path().join("newdir").is_dir());

    // Remove directory.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_rmdir_request("newdir");
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0, "rmdir should succeed");
    assert!(!tmp.path().join("newdir").exists());
}

#[test]
fn test_rmdir_nonexistent_fails() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_rmdir_request("ghost");
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0);
    assert!(!tmp.path().join("remove-me.txt").exists());
}

#[test]
fn test_unlink_nonexistent_fails() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_unlink_request("nope.txt");
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    let resp: LseekResponse = LseekResponse::decode(&response);
    assert_eq!(resp.offset, 5);

    // Read from current position (offset = -1 means current pos).
    let payload: [u8; Message::PAYLOAD_SIZE] = make_read_request(fd, 5, -1);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0);

    // Close and verify on disk.
    handler.handle_request(&make_close_request(fd));
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    handler.handle_request(&make_write_request(fd, b"data", -1));

    // Flush.
    let payload: [u8; Message::PAYLOAD_SIZE] = make_flush_request(fd);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    let ds: usize = HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response[ds..ds + 4].try_into().unwrap());
    assert_eq!(status, 0);

    // Verify data persisted.
    let _ = tmp; // keep tmp alive
    handler.handle_request(&make_close_request(fd));
    assert_eq!(fs::read(tmp.path().join("flush.txt")).unwrap(), b"data");
}

#[test]
fn test_flush_invalid_fd() {
    let (mut handler, _tmp) = setup();

    let payload: [u8; Message::PAYLOAD_SIZE] = make_flush_request(999);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
        let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    let entry: ReadDirEntry = ReadDirEntry::decode(&response);
    assert!(entry.name_len > 0);

    // Offset 1 should return empty (end of directory).
    let payload: [u8; Message::PAYLOAD_SIZE] = make_readdir_request_at(fd, 1);
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let resp: [u8; Message::PAYLOAD_SIZE] =
        handler.handle_request(&make_write_request(fd, b"round-trip-data", -1));
    let wr: WriteResponse = WriteResponse::decode(&resp);
    assert_eq!(wr.bytes_written, 15);
    handler.handle_request(&make_close_request(fd));

    // Reopen for reading.
    let fd2: i32 = open_file(&mut handler, "roundtrip.txt", O_RDONLY);
    assert!(fd2 > 0);

    // Read back.
    let resp: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&make_read_request(fd2, 42, -1));
    let rd: ReadResponse = ReadResponse::decode(&resp);
    assert_eq!(rd.bytes_read, 15);
    assert_eq!(&rd.data[..15], b"round-trip-data");
}

#[test]
fn test_nested_directory_workflow() {
    let (mut handler, tmp) = setup();

    // Create nested directories.
    handler.handle_request(&make_mkdir_request("a", 0o755));
    handler.handle_request(&make_mkdir_request("a/b", 0o755));
    assert!(tmp.path().join("a/b").is_dir());

    // Create a file inside nested dir.
    let fd: i32 = open_file(&mut handler, "a/b/deep.txt", O_WRONLY | O_CREAT | O_TRUNC);
    assert!(fd > 0);
    handler.handle_request(&make_write_request(fd, b"deep", -1));
    handler.handle_request(&make_close_request(fd));

    // Verify.
    assert_eq!(fs::read(tmp.path().join("a/b/deep.txt")).unwrap(), b"deep");

    // Clean up: unlink file, rmdir b, rmdir a.
    let resp: [u8; Message::PAYLOAD_SIZE] =
        handler.handle_request(&make_unlink_request("a/b/deep.txt"));
    let ds: usize = HOSTFS_DATA_START;
    assert_eq!(i32::from_le_bytes(resp[ds..ds + 4].try_into().unwrap()), 0);
    let resp: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&make_rmdir_request("a/b"));
    assert_eq!(i32::from_le_bytes(resp[ds..ds + 4].try_into().unwrap()), 0);
    let resp: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&make_rmdir_request("a"));
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

    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
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
    let close_response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&close_payload);
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

    let response: [u8; Message::PAYLOAD_SIZE] = handler.handle_request(&payload);
    assert_eq!(get_op_id(&response), OperationId::from_raw(0), "op_id 0 should be echoed");
}
