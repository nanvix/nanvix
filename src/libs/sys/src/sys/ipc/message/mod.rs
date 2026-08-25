// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod kernel;
mod request_identifier_layout;
mod system;
#[cfg(feature = "std")]
mod transfer;

//==================================================================================================
// Exports
//==================================================================================================

pub use kernel::{
    DataChunkHeader,
    GuestSgBulkHeader,
    GuestSgBulkKind,
    GuestSgSegment,
    HostBulkTransferHeader,
    Message,
    MessageReceiver,
    MessageSender,
    PullArgs,
    PushArgs,
    SegmentCount,
    Timeout,
    VmBusMessage,
    VmBusMessageKind,
    SG_BULK_MAX_BYTES,
    SG_BULK_MAX_SEGMENTS,
};
pub(crate) use request_identifier_layout::REQUEST_IDENTIFIER_SIZE;
pub use request_identifier_layout::{
    REQUEST_IDENTIFIER_OFFSET,
    REQUEST_IDENTIFIER_PREFIX_SIZE,
};
pub use system::{
    SystemMessage,
    SystemMessageHeader,
};
#[cfg(feature = "std")]
pub use transfer::{
    DataChunk,
    IkcFrame,
};
