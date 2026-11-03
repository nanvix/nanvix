// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn response_roundtrips() {
    let expected = StatTimesResponse {
        atim: StatTime {
            tv_sec: -1,
            tv_nsec: 999_999_999,
        },
        mtim: StatTime {
            tv_sec: 1_704_067_200,
            tv_nsec: 123_456_789,
        },
        ctim: StatTime {
            tv_sec: 1_704_067_201,
            tv_nsec: 42,
        },
    };
    let mut payload = [0; Message::PAYLOAD_SIZE];
    expected.encode(&mut payload);
    let actual = StatTimesResponse::decode(&payload).expect("valid timestamps");
    assert_eq!(actual.atim.tv_sec, expected.atim.tv_sec);
    assert_eq!(actual.atim.tv_nsec, expected.atim.tv_nsec);
    assert_eq!(actual.mtim.tv_sec, expected.mtim.tv_sec);
    assert_eq!(actual.mtim.tv_nsec, expected.mtim.tv_nsec);
    assert_eq!(actual.ctim.tv_sec, expected.ctim.tv_sec);
    assert_eq!(actual.ctim.tv_nsec, expected.ctim.tv_nsec);
}

#[test]
fn response_rejects_invalid_nanoseconds() {
    let response = StatTimesResponse {
        atim: StatTime {
            tv_sec: 0,
            tv_nsec: 1_000_000_000,
        },
        mtim: StatTime {
            tv_sec: 0,
            tv_nsec: 0,
        },
        ctim: StatTime {
            tv_sec: 0,
            tv_nsec: 0,
        },
    };
    let mut payload = [0; Message::PAYLOAD_SIZE];
    response.encode(&mut payload);
    assert!(StatTimesResponse::decode(&payload).is_none());
}
