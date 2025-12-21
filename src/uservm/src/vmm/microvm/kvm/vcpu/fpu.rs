// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::kvm::vcpu::{
    serialize_fam_struct,
    serialize_plain,
};
use ::anyhow::Result;
use ::arch::cpu::mxcrs::{
    DenormalOperationMask,
    DivideByZeroMask,
    OverflowMask,
    PrecisionMask,
    UnderflowMask,
};
use ::kvm_bindings::{
    Xsave,
    kvm_fpu,
    kvm_xsave,
};
use ::kvm_ioctls::{
    Cap,
    Kvm,
    VcpuFd,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::mem;
use ::syslog::{
    error,
    trace,
};

//==================================================================================================
// Constants
//==================================================================================================

// Mask all FP exceptions, set rounding to nearest even, and set precision to 64-bit.
const FP_CONTROL_WORD_DEFAULT: u16 = 0x37f;
// All eight x87 FPU registers are marked empty.
const FP_TAG_WORD_DEFAULT: u8 = 0xff;

//==================================================================================================
// Structures
//==================================================================================================

/// FPU device.
pub struct Fpu {
    /// XSAVE2 size.
    xsave2_size: usize,
}

/// FPU state.
#[derive(Serialize, Deserialize)]
pub struct FpuState {
    /// XSAVE state.
    xsave_bytes: Vec<u8>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Fpu {
    ///
    /// # Description
    ///
    /// Creates a new FPU device.
    ///
    /// # Parameters
    ///
    /// - `kvm_fd`: Reference to the KVM device.
    /// - `vcpu_fd`: Reference to the VCPU device.
    ///
    /// # Return Value
    ///
    /// On success, this function returns a new FPU device. On failure, an error object that
    /// describes the error is returned instead.
    ///
    pub fn new(kvm_fd: &Kvm, vcpu_fd: &VcpuFd) -> Result<Fpu> {
        let xsave2_size = match kvm_fd.check_extension_int(Cap::Xsave2) {
            size if size > 0 => size as usize,
            _ => 0,
        };

        // Reset FPU state.
        let fpu: kvm_fpu = kvm_fpu {
            fcw: FP_CONTROL_WORD_DEFAULT,
            ftwx: FP_TAG_WORD_DEFAULT,
            // Mask all SIMD exceptions.
            mxcsr: (PrecisionMask::Masked as u32)
                | (UnderflowMask::Masked as u32)
                | (OverflowMask::Masked as u32)
                | (DivideByZeroMask::Masked as u32)
                | (DenormalOperationMask::Masked as u32),
            ..Default::default()
        };
        vcpu_fd.set_fpu(&fpu)?;

        Ok(Fpu { xsave2_size })
    }

    ///
    /// # Description
    ///
    /// Saves the state of the FPU device.
    ///
    /// # Parameters
    ///
    /// - `fd`: Reference to the VCPU device.
    ///
    /// # Return Value
    ///
    /// On success, this function returns the saved FPU state. On failure, an error object that
    /// describes the error is returned instead.
    ///
    pub fn save_state(&self, fd: &VcpuFd) -> Result<FpuState> {
        trace!("save_state()");

        // xsave can be either `Xsave` or `kvm_xsave`. Declaring it as `Vec<u8>` fits both.
        let bytes: Vec<u8> = if self.xsave2_size > 0 {
            // Fam-wrapper type Xsave is a wrapper over kvm_xsave2 (post-5.17) or kvm_xsave.
            let header_size: usize = mem::size_of::<kvm_bindings::kvm_xsave2>();
            let fam_entries: usize = self.xsave2_size.saturating_sub(header_size);
            // Each Fam entry in kvm_xsave2 is u32 (per bindings).
            let fam_units: usize = fam_entries.div_ceil(mem::size_of::<u32>());
            let mut xsave2: Xsave = match Xsave::new(fam_units) {
                Ok(v) => v,
                Err(error) => {
                    let reason: String = format!("failed creating xsave2 (error={error:?})");
                    error!("save_state(): {reason}");
                    anyhow::bail!(reason)
                },
            };
            // SAFETY: This is safe because we've checked the number of elements before allocating.
            if let Err(e) = unsafe { fd.get_xsave2(&mut xsave2) } {
                let reason: String = format!("failed getting xsave2 (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            }
            serialize_fam_struct(&xsave2)
        } else {
            // Older kernel that only supports fixed 4KB kvm_xsave.
            let small_xsave: kvm_xsave = match fd.get_xsave() {
                Ok(v) => v,
                Err(error) => {
                    let reason: String = format!("failed getting small_xsave (error={error:?})");
                    error!("save_state(): {reason}");
                    anyhow::bail!(reason)
                },
            };
            serialize_plain(&small_xsave)
        };

        Ok(FpuState { xsave_bytes: bytes })
    }
}
