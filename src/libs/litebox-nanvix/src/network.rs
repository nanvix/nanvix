// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::NanvixUserland;

//==================================================================================================
// Implementations
//==================================================================================================

impl litebox::platform::IPInterfaceProvider for NanvixUserland {
    ///
    /// # Description
    ///
    /// Sends an IP packet.
    ///
    /// # Parameters
    ///
    /// - `packet`: The packet to send.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
    ///
    fn send_ip_packet(&self, _packet: &[u8]) -> Result<(), litebox::platform::SendError> {
        unimplemented!("send_ip_packet(): not supported")
    }

    ///
    ///
    /// # Description
    ///
    /// Receives an IP packet.
    ///
    /// # Parameters
    ///
    /// - `packet`: The packet to receive.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the number of bytes received is returned. Upon failure, an error
    /// is returned instead.
    ///
    fn receive_ip_packet(
        &self,
        _packet: &mut [u8],
    ) -> Result<usize, litebox::platform::ReceiveError> {
        unimplemented!("recive_ip_packet(): not supported")
    }
}
