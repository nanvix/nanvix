// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    super::Message,
    identifier::RequestIdentifier,
};
use crate::pm::ProcessIdentifier;

//==================================================================================================
// Enumerations
//==================================================================================================

/// Classification of one response received while waiting for a request token.
pub enum ResponseDisposition {
    /// The response matches the request currently waiting for it.
    Matched(Message),
    /// The response belongs to another active request and was stashed.
    Stashed,
    /// The response does not belong to any active request and must be logged and dropped.
    Stale(RequestIdentifier),
    /// The response carries an active identifier, but came from the wrong process.
    UnexpectedSource {
        /// Identifier carried by the response.
        identifier: RequestIdentifier,
        /// Process expected to answer the request.
        expected: ProcessIdentifier,
        /// Process that sent the response.
        actual: ProcessIdentifier,
    },
}
