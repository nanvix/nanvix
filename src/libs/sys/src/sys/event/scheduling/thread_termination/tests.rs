// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn thread_termination_info_round_trip() {
    // Each field carries a distinct value, so a field that the native-endian layout drops or
    // overlaps is caught rather than masked by a zero byte pattern.
    for status in [ExitStatus::ok(), ExitStatus::from(0x0102_0304_u32)] {
        let info: ThreadTerminationInfo = ThreadTerminationInfo::new(
            ProcessIdentifier::from(5),
            ThreadIdentifier::from(7),
            status,
        );

        assert_eq!(ThreadTerminationInfo::from_ne_bytes(info.to_ne_bytes()), info);
    }
}
