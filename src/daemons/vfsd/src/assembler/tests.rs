// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn assembler_key_separates_request_identifiers() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(6);
    let source: ThreadIdentifier = ThreadIdentifier::from(7);
    let header: SystemCallMessageKind = SystemCallMessageKind::OpenAtRequestPart;

    let first: AssemblerKey = assembler_key(pid, source, header, RequestIdentifier::from_raw(11));
    let second: AssemblerKey = assembler_key(pid, source, header, RequestIdentifier::from_raw(12));

    assert_ne!(first, second, "request identifiers should select separate streams");
}

#[test]
fn assembler_key_separates_processes() {
    let source: ThreadIdentifier = ThreadIdentifier::from(7);
    let header: SystemCallMessageKind = SystemCallMessageKind::OpenAtRequestPart;
    let request_id: RequestIdentifier = RequestIdentifier::from_raw(11);

    let first: AssemblerKey = assembler_key(ProcessIdentifier::from(6), source, header, request_id);
    let second: AssemblerKey =
        assembler_key(ProcessIdentifier::from(8), source, header, request_id);

    assert_ne!(first, second, "process identifiers should select separate streams");
}
