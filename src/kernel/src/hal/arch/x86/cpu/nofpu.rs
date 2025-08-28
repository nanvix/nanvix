// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure represents a dummy FPU state for systems without FPU support.
///
#[derive(Clone, Copy, Debug)]
pub struct FpuState;

//==================================================================================================
// Implementations
//==================================================================================================

impl FpuState {
    ///
    /// # Description
    ///
    /// Constructs a dummy FPU state.
    ///
    /// # Returns
    ///
    /// This function returns a new instance of a dummy FPU state.
    ///
    /// # Safety
    ///
    /// This function is marked as unsafe to match signature with the FPU-enabled version.
    ///
    /// When called on a system without FPU support, this function is safe
    ///
    pub unsafe fn new() -> Self {
        Self
    }
}
