// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn chunk_response_header_preserves_pull_correlation() {
    let pull_header: DataChunkHeader = DataChunkHeader::new_tagged(
        ProcessIdentifier::from(2),
        ThreadIdentifier::from(3),
        ProcessIdentifier::KERNEL,
        ThreadIdentifier::KERNEL,
        0x1000_0000,
        4080,
        42,
    );

    let response_header: DataChunkHeader =
        chunk_response_header(&pull_header, 32).expect("chunk length should fit in u32");

    assert_eq!(response_header.tag(), pull_header.tag());
    assert_eq!(response_header.source_pid(), pull_header.source_pid());
    assert_eq!(response_header.source_tid(), pull_header.source_tid());
    assert_eq!(response_header.destination_pid(), pull_header.destination_pid());
    assert_eq!(response_header.destination_tid(), pull_header.destination_tid());
    assert_eq!(response_header.data_addr(), pull_header.data_addr());
    assert_eq!(response_header.data_len(), 32);
}
