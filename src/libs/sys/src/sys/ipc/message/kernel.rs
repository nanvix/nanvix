// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::typ::MessageType,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::core::mem;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MessageSender(i32);

impl MessageSender {
    /// The kernel process is the sender of the message.
    pub const KERNEL: Self = MessageSender(ProcessIdentifier::KERNEL_RAW);
}

impl MessageSender {
    pub fn as_id(&self) -> Result<ProcessIdentifier, ThreadIdentifier> {
        if self.0 >= 0 {
            Ok(ProcessIdentifier::from(self.0))
        } else {
            Err(ThreadIdentifier::from(-self.0))
        }
    }
}

impl From<ProcessIdentifier> for MessageSender {
    fn from(pid: ProcessIdentifier) -> Self {
        Self(pid.into())
    }
}

impl From<ThreadIdentifier> for MessageSender {
    fn from(tid: ThreadIdentifier) -> Self {
        let tid: i32 = tid.into();
        Self(-tid)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MessageReceiver(i32);

impl MessageReceiver {
    pub fn as_id(&self) -> Result<ProcessIdentifier, ThreadIdentifier> {
        if self.0 >= 0 {
            Ok(ProcessIdentifier::from(self.0))
        } else {
            Err(ThreadIdentifier::from(-self.0))
        }
    }
}

impl MessageReceiver {
    /// The kernel process is the receiver of the message.
    pub const KERNEL: Self = MessageReceiver(ProcessIdentifier::KERNEL_RAW);
}

impl From<ProcessIdentifier> for MessageReceiver {
    fn from(pid: ProcessIdentifier) -> Self {
        Self(pid.into())
    }
}

impl From<ThreadIdentifier> for MessageReceiver {
    fn from(tid: ThreadIdentifier) -> Self {
        let tid: i32 = tid.into();
        Self(-tid)
    }
}

///
/// # Description
///
/// A structure that represents a message that can be sent between processes.
///
/// # Notes
///
/// - All fields in this structure are intentionally public to enable zero-copy message parsing.
///
#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct Message {
    /// Type of the message.
    pub message_type: MessageType,
    /// Process that sent the message.
    pub source: MessageSender,
    /// Process that should receive the message.
    pub destination: MessageReceiver,
    /// Message status.
    pub status: i32,
    /// Payload of the message.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}
::static_assert::assert_eq_size!(Message, config::kernel::IPC_MESSAGE_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl Message {
    /// The size of the message header fields (source, destination and type).
    pub const HEADER_SIZE: usize =
        2 * mem::size_of::<ProcessIdentifier>() + MessageType::SIZE + mem::size_of::<i32>();
    /// The size of the message's payload.
    pub const PAYLOAD_SIZE: usize = config::kernel::IPC_MESSAGE_SIZE - Self::HEADER_SIZE;

    ///
    /// # Description
    ///
    /// Creates a new message.
    ///
    /// # Parameters
    ///
    /// - `source`: The sender of the message.
    /// - `destination`: The recipient of the message.
    /// - `message_type`: The type of the message.
    /// - `status`: Error status of the message (`None` for success).
    /// - `payload`: The message payload.
    ///
    /// # Returns
    ///
    /// The new message.
    ///
    pub fn new(
        source: MessageSender,
        destination: MessageReceiver,
        message_type: MessageType,
        status: Option<ErrorCode>,
        payload: [u8; Self::PAYLOAD_SIZE],
    ) -> Self {
        Self {
            message_type,
            source,
            destination,
            status: if let Some(status) = status {
                status.get()
            } else {
                0
            },
            payload,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the target message to a byte array.
    ///
    /// # Returns
    ///
    /// A byte array that represents the target message.
    ///
    pub fn to_bytes(self) -> [u8; Self::HEADER_SIZE + Self::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    ///
    /// # Description
    ///
    /// Attempts to convert a byte array to a message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array to convert.
    ///
    /// # Returns
    ///
    /// Upon success, the message is returned. Upon failure, an error is returned instead.
    ///
    pub fn try_from_bytes(
        bytes: [u8; Self::HEADER_SIZE + Self::PAYLOAD_SIZE],
    ) -> Result<Self, Error> {
        Ok(unsafe { mem::transmute::<[u8; config::kernel::IPC_MESSAGE_SIZE], Message>(bytes) })
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            message_type: MessageType::Ikc,
            source: MessageSender::KERNEL,
            destination: MessageReceiver::KERNEL,
            status: 0,
            payload: [0; Self::PAYLOAD_SIZE],
        }
    }
}

///
/// # Description
///
/// A wrapping structure for IPC messages exchanged between the user VM and the kernel over the
/// virtual message bus (vmbus). Instead of passing the raw message address, the vmbus
/// reads/writes the address of this structure.
///
/// # Notes
///
/// - The `message_addr` field stores a guest virtual address (32-bit) pointing to the actual
///   message bytes.
/// - All fields are private and accessed via getter/setter methods.
/// - Fields are stored as `u64` (not `u32`) for performance on host side.
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VmBusMessage {
    /// Size of the message in bytes (stored as `u64`, logical type is `u32`).
    size: u64,
    /// Whether this is an IKC message (stored as `u64`, logical type is `bool`).
    is_ikc: u64,
    /// Guest virtual address of the message (stored as `u64`, logical type is `u32`).
    message_addr: u64,
}
::static_assert::assert_eq_size!(VmBusMessage, 3 * mem::size_of::<u64>());

//==================================================================================================
// Implementations
//==================================================================================================

impl VmBusMessage {
    /// Size of the envelope in bytes.
    pub const SIZE: usize = mem::size_of::<Self>();

    ///
    /// # Description
    ///
    /// Creates a new message envelope.
    ///
    /// # Parameters
    ///
    /// - `size`: Size of the message in bytes.
    /// - `is_ikc`: Whether this is an IKC message.
    /// - `message_addr`: Guest virtual address of the message.
    ///
    /// # Returns
    ///
    /// The new message envelope.
    ///
    pub fn new(size: u32, is_ikc: bool, message_addr: u32) -> Self {
        Self {
            size: size as u64,
            is_ikc: is_ikc as u64,
            message_addr: message_addr as u64,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the size of the message in bytes.
    ///
    pub fn size(&self) -> u32 {
        self.size as u32
    }

    ///
    /// # Description
    ///
    /// Sets the size of the message in bytes.
    ///
    /// # Parameters
    ///
    /// - `size`: Size of the message in bytes.
    ///
    pub fn set_size(&mut self, size: u32) {
        self.size = size as u64;
    }

    ///
    /// # Description
    ///
    /// Returns whether this is an IKC message.
    ///
    pub fn is_ikc(&self) -> bool {
        self.is_ikc != 0
    }

    ///
    /// # Description
    ///
    /// Sets whether this is an IKC message.
    ///
    /// # Parameters
    ///
    /// - `is_ikc`: Whether this is an IKC message.
    ///
    pub fn set_is_ikc(&mut self, is_ikc: bool) {
        self.is_ikc = is_ikc as u64;
    }

    ///
    /// # Description
    ///
    /// Returns the guest virtual address of the message.
    ///
    pub fn message_addr(&self) -> u32 {
        self.message_addr as u32
    }

    ///
    /// # Description
    ///
    /// Sets the guest virtual address of the message.
    ///
    /// # Parameters
    ///
    /// - `message_addr`: Guest virtual address of the message.
    ///
    pub fn set_message_addr(&mut self, message_addr: u32) {
        self.message_addr = message_addr as u64;
    }

    ///
    /// # Description
    ///
    /// Converts the target message envelope to a byte array.
    ///
    /// # Returns
    ///
    /// A byte array that represents the target message envelope.
    ///
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        unsafe { mem::transmute(self) }
    }

    ///
    /// # Description
    ///
    /// Attempts to convert a byte array to a message envelope.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array to convert.
    ///
    /// # Returns
    ///
    /// Upon success, the message envelope is returned. Upon failure, an error is returned instead.
    ///
    /// # Notes
    ///
    /// This function currently cannot fail because all bit patterns are valid for the `repr(C)`
    /// layout. The `Result` return type is retained for forward compatibility in case field
    /// validation is added in the future.
    ///
    pub fn try_from_bytes(bytes: [u8; Self::SIZE]) -> Result<Self, Error> {
        Ok(unsafe { mem::transmute::<[u8; Self::SIZE], VmBusMessage>(bytes) })
    }
}

//==================================================================================================
// DataChunkHeader
//==================================================================================================

///
/// # Description
///
/// Header structure describing a data chunk transfer between a user process and the kernel
/// (linuxd). This header is placed in guest memory and referenced by a [`VmBusMessage`] with
/// `is_ikc` set to `false`. The `message_addr` field of the envelope points to this header, and
/// the envelope's `size` field holds the byte count of the bulk payload.
///
/// # Notes
///
/// - All fields use fixed-width types for ABI stability across the guest/host boundary.
/// - The `data_addr` field stores a guest physical address pointing to the bulk payload buffer.
/// - This structure is `no_std`-compatible so it can be used inside the kernel.
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DataChunkHeader {
    /// Process identifier of the source (sender), stored as a fixed-width `i32`.
    source_pid: i32,
    /// Thread identifier of the source (sender), stored as a fixed-width `i32`.
    source_tid: i32,
    /// Process identifier of the destination (receiver), stored as a fixed-width `i32`.
    destination_pid: i32,
    /// Thread identifier of the destination (receiver), stored as a fixed-width `i32`.
    destination_tid: i32,
    /// Guest physical address of the bulk data buffer.
    data_addr: u32,
    /// Number of bytes in the bulk payload.
    data_len: u32,
}
::static_assert::assert_eq_size!(DataChunkHeader, 6 * mem::size_of::<u32>());

//==================================================================================================
// Implementations
//==================================================================================================

impl DataChunkHeader {
    /// Size of the header in bytes.
    pub const SIZE: usize = mem::size_of::<Self>();

    ///
    /// # Description
    ///
    /// Creates a new data chunk transfer header.
    ///
    /// # Parameters
    ///
    /// - `source_pid`: Process identifier of the source.
    /// - `source_tid`: Thread identifier of the source.
    /// - `destination_pid`: Process identifier of the destination.
    /// - `destination_tid`: Thread identifier of the destination.
    /// - `data_addr`: Guest physical address of the bulk data buffer.
    /// - `data_len`: Number of bytes in the bulk payload.
    ///
    /// # Returns
    ///
    /// The new data chunk transfer header.
    ///
    pub fn new(
        source_pid: ProcessIdentifier,
        source_tid: ThreadIdentifier,
        destination_pid: ProcessIdentifier,
        destination_tid: ThreadIdentifier,
        data_addr: u32,
        data_len: u32,
    ) -> Self {
        let source_pid_raw: i32 = source_pid.into();
        let source_tid_raw: i32 = source_tid.into();
        let destination_pid_raw: i32 = destination_pid.into();
        let destination_tid_raw: i32 = destination_tid.into();
        Self {
            source_pid: source_pid_raw,
            source_tid: source_tid_raw,
            destination_pid: destination_pid_raw,
            destination_tid: destination_tid_raw,
            data_addr,
            data_len,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the process identifier of the source.
    ///
    pub fn source_pid(&self) -> ProcessIdentifier {
        ProcessIdentifier::from(self.source_pid)
    }

    ///
    /// # Description
    ///
    /// Returns the thread identifier of the source.
    ///
    pub fn source_tid(&self) -> ThreadIdentifier {
        ThreadIdentifier::from(self.source_tid)
    }

    ///
    /// # Description
    ///
    /// Returns the process identifier of the destination.
    ///
    pub fn destination_pid(&self) -> ProcessIdentifier {
        ProcessIdentifier::from(self.destination_pid)
    }

    ///
    /// # Description
    ///
    /// Returns the thread identifier of the destination.
    ///
    pub fn destination_tid(&self) -> ThreadIdentifier {
        ThreadIdentifier::from(self.destination_tid)
    }

    ///
    /// # Description
    ///
    /// Returns the guest physical address of the bulk data buffer.
    ///
    pub fn data_addr(&self) -> u32 {
        self.data_addr
    }

    ///
    /// # Description
    ///
    /// Returns the number of bytes in the bulk payload.
    ///
    pub fn data_len(&self) -> u32 {
        self.data_len
    }

    ///
    /// # Description
    ///
    /// Converts the target data chunk transfer header to a byte array.
    ///
    /// # Returns
    ///
    /// A byte array that represents the target data chunk transfer header.
    ///
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        unsafe { mem::transmute(self) }
    }

    ///
    /// # Description
    ///
    /// Attempts to convert a byte array to a data chunk transfer header.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array to convert.
    ///
    /// # Returns
    ///
    /// Upon success, the data chunk transfer header is returned. Upon failure, an error is returned
    /// instead.
    ///
    /// # Notes
    ///
    /// This function currently cannot fail because all bit patterns are valid for the `repr(C)`
    /// layout. The `Result` return type is retained for forward compatibility in case field
    /// validation is added in the future.
    ///
    pub fn try_from_bytes(bytes: [u8; Self::SIZE]) -> Result<Self, Error> {
        Ok(unsafe { mem::transmute::<[u8; Self::SIZE], DataChunkHeader>(bytes) })
    }
}
