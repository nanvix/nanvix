// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Asserts that every identifier in a namespace is nonzero and unique.
fn assert_valid_namespace(ids: &[u64]) {
    for (index, id) in ids.iter().enumerate() {
        assert_ne!(*id, 0, "identifier at index {index} is reserved");
        assert!(!ids[..index].contains(id), "identifier {id} is allocated more than once");
    }
}

#[test]
fn fixed_identifiers_are_nonzero_and_unique() {
    assert_valid_namespace(&[
        FilesystemDeviceId::Vfs.into(),
        FilesystemDeviceId::PipeFs.into(),
        FilesystemDeviceId::Console.into(),
        FilesystemDeviceId::DevFs.into(),
        FilesystemDeviceId::HostFs.into(),
    ]);
    assert_valid_namespace(&[VfsInodeId::Fallback.into()]);
    assert_valid_namespace(&[HostFsInodeId::Fallback.into()]);
    assert_valid_namespace(&[
        DevFsInodeId::Directory.into(),
        DevFsInodeId::Null.into(),
        DevFsInodeId::Tty.into(),
        DevFsInodeId::Console.into(),
    ]);
    assert_valid_namespace(&[
        ConsoleInodeId::Stdin.into(),
        ConsoleInodeId::Stdout.into(),
        ConsoleInodeId::Stderr.into(),
    ]);
    assert_valid_namespace(&[
        DevFsCharacterDeviceId::Null.into(),
        DevFsCharacterDeviceId::Tty.into(),
        DevFsCharacterDeviceId::Console.into(),
    ]);
}
