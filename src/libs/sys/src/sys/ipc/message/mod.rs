// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod kernel;
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
pub use system::{
    SystemMessage,
    SystemMessageHeader,
};
#[cfg(feature = "std")]
pub use transfer::{
    DataChunk,
    IkcFrame,
};
