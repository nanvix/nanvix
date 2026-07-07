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
    mm::{
        Address,
        Alignment,
        VirtualAddress,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    time::NANOSECONDS_PER_SECOND,
};
use ::core::{
    mem,
    time::Duration,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Page size, in bytes, used to reason about scatter/gather bulk transfer limits.
///
/// Scatter/gather chains describe guest memory at page granularity, so the limits below are
/// expressed as multiples of this value rather than as opaque byte counts. Sourced from
/// [`Alignment`] (the same canonical definition that the `arch` crate's page alignment resolves to)
/// because the `sys` crate cannot depend on the `arch` crate: `arch` already depends on `sys`, so
/// the reverse dependency would form a cycle.
const PAGE_SIZE: usize = Alignment::Align4096 as usize;

/// Largest single allocation, in bytes, that the guest kernel's slab heap allocator can satisfy.
///
/// The kernel assembles the scatter/gather descriptor list for a transfer as one contiguous heap
/// allocation, and its slab allocator rejects any request larger than this. Sourced from the shared
/// [`config::kernel::MAX_SLAB_SIZE`] budget (the same value the kernel heap enforces in
/// `kernel::mm::kheap`) so the kernel can always build the descriptor chain without overflowing the
/// heap; reading the same constant keeps the two in sync.
const SG_BULK_MAX_DESCRIPTOR_BYTES: usize = config::kernel::MAX_SLAB_SIZE;

/// Maximum number of segments in a scatter/gather bulk transfer.
///
/// In the worst case every page of a transfer is physically discontiguous and needs its own segment
/// descriptor, so the kernel builds up to this many [`GuestSgSegment`] descriptors in a single
/// heap allocation. The limit is therefore the number of fixed-size descriptors that fit within
/// [`SG_BULK_MAX_DESCRIPTOR_BYTES`]; raising that heap budget is required before this can grow.
///
/// Typed as `u16` to match the wire representation of a segment count, so call sites do not need a
/// narrowing `as` cast. The single narrowing cast below is a compile-time constant that is
/// provably lossless (the descriptor budget is far smaller than `u16::MAX`).
pub const SG_BULK_MAX_SEGMENTS: u16 = (SG_BULK_MAX_DESCRIPTOR_BYTES / GuestSgSegment::SIZE) as u16;

/// Maximum number of bytes in a scatter/gather bulk transfer.
///
/// Guideline: this is derived from [`SG_BULK_MAX_SEGMENTS`] assuming the worst case of one page per
/// segment, so the byte ceiling is the segment ceiling times the page size. Keeping it a multiple
/// of [`PAGE_SIZE`] ensures the two limits stay consistent. Larger user transfers are split
/// into several bulk transfers by the read/write chunking path, so this bounds a single transfer
/// rather than the total amount a caller may move.
pub const SG_BULK_MAX_BYTES: usize = SG_BULK_MAX_SEGMENTS as usize * PAGE_SIZE;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Identity of the process and thread that originated a message.
///
/// Both halves are carried explicitly: `pid` attributes the message to a process (used by servers
/// that key per-process state, such as vfsd), while `tid` names the originating thread (or
/// [`ThreadIdentifier::NONE`] when unspecified). The kernel stamps the authoritative identity on
/// every message that passes through the `send` kernel call (see `src/kernel/src/ipc/send.rs`), so
/// a server may trust these fields instead of reconstructing identity from a sign-encoded value.
/// This is correct across `fork()` + `execv()` (which keeps the process identifier but installs a
/// new main-thread thread identifier, so `TID != PID`) and for requests issued by non-main threads
/// of a multi-threaded caller (nanvix/nanvix#2650, nanvix/nanvix#2529).
///
/// # Notes
///
/// - All fields are intentionally public to enable zero-copy message parsing.
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct MessageSender {
    /// Process that originated the message.
    pub pid: ProcessIdentifier,
    /// Thread that originated the message, or [`ThreadIdentifier::NONE`] when unspecified.
    pub tid: ThreadIdentifier,
}
::static_assert::assert_eq_size!(MessageSender, 2 * mem::size_of::<u32>());

impl MessageSender {
    /// The size of a message sender, in bytes.
    pub const SIZE: usize = mem::size_of::<Self>();

    /// The kernel process is the sender of the message.
    pub const KERNEL: Self = Self::new(ProcessIdentifier::KERNEL, ThreadIdentifier::NONE);
    /// The memory management daemon is the sender of the message (standalone mode only).
    /// NOTE: Aliases [`Self::NETWORKD`] — these are mutually exclusive deployment modes.
    pub const MEMD: Self = Self::new(ProcessIdentifier::MEMD, ThreadIdentifier::NONE);
    /// The network daemon is the sender of the message (hosted mode only).
    /// NOTE: Aliases [`Self::MEMD`] — these are mutually exclusive deployment modes.
    pub const NETWORKD: Self = Self::new(ProcessIdentifier::NETWORKD, ThreadIdentifier::NONE);
    /// The VFS daemon is the sender of the message.
    pub const VFSD: Self = Self::new(ProcessIdentifier::VFSD, ThreadIdentifier::NONE);

    /// Creates a message sender from an explicit process and thread identifier.
    pub const fn new(pid: ProcessIdentifier, tid: ThreadIdentifier) -> Self {
        Self { pid, tid }
    }
}

///
/// # Description
///
/// Destination of a message, naming the receiving process and optionally a specific thread.
///
/// When `tid` is [`ThreadIdentifier::NONE`] the message is delivered to the process mailbox of
/// `pid` (any thread may consume it); otherwise it is delivered to the named thread. This explicit
/// tuple replaces the previous sign-encoded routing discriminator.
///
/// # Notes
///
/// - All fields are intentionally public to enable zero-copy message parsing.
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct MessageReceiver {
    /// Process that should receive the message.
    pub pid: ProcessIdentifier,
    /// Thread that should receive the message, or [`ThreadIdentifier::NONE`] for the process
    /// mailbox.
    pub tid: ThreadIdentifier,
}
::static_assert::assert_eq_size!(MessageReceiver, 2 * mem::size_of::<u32>());

impl MessageReceiver {
    /// The size of a message receiver, in bytes.
    pub const SIZE: usize = mem::size_of::<Self>();

    /// The kernel process is the receiver of the message.
    pub const KERNEL: Self = Self::new(ProcessIdentifier::KERNEL, ThreadIdentifier::NONE);
    /// The memory management daemon is the receiver of the message (standalone mode only).
    /// NOTE: Aliases [`Self::NETWORKD`] — these are mutually exclusive deployment modes.
    pub const MEMD: Self = Self::new(ProcessIdentifier::MEMD, ThreadIdentifier::NONE);
    /// The network daemon is the receiver of the message (hosted mode only).
    /// NOTE: Aliases [`Self::MEMD`] — these are mutually exclusive deployment modes.
    pub const NETWORKD: Self = Self::new(ProcessIdentifier::NETWORKD, ThreadIdentifier::NONE);
    /// The VFS daemon is the receiver of the message.
    pub const VFSD: Self = Self::new(ProcessIdentifier::VFSD, ThreadIdentifier::NONE);

    /// Creates a message receiver from an explicit process and thread identifier.
    pub const fn new(pid: ProcessIdentifier, tid: ThreadIdentifier) -> Self {
        Self { pid, tid }
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
    /// The size of the message header fields (type, source, destination and status).
    pub const HEADER_SIZE: usize =
        MessageType::SIZE + MessageSender::SIZE + MessageReceiver::SIZE + mem::size_of::<i32>();
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
        MessageType::try_from_bytes([bytes[0]])?;
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
    /// Type of message carried by this envelope (stored as `u64`).
    kind: u64,
    /// Guest virtual address of the message (stored as `u64`, logical type is `u32`).
    message_addr: u64,
}
::static_assert::assert_eq_size!(VmBusMessage, 3 * mem::size_of::<u64>());

///
/// # Description
///
/// Message kind carried by a [`VmBusMessage`].
///
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmBusMessageKind {
    /// Legacy contiguous bulk transfer descriptor.
    LegacyBulk = 0,
    /// Standard IKC message.
    Ikc = 1,
    /// Guest scatter/gather bulk transfer descriptor.
    GuestSgBulk = 2,
    /// Kernel-log block. Carries a contiguous run of console bytes flushed by the kernel log
    /// buffer in a single transfer (delivered on `DEFAULT_KLOG_PORT`), so a whole buffer flush
    /// costs one VM exit instead of one per byte.
    KlogBlock = 3,
}

impl TryFrom<u64> for VmBusMessageKind {
    type Error = Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        // Match against the enum discriminants themselves so the wire values have a single source
        // of truth and are not duplicated as literals here.
        match value {
            value if value == Self::LegacyBulk as u64 => Ok(Self::LegacyBulk),
            value if value == Self::Ikc as u64 => Ok(Self::Ikc),
            value if value == Self::GuestSgBulk as u64 => Ok(Self::GuestSgBulk),
            value if value == Self::KlogBlock as u64 => Ok(Self::KlogBlock),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid vmbus message kind")),
        }
    }
}

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
    /// - `kind`: Type of message carried by this envelope.
    /// - `message_addr`: Guest virtual address of the message.
    ///
    /// # Returns
    ///
    /// The new message envelope.
    ///
    pub fn new(size: u32, kind: VmBusMessageKind, message_addr: u32) -> Self {
        Self {
            size: size as u64,
            kind: kind as u64,
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
    /// Returns the type of message carried by this envelope.
    ///
    /// # Returns
    ///
    /// Upon success, the message kind is returned. Upon failure, an error is returned instead.
    ///
    /// # Errors
    ///
    /// This function returns an error if the envelope contains an unknown message kind.
    ///
    pub fn kind(&self) -> Result<VmBusMessageKind, Error> {
        VmBusMessageKind::try_from(self.kind)
    }

    ///
    /// # Description
    ///
    /// Returns whether this is an IKC message.
    ///
    pub fn is_ikc(&self) -> bool {
        self.kind == VmBusMessageKind::Ikc as u64
    }

    ///
    /// # Description
    ///
    /// Sets the type of message carried by this envelope.
    ///
    /// # Parameters
    ///
    /// - `kind`: Type of message carried by this envelope.
    ///
    pub fn set_kind(&mut self, kind: VmBusMessageKind) {
        self.kind = kind as u64;
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
        // SAFETY: The header is `repr(C)` and contains only fixed-width integer fields, so its
        // in-memory representation is exactly `Self::SIZE` bytes.
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
// HostBulkTransferHeader
//==================================================================================================

///
/// # Description
///
/// Header structure describing a contiguous data chunk transfer between UserVM and linuxd.
///
/// # Notes
///
/// - All fields use fixed-width types for ABI stability across the guest/host boundary.
/// - The `data_addr` field stores an opaque UserVM value. It may be a guest physical address on
///   legacy paths, or a UserVM transfer identifier for scatter/gather pull responses.
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct HostBulkTransferHeader {
    /// Process identifier of the source (sender), stored as a fixed-width `i32`.
    source_pid: i32,
    /// Thread identifier of the source (sender), stored as a fixed-width `i32`.
    source_tid: i32,
    /// Process identifier of the destination (receiver), stored as a fixed-width `i32`.
    destination_pid: i32,
    /// Thread identifier of the destination (receiver), stored as a fixed-width `i32`.
    destination_tid: i32,
    /// Opaque bulk payload location.
    ///
    /// This is a guest physical address on legacy contiguous paths and a UserVM transfer identifier
    /// on scatter/gather pull responses.
    data_addr: u32,
    /// Number of bytes in the bulk payload.
    data_len: u32,
}
::static_assert::assert_eq_size!(HostBulkTransferHeader, 6 * mem::size_of::<u32>());

//==================================================================================================
// Implementations
//==================================================================================================

impl HostBulkTransferHeader {
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
    /// - `data_addr`: Guest physical address or opaque UserVM transfer identifier.
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
    /// Returns the guest physical address or opaque UserVM transfer identifier.
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
        // SAFETY: All bit patterns are valid because the header contains only fixed-width integer
        // fields.
        Ok(unsafe { mem::transmute::<[u8; Self::SIZE], HostBulkTransferHeader>(bytes) })
    }
}

/// Backwards-compatible name for the contiguous UserVM/linuxd bulk header.
pub type DataChunkHeader = HostBulkTransferHeader;

///
/// # Description
///
/// Kind of guest scatter/gather bulk transfer.
///
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestSgBulkKind {
    /// UserVM should gather guest memory and send it to linuxd.
    Push = 1,
    /// UserVM should register guest memory as the destination for a later linuxd response.
    Pull = 2,
}

impl TryFrom<u16> for GuestSgBulkKind {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        // Match against the enum discriminants themselves so the wire values have a single source
        // of truth and are not duplicated as literals here.
        match value {
            value if value == Self::Push as u16 => Ok(Self::Push),
            value if value == Self::Pull as u16 => Ok(Self::Pull),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid scatter/gather kind")),
        }
    }
}

///
/// # Description
///
/// A guest physical memory segment in a scatter/gather bulk transfer.
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GuestSgSegment {
    /// Guest virtual address of the next segment descriptor, or zero if this is the last segment.
    ///
    /// Stored as a fixed-width `u32` for ABI stability across the guest/host boundary; the typed
    /// [`VirtualAddress`] accessors convert to and from this representation.
    next: u32,
    /// Guest physical address of this segment's data.
    data_gpa: u32,
    /// Number of bytes in this segment.
    data_len: u32,
}
::static_assert::assert_eq_size!(GuestSgSegment, 3 * mem::size_of::<u32>());

impl GuestSgSegment {
    /// Size of the segment descriptor in bytes.
    pub const SIZE: usize = mem::size_of::<Self>();

    ///
    /// # Description
    ///
    /// Creates a new guest scatter/gather segment.
    ///
    /// # Parameters
    ///
    /// - `next`: Guest virtual address of the next segment descriptor, or zero for the last
    ///   segment.
    /// - `data_gpa`: Guest physical address of this segment's data.
    /// - `data_len`: Number of bytes in this segment.
    ///
    /// # Returns
    ///
    /// The new guest scatter/gather segment.
    ///
    pub fn new(next: VirtualAddress, data_gpa: u32, data_len: u32) -> Self {
        Self {
            next: next.into_raw_value() as u32,
            data_gpa,
            data_len,
        }
    }

    /// Returns the guest virtual address of the next descriptor.
    pub fn next(&self) -> VirtualAddress {
        VirtualAddress::from_raw_value(self.next as usize)
    }

    /// Sets the guest virtual address of the next descriptor.
    pub fn set_next(&mut self, next: VirtualAddress) {
        self.next = next.into_raw_value() as u32;
    }

    /// Returns the guest physical address of this segment's data.
    pub fn data_gpa(&self) -> u32 {
        self.data_gpa
    }

    /// Returns the number of bytes in this segment.
    pub fn data_len(&self) -> u32 {
        self.data_len
    }

    /// Sets the number of bytes in this segment.
    pub fn set_data_len(&mut self, data_len: u32) {
        self.data_len = data_len;
    }

    ///
    /// # Description
    ///
    /// Converts the segment descriptor to a byte array.
    ///
    /// # Returns
    ///
    /// A byte array that represents the segment descriptor.
    ///
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        // SAFETY: The segment is `repr(C)` and contains only fixed-width integer fields, so its
        // in-memory representation is exactly `Self::SIZE` bytes.
        unsafe { mem::transmute(self) }
    }

    ///
    /// # Description
    ///
    /// Attempts to convert a byte array to a segment descriptor.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array to convert.
    ///
    /// # Returns
    ///
    /// Upon success, the segment descriptor is returned. Upon failure, an error is returned
    /// instead.
    ///
    /// # Errors
    ///
    /// This function currently cannot fail because all bit patterns are valid for the `repr(C)`
    /// layout. The `Result` return type is retained for forward compatibility in case field
    /// validation is added in the future.
    ///
    pub fn try_from_bytes(bytes: [u8; Self::SIZE]) -> Result<Self, Error> {
        // SAFETY: All bit patterns are valid because the segment contains only fixed-width integer
        // fields.
        Ok(unsafe { mem::transmute::<[u8; Self::SIZE], GuestSgSegment>(bytes) })
    }
}

///
/// # Description
///
/// Number of segments in a scatter/gather bulk transfer.
///
/// This is a thin newtype over `u16` (the wire representation) that provides stronger type safety
/// than a bare integer when describing the length of a scatter/gather segment chain.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SegmentCount(u16);

impl SegmentCount {
    /// Maximum number of segments a scatter/gather bulk transfer may contain.
    pub const MAX: Self = Self(SG_BULK_MAX_SEGMENTS);

    ///
    /// # Description
    ///
    /// Creates a new [`SegmentCount`] from a raw count.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of segments.
    ///
    /// # Returns
    ///
    /// The new segment count.
    ///
    pub fn new(count: u16) -> Self {
        Self(count)
    }

    ///
    /// # Description
    ///
    /// Returns the raw number of segments.
    ///
    pub fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<usize> for SegmentCount {
    type Error = Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value == 0 || value > SG_BULK_MAX_SEGMENTS as usize {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "invalid scatter/gather segment count",
            ));
        }
        let count: u16 = u16::try_from(value).map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, "too many scatter/gather segments")
        })?;
        Ok(Self(count))
    }
}

///
/// # Description
///
/// Guest scatter/gather bulk transfer descriptor consumed by UserVM.
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GuestSgBulkHeader {
    /// Process identifier of the source.
    source_pid: i32,
    /// Thread identifier of the source.
    source_tid: i32,
    /// Process identifier of the destination.
    destination_pid: i32,
    /// Thread identifier of the destination.
    destination_tid: i32,
    /// Scatter/gather transfer kind.
    kind: u16,
    /// Number of segments in the chain, including `first`.
    segment_count: u16,
    /// Total number of bytes in the transfer.
    total_len: u32,
    /// First segment descriptor.
    first: GuestSgSegment,
}
::static_assert::assert_eq_size!(GuestSgBulkHeader, 9 * mem::size_of::<u32>());

impl GuestSgBulkHeader {
    /// Size of the scatter/gather bulk header in bytes.
    pub const SIZE: usize = mem::size_of::<Self>();

    ///
    /// # Description
    ///
    /// Creates a new guest scatter/gather bulk header.
    ///
    /// # Parameters
    ///
    /// - `source`: Process and thread identifiers of the source.
    /// - `destination`: Process and thread identifiers of the destination.
    /// - `kind`: Type of scatter/gather transfer.
    /// - `segment_count`: Number of segments in the chain, including `first`.
    /// - `total_len`: Total number of bytes in the transfer.
    /// - `first`: First segment descriptor.
    ///
    /// # Returns
    ///
    /// The new guest scatter/gather bulk header.
    ///
    pub fn new(
        source: (ProcessIdentifier, ThreadIdentifier),
        destination: (ProcessIdentifier, ThreadIdentifier),
        kind: GuestSgBulkKind,
        segment_count: SegmentCount,
        total_len: u32,
        first: GuestSgSegment,
    ) -> Self {
        let (source_pid, source_tid) = source;
        let (destination_pid, destination_tid) = destination;
        Self {
            source_pid: source_pid.into(),
            source_tid: source_tid.into(),
            destination_pid: destination_pid.into(),
            destination_tid: destination_tid.into(),
            kind: kind as u16,
            segment_count: segment_count.get(),
            total_len,
            first,
        }
    }

    /// Returns the process identifier of the source.
    pub fn source_pid(&self) -> ProcessIdentifier {
        ProcessIdentifier::from(self.source_pid)
    }

    /// Returns the thread identifier of the source.
    pub fn source_tid(&self) -> ThreadIdentifier {
        ThreadIdentifier::from(self.source_tid)
    }

    /// Returns the process identifier of the destination.
    pub fn destination_pid(&self) -> ProcessIdentifier {
        ProcessIdentifier::from(self.destination_pid)
    }

    /// Returns the thread identifier of the destination.
    pub fn destination_tid(&self) -> ThreadIdentifier {
        ThreadIdentifier::from(self.destination_tid)
    }

    ///
    /// # Description
    ///
    /// Returns the scatter/gather transfer kind.
    ///
    /// # Returns
    ///
    /// Upon success, the scatter/gather transfer kind is returned. Upon failure, an error is
    /// returned instead.
    ///
    /// # Errors
    ///
    /// This function returns an error if the header contains an unknown scatter/gather transfer
    /// kind.
    ///
    pub fn kind(&self) -> Result<GuestSgBulkKind, Error> {
        GuestSgBulkKind::try_from(self.kind)
    }

    /// Returns the number of segments in the chain.
    pub fn segment_count(&self) -> SegmentCount {
        SegmentCount::new(self.segment_count)
    }

    /// Returns the total number of bytes in the transfer.
    pub fn total_len(&self) -> u32 {
        self.total_len
    }

    /// Returns the first segment descriptor.
    pub fn first(&self) -> GuestSgSegment {
        self.first
    }

    ///
    /// # Description
    ///
    /// Converts the scatter/gather bulk header to a byte array.
    ///
    /// # Returns
    ///
    /// A byte array that represents the scatter/gather bulk header.
    ///
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        // SAFETY: The header is `repr(C)` and contains only fixed-width integer fields plus a
        // fixed-layout segment, so its in-memory representation is exactly `Self::SIZE` bytes.
        unsafe { mem::transmute(self) }
    }

    ///
    /// # Description
    ///
    /// Attempts to convert a byte array to a scatter/gather bulk header.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array to convert.
    ///
    /// # Returns
    ///
    /// Upon success, the scatter/gather bulk header is returned. Upon failure, an error is returned
    /// instead.
    ///
    /// # Errors
    ///
    /// This function currently cannot fail because all bit patterns are valid for the `repr(C)`
    /// layout. The `Result` return type is retained for forward compatibility in case field
    /// validation is added in the future.
    ///
    pub fn try_from_bytes(bytes: [u8; Self::SIZE]) -> Result<Self, Error> {
        // SAFETY: All bit patterns are valid because the header contains only fixed-width integer
        // fields and a segment whose bit patterns are all valid.
        Ok(unsafe { mem::transmute::<[u8; Self::SIZE], GuestSgBulkHeader>(bytes) })
    }
}

///
/// # Description
///
/// Optional timeout for a rendezvous push/pull kernel call.
///
/// `push`/`pull` bound their blocking wait with the three-point spectrum common to
/// synchronous-rendezvous microkernels:
///
/// - **Infinite** (`kind == KIND_INFINITE`): block until the counterpart arrives. This is the
///   historical, unbounded behavior.
/// - **Finite** (`kind == KIND_FINITE`): block for at most `secs` seconds plus `nanos` nanoseconds.
///   A finite timeout of zero duration is a non-blocking probe that never sleeps.
///
/// This is a plain-old-data encoding carried across the kernel-call boundary inside [`PushArgs`]
/// and [`PullArgs`]. All fields are fixed-width and naturally aligned, so the layout is identical on
/// the 32-bit guest and the kernel.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Timeout {
    /// Discriminant selecting the timeout mode: [`Self::KIND_INFINITE`] or [`Self::KIND_FINITE`].
    kind: u32,
    /// Seconds component of a finite timeout.
    secs: u32,
    /// Nanoseconds component of a finite timeout.
    nanos: u32,
}
::static_assert::assert_eq_size!(Timeout, 3 * mem::size_of::<u32>());

impl Timeout {
    /// Discriminant value for an infinite (unbounded) timeout.
    const KIND_INFINITE: u32 = 0;

    /// Discriminant value for a finite (bounded) timeout.
    const KIND_FINITE: u32 = 1;

    /// Size of the descriptor in bytes.
    pub const SIZE: usize = mem::size_of::<Self>();

    ///
    /// # Description
    ///
    /// Creates an infinite timeout, i.e. one that blocks until the counterpart arrives.
    ///
    /// # Returns
    ///
    /// The new infinite timeout.
    ///
    pub const fn infinite() -> Self {
        Self {
            kind: Self::KIND_INFINITE,
            secs: 0,
            nanos: 0,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a finite timeout of the given duration. A zero duration is a non-blocking probe.
    ///
    /// # Parameters
    ///
    /// - `secs`: Seconds component of the timeout.
    /// - `nanos`: Nanoseconds component of the timeout.
    ///
    /// # Returns
    ///
    /// The new finite timeout.
    ///
    pub const fn finite(secs: u32, nanos: u32) -> Self {
        Self {
            kind: Self::KIND_FINITE,
            secs,
            nanos,
        }
    }

    ///
    /// # Description
    ///
    /// Creates a timeout from an optional [`Duration`]: [`None`] yields an infinite timeout, while
    /// [`Some`] yields a finite one. A duration whose whole-seconds component exceeds [`u32::MAX`]
    /// is saturated, which still bounds the wait far beyond any practical deadline.
    ///
    /// # Parameters
    ///
    /// - `timeout`: The optional duration to convert.
    ///
    /// # Returns
    ///
    /// The corresponding timeout.
    ///
    pub fn from_duration(timeout: Option<Duration>) -> Self {
        match timeout {
            None => Self::infinite(),
            Some(duration) => {
                let secs: u32 = u32::try_from(duration.as_secs()).unwrap_or(u32::MAX);
                Self::finite(secs, duration.subsec_nanos())
            },
        }
    }

    ///
    /// # Description
    ///
    /// Returns the finite duration components `(secs, nanos)`, or [`None`] if the timeout is
    /// infinite.
    ///
    /// # Errors
    ///
    /// This function returns an error if the descriptor carries an unknown timeout kind or an
    /// invalid nanosecond component.
    ///
    pub fn as_finite(&self) -> Result<Option<(u32, u32)>, Error> {
        match self.kind {
            Self::KIND_INFINITE => Ok(None),
            Self::KIND_FINITE if self.nanos < NANOSECONDS_PER_SECOND => {
                Ok(Some((self.secs, self.nanos)))
            },
            Self::KIND_FINITE => Err(Error::new(
                ErrorCode::InvalidArgument,
                "timeout nanoseconds component is out of range",
            )),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid timeout kind")),
        }
    }
}

///
/// # Description
///
/// Arguments for a rendezvous push kernel call, passed by pointer.
///
/// `push` and `pull` already consume all four kernel-call argument registers for their mandatory
/// operands, leaving no register for the optional [`Timeout`]. The operands are therefore gathered
/// into this descriptor, which the guest builds on its stack and passes by address; the kernel
/// copies it into kernel space before use, exactly as it does for a [`Message`]. All fields are
/// fixed-width and naturally aligned, so the layout is identical on the 32-bit guest and the kernel.
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PushArgs {
    /// Destination process identifier.
    pub dst_pid: ProcessIdentifier,
    /// Destination thread identifier.
    pub dst_tid: ThreadIdentifier,
    /// Address of the source buffer in the caller's user space.
    pub buffer: u32,
    /// Number of bytes to transfer.
    pub len: u32,
    /// Optional timeout that bounds the blocking wait.
    pub timeout: Timeout,
}
::static_assert::assert_eq_size!(PushArgs, 7 * mem::size_of::<u32>());

impl PushArgs {
    /// Size of the descriptor in bytes.
    pub const SIZE: usize = mem::size_of::<Self>();

    ///
    /// # Description
    ///
    /// Creates a zeroed descriptor suitable as a destination for a copy from user space. The
    /// identifier fields are set to the kernel sentinels (raw value zero); every field is
    /// overwritten by the copy before use.
    ///
    /// # Returns
    ///
    /// The zeroed descriptor.
    ///
    pub const fn zeroed() -> Self {
        Self {
            dst_pid: ProcessIdentifier::KERNEL,
            dst_tid: ThreadIdentifier::KERNEL,
            buffer: 0,
            len: 0,
            timeout: Timeout::infinite(),
        }
    }
}

///
/// # Description
///
/// Arguments for a rendezvous pull kernel call, passed by pointer.
///
/// This is the receive-side counterpart of [`PushArgs`]; see that type for the rationale behind the
/// pointer-passed descriptor. All fields are fixed-width and naturally aligned, so the layout is
/// identical on the 32-bit guest and the kernel.
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PullArgs {
    /// Source (expected sender) process identifier.
    pub src_pid: ProcessIdentifier,
    /// Source (expected sender) thread identifier.
    pub src_tid: ThreadIdentifier,
    /// Address of the destination buffer in the caller's user space.
    pub buffer: u32,
    /// Maximum number of bytes to receive.
    pub len: u32,
    /// Optional timeout that bounds the blocking wait.
    pub timeout: Timeout,
}
::static_assert::assert_eq_size!(PullArgs, 7 * mem::size_of::<u32>());

impl PullArgs {
    /// Size of the descriptor in bytes.
    pub const SIZE: usize = mem::size_of::<Self>();

    ///
    /// # Description
    ///
    /// Creates a zeroed descriptor suitable as a destination for a copy from user space. The
    /// identifier fields are set to the kernel sentinels (raw value zero); every field is
    /// overwritten by the copy before use.
    ///
    /// # Returns
    ///
    /// The zeroed descriptor.
    ///
    pub const fn zeroed() -> Self {
        Self {
            src_pid: ProcessIdentifier::KERNEL,
            src_tid: ThreadIdentifier::KERNEL,
            buffer: 0,
            len: 0,
            timeout: Timeout::infinite(),
        }
    }
}
