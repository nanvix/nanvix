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
use ::log::{
    error,
    trace,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::mem;

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

impl FpuState {
    ///
    /// # Description
    ///
    /// Validates the FPU/XSAVE state from a snapshot against the current host's capabilities.
    ///
    /// # Parameters
    ///
    /// - `kvm`: Handle to the KVM hypervisor for querying host XSAVE support.
    ///
    /// # Return Value
    ///
    /// On success, this function returns empty. On failure, an error describing the incompatibility
    /// is returned.
    ///
    pub(crate) fn validate(&self, kvm: &Kvm) -> Result<()> {
        trace!("FpuState::validate()");

        if self.xsave_bytes.is_empty() {
            let reason: &str = "snapshot XSAVE state is empty";
            error!("validate(): {reason}");
            anyhow::bail!(reason);
        }

        let xsave2_size: usize = match kvm.check_extension_int(Cap::Xsave2) {
            size if size > 0 => size as usize,
            _ => 0,
        };

        if xsave2_size > 0 {
            // Host supports extended XSAVE — snapshot must have at least the kvm_xsave2 header.
            // Note: kvm_xsave2 wraps kvm_xsave with an additional `len: usize` field, so its
            // size_of is larger than the raw XSAVE area reported by Cap::Xsave2.
            let header_size: usize = mem::size_of::<kvm_bindings::kvm_xsave2>();
            if self.xsave_bytes.len() < header_size {
                let reason: String = format!(
                    "snapshot XSAVE data too short for xsave2 header: expected at least {}, got {}",
                    header_size,
                    self.xsave_bytes.len()
                );
                error!("validate(): {reason}");
                anyhow::bail!(reason);
            }
            // Compute the maximum valid serialized size.  Cap::Xsave2 returns the kernel XSAVE
            // buffer size; the serialized form additionally includes the kvm_xsave2 wrapper header.
            // FAM entries (u32) hold any extra bytes beyond the header, rounded up.
            // Note: if xsave2_size < header_size (unusual), saturating_sub yields 0, so
            // max_serialized_size == header_size and any extra data is correctly rejected.
            let fam_bytes: usize = xsave2_size.saturating_sub(header_size);
            let fam_units: usize = fam_bytes.div_ceil(mem::size_of::<u32>());
            let max_serialized_size: usize = header_size + fam_units * mem::size_of::<u32>();
            if self.xsave_bytes.len() > max_serialized_size {
                let reason: String = format!(
                    "snapshot XSAVE data ({} bytes) exceeds host XSAVE2 capacity ({} bytes)",
                    self.xsave_bytes.len(),
                    max_serialized_size
                );
                error!("validate(): {reason}");
                anyhow::bail!(reason);
            }
        } else {
            // Host only supports legacy kvm_xsave — size must match exactly.
            let expected: usize = mem::size_of::<kvm_xsave>();
            if self.xsave_bytes.len() != expected {
                let reason: String = format!(
                    "snapshot XSAVE data size mismatch: expected {}, got {}",
                    expected,
                    self.xsave_bytes.len()
                );
                error!("validate(): {reason}");
                anyhow::bail!(reason);
            }
        }

        Ok(())
    }
}

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

    ///
    /// # Description
    ///
    /// Restores the FPU state from a previously saved snapshot.
    ///
    /// # Parameters
    ///
    /// - `fd`: Reference to the VCPU device.
    /// - `state`: The FPU state to restore.
    ///
    /// # Return Value
    ///
    /// On success, this function returns empty. On failure, an error object that
    /// describes the error is returned instead.
    ///
    pub fn load_state(&self, fd: &VcpuFd, state: &FpuState) -> Result<()> {
        trace!("load_state()");

        if self.xsave2_size > 0 {
            // Reconstruct Xsave (FamStructWrapper<kvm_xsave2>) from serialized bytes.
            let header_size: usize = mem::size_of::<kvm_bindings::kvm_xsave2>();
            let entry_size: usize = mem::size_of::<u32>();
            if state.xsave_bytes.len() < header_size {
                let reason: &str = "xsave2 data too short for header";
                error!("load_state(): {reason}");
                anyhow::bail!(reason)
            }
            let payload_len: usize = state.xsave_bytes.len() - header_size;
            if !payload_len.is_multiple_of(entry_size) {
                let reason: String = format!(
                    "xsave2 payload size {} not divisible by entry size {}",
                    payload_len, entry_size
                );
                error!("load_state(): {reason}");
                anyhow::bail!(reason)
            }
            let num_entries: usize = payload_len / entry_size;
            let mut xsave2: Xsave = match Xsave::new(num_entries) {
                Ok(v) => v,
                Err(e) => {
                    let reason: String = format!("failed creating xsave2 (error={e:?})");
                    error!("load_state(): {reason}");
                    anyhow::bail!(reason)
                },
            };
            // SAFETY: We copy exactly the saved bytes into the freshly-allocated Xsave region.
            // The FamStructWrapper guarantees contiguous layout matching the serialized data.
            unsafe {
                let dst: *mut u8 = xsave2.as_mut_fam_struct_ptr().cast::<u8>();
                std::ptr::copy_nonoverlapping(
                    state.xsave_bytes.as_ptr(),
                    dst,
                    state.xsave_bytes.len(),
                );
            }
            // SAFETY: The Xsave buffer has been populated with valid data from the snapshot.
            if let Err(e) = unsafe { fd.set_xsave2(&xsave2) } {
                let reason: String = format!("failed setting xsave2 (error={e:?})");
                error!("load_state(): {reason}");
                anyhow::bail!(reason)
            }
        } else {
            // Reconstruct kvm_xsave from serialized bytes.
            if state.xsave_bytes.len() != mem::size_of::<kvm_xsave>() {
                let reason: String = format!(
                    "xsave data size mismatch: expected {}, got {}",
                    mem::size_of::<kvm_xsave>(),
                    state.xsave_bytes.len()
                );
                error!("load_state(): {reason}");
                anyhow::bail!(reason)
            }
            // SAFETY: kvm_xsave is repr(C), we've verified the byte length matches, and we use
            // read_unaligned because the Vec<u8> buffer does not guarantee kvm_xsave alignment.
            let xsave: kvm_xsave =
                unsafe { std::ptr::read_unaligned(state.xsave_bytes.as_ptr().cast::<kvm_xsave>()) };
            // SAFETY: The xsave buffer has been populated with valid data from the snapshot.
            if let Err(e) = unsafe { fd.set_xsave(&xsave) } {
                let reason: String = format!("failed setting xsave (error={e:?})");
                error!("load_state(): {reason}");
                anyhow::bail!(reason)
            }
        }

        Ok(())
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::anyhow::Result as AnyResult;
    use ::kvm_ioctls::VmFd;

    /// Creates a minimal KVM VM with one vCPU for testing.
    /// Returns the `Kvm` and `VmFd` handles alongside the `VcpuFd` so that the KVM
    /// file descriptors remain open for the lifetime of the test.
    fn create_test_vcpu() -> AnyResult<(Kvm, VmFd, VcpuFd)> {
        let kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
        let vm: VmFd = kvm.create_vm().expect("failed to create VM");
        let vcpu: VcpuFd = vm.create_vcpu(0).expect("failed to create vCPU");
        Ok((kvm, vm, vcpu))
    }

    /// Verifies that `save_state` followed by `load_state` followed by another `save_state`
    /// produces identical FPU state bytes (round-trip invariant).
    #[test]
    fn save_load_round_trip() -> AnyResult<()> {
        let (kvm, _vm, vcpu_fd): (Kvm, VmFd, VcpuFd) = create_test_vcpu()?;
        let fpu: Fpu = Fpu::new(&kvm, &vcpu_fd).expect("failed to create FPU");

        // Save the initial FPU state.
        let state_before: FpuState = fpu.save_state(&vcpu_fd).expect("first save_state failed");
        assert!(!state_before.xsave_bytes.is_empty(), "saved FPU state should not be empty");

        // Load the saved state back.
        fpu.load_state(&vcpu_fd, &state_before)
            .expect("load_state failed");

        // Save again and verify the bytes are identical.
        let state_after: FpuState = fpu.save_state(&vcpu_fd).expect("second save_state failed");
        assert_eq!(
            state_before.xsave_bytes, state_after.xsave_bytes,
            "FPU state should be identical after a save-load-save round trip"
        );

        Ok(())
    }

    /// Verifies that `load_state` rejects data that is too short for the xsave header.
    #[test]
    fn load_state_rejects_truncated_data() -> AnyResult<()> {
        let (kvm, _vm, vcpu_fd): (Kvm, VmFd, VcpuFd) = create_test_vcpu()?;
        let fpu: Fpu = Fpu::new(&kvm, &vcpu_fd).expect("failed to create FPU");

        let bad_state: FpuState = FpuState {
            xsave_bytes: vec![0u8; 4],
        };
        let result: AnyResult<()> = fpu.load_state(&vcpu_fd, &bad_state);
        assert!(result.is_err(), "load_state should reject truncated data");

        Ok(())
    }

    /// Verifies that `validate` accepts a well-formed FPU/XSAVE snapshot.
    #[test]
    fn validate_accepts_valid_snapshot() -> AnyResult<()> {
        let (kvm, _vm, vcpu_fd): (Kvm, VmFd, VcpuFd) = create_test_vcpu()?;
        let fpu: Fpu = Fpu::new(&kvm, &vcpu_fd).expect("failed to create FPU");

        let state: FpuState = fpu.save_state(&vcpu_fd).expect("save_state failed");
        state
            .validate(&kvm)
            .expect("validate should accept a valid FPU snapshot");

        Ok(())
    }

    /// Verifies that `validate` rejects an empty XSAVE state.
    #[test]
    fn validate_rejects_empty_xsave() {
        let kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
        let bad_state: FpuState = FpuState {
            xsave_bytes: vec![],
        };
        assert!(bad_state.validate(&kvm).is_err(), "validate should reject empty XSAVE data");
    }

    /// Verifies that `validate` rejects XSAVE data that is too short for the header.
    #[test]
    fn validate_rejects_truncated_xsave() {
        let kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
        let bad_state: FpuState = FpuState {
            xsave_bytes: vec![0u8; 4],
        };
        assert!(bad_state.validate(&kvm).is_err(), "validate should reject truncated XSAVE data");
    }
}
