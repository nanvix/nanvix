// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

cfg_if::cfg_if! {
    if #[cfg(feature = "hyperlight")] {
        mod hyperlight;
        pub use hyperlight::*;
    } else {
        mod microvm;
        pub use microvm::*;
    }
}
