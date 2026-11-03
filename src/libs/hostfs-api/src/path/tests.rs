// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn strips_guest_mount_prefix_without_normalizing() {
    for (path, expected) in [
        ("/mnt", ""),
        ("/mnt/", ""),
        ("/mnt//", "/"),
        ("/mnt/mnt/file", "mnt/file"),
        ("/mnt/a//./b/", "a//./b/"),
        ("/mnt/file", "file"),
        ("/mnt//file", "/file"),
        ("/mnt/./file", "./file"),
        ("/mnt/directory/../file", "directory/../file"),
        ("/mnt/directory//file", "directory//file"),
    ] {
        let resolved =
            HostResolvedPath::from_guest_path(path).expect("hostfs guest path should be accepted");
        assert_eq!(resolved.as_str(), expected);
        assert_eq!(resolved.into_string(), expected);
    }
}

#[test]
fn rejects_paths_outside_hostfs_mount() {
    for path in ["", "/", "/mntfoo", "/mntfoo/file"] {
        assert_eq!(
            HostResolvedPath::from_guest_path(path),
            Err(HOSTFS_ERR_INVALID),
            "path outside hostfs mount should be rejected: {path:?}",
        );
    }
}

#[test]
fn checks_wire_paths_without_normalizing_them() {
    for path in ["", "file", "/file", "./file", "directory/../file", "a//b"] {
        let resolved: HostResolvedPath = HostResolvedPath::from_wire(String::from(path))
            .expect("valid hostfs wire path should be accepted");
        assert_eq!(resolved.as_str(), path);
    }

    assert_eq!(HostResolvedPath::from_wire(String::from("file\0name")), Err(HOSTFS_ERR_INVALID),);
}

#[test]
fn typed_paths_keep_existing_wire_encoding() {
    use crate::{
        long_msg,
        LstatRequest,
        OperationId,
    };

    let path = HostResolvedPath::from_guest_path("/mnt/a/../b//").expect("valid hostfs guest path");
    let bytes = path.as_str().as_bytes();
    let op_id = OperationId::from_le_bytes(7u32.to_le_bytes());
    // The wire library treats the caller's message kind as an opaque field.
    const KIND: u16 = 1;
    let inline = LstatRequest::from_path(bytes).expect("short path fits inline");
    let expected = LstatRequest::from_path(b"a/../b//").expect("short path fits inline");
    assert_eq!(inline.serialize(KIND, op_id), expected.serialize(KIND, op_id));
    let long =
        long_msg::serialize_long_open_request(op_id, 0, 0, bytes).expect("valid long request");
    let expected_long = long_msg::serialize_long_open_request(op_id, 0, 0, b"a/../b//")
        .expect("valid long request");
    assert_eq!(long, expected_long, "typed paths must not add wire tags or normalize spelling");
}

#[test]
fn rejects_malformed_wire_bytes() {
    assert_eq!(HostResolvedPath::from_wire_bytes(&[0xff]), Err(HOSTFS_ERR_INVALID),);
    assert_eq!(HostResolvedPath::from_wire_bytes(b"file\0name"), Err(HOSTFS_ERR_INVALID),);

    let resolved: HostResolvedPath = HostResolvedPath::from_wire_bytes(b"a/../b")
        .expect("valid UTF-8 hostfs path should be accepted");
    assert_eq!(resolved.as_str(), "a/../b");
}
