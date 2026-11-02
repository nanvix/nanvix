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
mod tests;
