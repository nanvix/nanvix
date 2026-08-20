// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Timestamp response shared by hostfs stat operations.

use crate::HOSTFS_DATA_START;
use ::sys::ipc::Message;

/// Wire timestamp with full POSIX precision.
#[derive(Debug, Clone, Copy)]
pub struct StatTime {
    /// Seconds since the Unix epoch.
    pub tv_sec: i64,
    /// Nanoseconds within the second.
    pub tv_nsec: u32,
}

/// Timestamps returned after successful stat metadata.
#[derive(Debug, Clone, Copy)]
pub struct StatTimesResponse {
    /// Last access time.
    pub atim: StatTime,
    /// Last modification time.
    pub mtim: StatTime,
    /// Last status-change time.
    pub ctim: StatTime,
}

impl StatTimesResponse {
    const TIME_SIZE: usize = 12;

    /// Encodes this response into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        for (index, time) in [self.atim, self.mtim, self.ctim].iter().enumerate() {
            let offset: usize = HOSTFS_DATA_START + index * Self::TIME_SIZE;
            payload[offset..offset + 8].copy_from_slice(&time.tv_sec.to_le_bytes());
            payload[offset + 8..offset + 12].copy_from_slice(&time.tv_nsec.to_le_bytes());
        }
    }

    /// Decodes this response from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Option<Self> {
        let decode = |index: usize| {
            let offset: usize = HOSTFS_DATA_START + index * Self::TIME_SIZE;
            let tv_nsec: u32 =
                u32::from_le_bytes(payload[offset + 8..offset + 12].try_into().ok()?);
            if tv_nsec >= 1_000_000_000 {
                return None;
            }
            Some(StatTime {
                tv_sec: i64::from_le_bytes(payload[offset..offset + 8].try_into().ok()?),
                tv_nsec,
            })
        };
        Some(Self {
            atim: decode(0)?,
            mtim: decode(1)?,
            ctim: decode(2)?,
        })
    }
}

#[cfg(test)]
mod tests {
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
}
