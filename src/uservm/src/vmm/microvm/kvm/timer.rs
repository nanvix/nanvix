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
use ::serde::{
    Deserialize,
    Serialize,
};
use ::syslog::{
    error,
    trace,
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

        vm_fd.set_clock(&state.clock_data).map_err(|e| {
            let reason: String = format!("failed setting clock_data (error={e:?})");
            error!("restore_state(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        Ok(())
    }
}
