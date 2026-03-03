// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::kvm_bindings::{
    KVM_PIT_SPEAKER_DUMMY,
    kvm_pit_config,
};
use ::kvm_ioctls::{
    Kvm,
    VmFd,
};
use ::log::{
    error,
    trace,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use kvm_bindings::{
    kvm_clock_data,
    kvm_pit_state2,
};

//==================================================================================================
// Timer State
//==================================================================================================

#[derive(Serialize, Deserialize)]
pub struct TimerState {
    /// Timer.
    pit_state: kvm_pit_state2,
    /// Timestamp of kvmclock.
    clock_data: kvm_clock_data,
}

//==================================================================================================
// Timer
//==================================================================================================

pub struct Timer;

impl Timer {
    ///
    /// # Description
    ///
    /// Creates a programmable interrupt timer and attaches it to a virtual partition.
    ///
    /// # Parameters
    ///
    /// - `partition`: Handle to the virtual partition.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn new(kvm_fd: &mut Kvm, vm_fd: &mut VmFd) -> Result<Timer> {
        trace!("setup_pit()");

        let has_pit2_support: bool = kvm_fd.check_extension(kvm_ioctls::Cap::Pit2);
        if !has_pit2_support {
            let reason: &str = "pit2 is not supported";
            error!("new(): {reason}");
            anyhow::bail!(reason);
        }

        // Enable the emulation of a dummy speaker port stub so that writing to port 0x61
        // does not cause a KVM_EXIT event.
        let pit_config: kvm_pit_config = kvm_pit_config {
            flags: KVM_PIT_SPEAKER_DUMMY,
            ..Default::default()
        };

        vm_fd.create_pit2(pit_config)?;

        Ok(Self)
    }

    ///
    /// # Description
    ///
    /// Saves the state of the timer.
    ///
    /// # Parameters
    ///
    /// - `partition`: Handle to the virtual partition.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the state of the timer. Otherwise, it
    /// returns an error.
    ///
    pub fn save_state(&self, vm_fd: &VmFd) -> Result<TimerState> {
        trace!("save_state()");

        let pit_state: kvm_pit_state2 = match vm_fd.get_pit2() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting pit_state (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let clock_data: kvm_clock_data = match vm_fd.get_clock() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting clock_data (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };

        Ok(TimerState {
            pit_state,
            clock_data,
        })
    }

    ///
    /// # Description
    ///
    /// Restores the state of the timer.
    ///
    /// # Parameters
    ///
    /// - `partition`: Handle to the virtual partition.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn restore_state(&mut self, vm_fd: &VmFd, state: &TimerState) -> Result<()> {
        trace!("restore_state()");

        vm_fd.set_pit2(&state.pit_state).map_err(|e| {
            let reason: String = format!("failed setting pit_state (error={e:?})");
            error!("restore_state(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        // Clear flags before setting clock — some bits (e.g., KVM_CLOCK_TSC_STABLE) are
        // read-only and cause EINVAL if passed to set_clock.
        let mut clock_data: kvm_clock_data = state.clock_data;
        clock_data.flags = 0;
        vm_fd.set_clock(&clock_data).map_err(|e| {
            let reason: String = format!("failed setting clock_data (error={e:?})");
            error!("restore_state(): {reason}");
            anyhow::anyhow!(reason)
        })?;

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

    /// Creates a KVM VM with a PIT2 timer attached for testing.
    /// Returns the `Kvm` and `VmFd` handles alongside the `Timer` so that the KVM
    /// file descriptors remain open for the lifetime of the test.
    fn create_test_vm_with_timer() -> AnyResult<(Kvm, VmFd, Timer)> {
        let mut kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
        let mut vm: VmFd = kvm.create_vm().expect("failed to create VM");
        // IRQ chip is required for the PIT to function.
        vm.create_irq_chip().expect("failed to create IRQ chip");
        let timer: Timer = Timer::new(&mut kvm, &mut vm).expect("failed to create Timer");
        Ok((kvm, vm, timer))
    }

    /// Verifies that `save_state` produces a `TimerState` that is serializable.
    #[test]
    fn save_state_produces_serializable_snapshot() -> AnyResult<()> {
        let (_kvm, vm, timer): (Kvm, VmFd, Timer) = create_test_vm_with_timer()?;

        let state: TimerState = timer.save_state(&vm).expect("save_state failed");

        let encoded: Vec<u8> =
            ::serde_cbor::to_vec(&state).expect("TimerState should be serializable");
        assert!(!encoded.is_empty(), "serialized TimerState should not be empty");

        Ok(())
    }

    /// Verifies that a save → restore → save round trip succeeds without error.
    /// Note: PIT counters advance between saves, so we verify operability rather than byte
    /// equality.
    #[test]
    fn save_restore_round_trip() -> AnyResult<()> {
        let (_kvm, vm, mut timer): (Kvm, VmFd, Timer) = create_test_vm_with_timer()?;

        // Save the initial timer state.
        let state_before: TimerState = timer.save_state(&vm).expect("first save_state failed");

        // Restore it.
        timer
            .restore_state(&vm, &state_before)
            .expect("restore_state failed");

        // Save again — this must succeed, proving the timer is in a valid state after restore.
        let _state_after: TimerState = timer.save_state(&vm).expect("second save_state failed");

        Ok(())
    }

    /// Verifies that `restore_state` clears read-only clock flags so `set_clock` does not fail.
    #[test]
    fn restore_state_clears_clock_flags() -> AnyResult<()> {
        let (_kvm, vm, mut timer): (Kvm, VmFd, Timer) = create_test_vm_with_timer()?;

        let mut state: TimerState = timer.save_state(&vm).expect("save_state failed");

        // Artificially set a read-only flag that would cause EINVAL if passed through.
        state.clock_data.flags = 0xFFFF_FFFF;

        // restore_state should clear flags internally and succeed.
        timer
            .restore_state(&vm, &state)
            .expect("restore_state should succeed even with dirty clock flags");

        Ok(())
    }
}
