// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// Size of the ring header in bytes.
pub const RING_HEADER_SIZE: usize = 16;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Header for a shared-memory SPSC ring buffer used by the VMBus. This structure is placed at the
/// beginning of each ring buffer region and is accessed via volatile reads and writes from both the
/// guest kernel and the VMM host.
///
/// On x86, Total Store Order (TSO) guarantees that stores by the producer are visible in program
/// order to the consumer. Therefore, volatile accesses alone are sufficient for correctness in a
/// single-producer, single-consumer ring buffer -- no explicit memory fences are required.
///
/// # Fields
///
/// - `head`: Index of the next slot to be written by the producer. The producer increments this
///   after writing a message. The consumer reads this to determine availability.
/// - `tail`: Index of the next slot to be read by the consumer. The consumer increments this after
///   reading a message. The producer reads this to determine free space.
/// - `capacity`: Total number of message slots in the ring buffer. This value is set during
///   initialization and must not be modified afterwards.
/// - `flags`: Reserved for future use. Must be initialized to zero.
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RingHeader {
    /// Written by producer, read by consumer.
    pub head: u32,
    /// Written by consumer, read by producer.
    pub tail: u32,
    /// Number of message slots (immutable after init).
    pub capacity: u32,
    /// Reserved flags.
    pub flags: u32,
}

::static_assert::assert_eq_size!(RingHeader, RING_HEADER_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl RingHeader {
    ///
    /// # Description
    ///
    /// Creates a new ring header with the given capacity.
    ///
    /// # Parameters
    ///
    /// - `capacity`: Number of message slots in the ring.
    ///
    /// # Returns
    ///
    /// A new ring header with `head`, `tail`, and `flags` initialized to zero.
    ///
    pub const fn new(capacity: u32) -> Self {
        Self {
            head: 0,
            tail: 0,
            capacity,
            flags: 0,
        }
    }

    ///
    /// # Description
    ///
    /// Serializes the ring header to its little-endian byte representation.
    ///
    /// # Returns
    ///
    /// A 16-byte array containing the little-endian encoding of the ring header fields.
    ///
    pub fn to_bytes(self) -> [u8; RING_HEADER_SIZE] {
        let mut bytes: [u8; RING_HEADER_SIZE] = [0u8; RING_HEADER_SIZE];
        bytes[0..4].copy_from_slice(&self.head.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.tail.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.capacity.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.flags.to_le_bytes());
        bytes
    }

    ///
    /// # Description
    ///
    /// Deserializes a ring header from a little-endian byte representation.
    ///
    /// # Parameters
    ///
    /// - `bytes`: 16-byte array containing the ring header fields.
    ///
    /// # Returns
    ///
    /// The deserialized ring header.
    ///
    pub fn from_bytes(bytes: [u8; RING_HEADER_SIZE]) -> Self {
        Self {
            head: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            tail: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            capacity: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            flags: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        }
    }
}
