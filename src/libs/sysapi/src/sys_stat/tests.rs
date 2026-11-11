// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::file_type::*;
use crate::sys_types::mode_t;

/// A file type test, with its name and the file type that it reports.
type FileTypeTest = (&'static str, fn(mode_t) -> bool, mode_t);

/// File type tests.
const FILE_TYPE_TESTS: [FileTypeTest; 11] = [
    ("S_ISFIFO", S_ISFIFO, S_IFIFO),
    ("S_ISCHR", S_ISCHR, S_IFCHR),
    ("S_ISDIR", S_ISDIR, S_IFDIR),
    ("S_ISBLK", S_ISBLK, S_IFBLK),
    ("S_ISREG", S_ISREG, S_IFREG),
    ("S_ISLNK", S_ISLNK, S_IFLNK),
    ("S_ISSOCK", S_ISSOCK, S_IFSOCK),
    ("S_TYPEISMQ", S_TYPEISMQ, S_IFMQ),
    ("S_TYPEISSEM", S_TYPEISSEM, S_IFSEM),
    ("S_TYPEISSHM", S_TYPEISSHM, S_IFSHM),
    ("S_TYPEISTMO", S_TYPEISTMO, S_IFTMO),
];

/// Tests that each file type test reports its own file type and no other one.
#[test]
fn file_type_tests_tell_file_types_apart() {
    for (i, (name, is_file_type, _)) in FILE_TYPE_TESTS.iter().enumerate() {
        for (j, (_, _, file_type)) in FILE_TYPE_TESTS.iter().enumerate() {
            // Set all other file mode bits too, so that the test has to mask them out.
            let mode: mode_t = file_type | 0o7777;
            assert_eq!(is_file_type(mode), i == j, "{name}({mode:#o})");
        }
    }
}
