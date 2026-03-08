// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Main
//==================================================================================================

fn main() {
    // Intentionally left empty. The standalone rootfs image is now produced
    // by the `mkramfs` host utility, invoked from the build system
    // (`build/make/nanvixd.mk`) instead of being generated here.
}
