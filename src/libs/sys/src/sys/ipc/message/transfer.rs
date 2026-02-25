// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ipc::{
    DataChunkHeader,
    Message,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Represents a data chunk transfer between a user process and the kernel (linuxd). This structure
/// pairs the fixed-size [`DataChunkHeader`] with a variable-length data buffer that holds the
/// actual payload bytes.
///
/// # Notes
///
/// - This type is only available when the `std` feature is enabled because it uses a heap-allocated
///   `Vec<u8>` for the data payload.
///
#[derive(Debug, Clone)]
pub struct DataChunk {
    /// Header containing metadata about the transfer.
    header: DataChunkHeader,
    /// Variable-length payload data.
    data: Vec<u8>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl DataChunk {
    ///
    /// # Description
    ///
    /// Creates a new data chunk transfer.
    ///
    /// # Parameters
    ///
    /// - `header`: Metadata describing the transfer endpoints and size.
    /// - `data`: The bulk payload bytes.
    ///
    /// # Returns
    ///
    /// The new data chunk transfer.
    ///
    pub fn new(header: DataChunkHeader, data: Vec<u8>) -> Self {
        Self { header, data }
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the transfer header.
    ///
    pub fn header(&self) -> &DataChunkHeader {
        &self.header
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the payload data.
    ///
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    ///
    /// # Description
    ///
    /// Consumes the data chunk transfer and returns the payload data.
    ///
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    ///
    /// # Description
    ///
    /// Returns a mutable reference to the payload data.
    ///
    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    ///
    /// # Description
    ///
    /// Serializes the data chunk transfer into a byte vector. The format is: header bytes followed by
    /// data bytes.
    ///
    /// # Returns
    ///
    /// A byte vector containing the serialized data chunk transfer.
    ///
    pub fn to_bytes(&self) -> Vec<u8> {
        let header_bytes: [u8; DataChunkHeader::SIZE] = self.header.to_bytes();
        let mut bytes: Vec<u8> = Vec::with_capacity(DataChunkHeader::SIZE + self.data.len());
        bytes.extend_from_slice(&header_bytes);
        bytes.extend_from_slice(&self.data);
        bytes
    }

    ///
    /// # Description
    ///
    /// Attempts to deserialize a data chunk transfer from a byte slice. The byte slice must contain at
    /// least [`DataChunkHeader::SIZE`] bytes for the header, followed by the payload data.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte slice to deserialize from.
    ///
    /// # Returns
    ///
    /// Upon success, the deserialized data chunk transfer is returned. Upon failure, an error is
    /// returned instead.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice is too short to contain a valid header.
    ///
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, crate::error::Error> {
        if bytes.len() < DataChunkHeader::SIZE {
            return Err(crate::error::Error::new(
                crate::error::ErrorCode::InvalidArgument,
                "byte slice too short for data chunk transfer header",
            ));
        }
        let mut header_bytes: [u8; DataChunkHeader::SIZE] = [0u8; DataChunkHeader::SIZE];
        header_bytes.copy_from_slice(&bytes[..DataChunkHeader::SIZE]);
        let header: DataChunkHeader = DataChunkHeader::try_from_bytes(header_bytes)?;
        let data: Vec<u8> = bytes[DataChunkHeader::SIZE..].to_vec();
        Ok(Self { header, data })
    }
}

//==================================================================================================
// Transfer Enum
//==================================================================================================

///
/// # Description
///
/// An enumeration representing either a standard IPC [`Message`] or a [`DataChunk`]. This
/// allows channels and I/O paths to carry both message types without separate plumbing.
///
/// # Notes
///
/// - This type is only available when the `std` feature is enabled.
///
#[derive(Debug, Clone)]
pub enum IkcFrame {
    /// A standard fixed-size IPC message.
    Message(Message),
    /// A variable-length data chunk transfer.
    Bulk(DataChunk),
}

//==================================================================================================
// Transfer Implementations
//==================================================================================================

impl IkcFrame {
    /// Wire byte identifying a standard IPC message frame.
    pub const MESSAGE_FRAME: u8 = 0x01;
    /// Wire byte identifying a data chunk transfer frame.
    pub const DATA_CHUNK_FRAME: u8 = 0x02;

    ///
    /// # Description
    ///
    /// Returns the wire discriminator byte for this transfer variant.
    ///
    pub fn frame_type_byte(&self) -> u8 {
        match self {
            IkcFrame::Message(_) => Self::MESSAGE_FRAME,
            IkcFrame::Bulk(_) => Self::DATA_CHUNK_FRAME,
        }
    }
}
