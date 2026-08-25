// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//  Modules
//==================================================================================================

#[cfg(feature = "stdio")]
pub mod bulk_pull;
mod mbx;
mod pull;
mod push;
mod recv;
pub(crate) mod rendezvous;
mod send;
#[cfg(feature = "stdio")]
mod sg;

//==================================================================================================
//  Exports
//==================================================================================================

pub use mbx::Mailbox;
pub use pull::pull;
pub use push::push;
pub use recv::recv;
#[cfg(feature = "test")]
pub(crate) use recv::recv_with;
pub use send::send;
