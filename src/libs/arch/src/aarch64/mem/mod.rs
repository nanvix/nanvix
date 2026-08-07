// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[path = "../../x86/mem/constants.rs"]
mod constants;

// Nanvix keeps an architecture-independent software page map whose compact entry encoding predates
// the hardware page-table backends. AArch64 reuses that metadata representation; EL1 translation
// tables are maintained separately by the kernel HAL.
#[path = "../../x86/mem/paging/mod.rs"]
pub mod paging;

//==================================================================================================
// Exports
//==================================================================================================

pub use constants::*;
