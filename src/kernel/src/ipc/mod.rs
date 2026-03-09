// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//  Modules
//==================================================================================================

#[cfg(feature = "stdio")]
pub mod bulk_pull;
#[cfg(all(feature = "stdio", feature = "ring-buffer"))]
pub mod fixed_pull;
mod mbx;
mod pull;
mod push;
mod recv;
pub(crate) mod rendezvous;
mod send;

//==================================================================================================
//  Exports
//==================================================================================================

pub use mbx::Mailbox;
pub use pull::pull;
pub use push::push;
pub use recv::recv;
pub use send::send;
