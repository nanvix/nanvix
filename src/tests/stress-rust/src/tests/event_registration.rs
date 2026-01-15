// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::StressError;
use ::sys::{
    event::{
        Event,
        EventCtrlRequest,
        ExceptionEvent,
    },
    kcall::{
        event::evctrl,
        pm::capctl,
        sched::sched_yield,
    },
    pm::Capability,
};

//==================================================================================================
// Structures
//==================================================================================================

struct CapabilityGuard {
    capability: Capability,
    released: bool,
}

impl CapabilityGuard {
    fn enable(capability: Capability) -> Result<Self, StressError> {
        capctl(capability, true)?;
        Ok(Self {
            capability,
            released: false,
        })
    }

    fn disable(&mut self) -> Result<(), StressError> {
        if !self.released {
            capctl(self.capability, false)?;
            self.released = true;
        }
        Ok(())
    }
}

impl Drop for CapabilityGuard {
    fn drop(&mut self) {
        if !self.released {
            let _ = capctl(self.capability, false);
            self.released = true;
        }
    }
}

//==================================================================================================
// Constants
//==================================================================================================

const EVENT_REGISTRATION_ROUNDS: usize = 6;
const EVENT_SAMPLE_SIZE: usize = 8;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Repeatedly registers and unregisters a subset of exception events, simulating crash handlers or
/// profilers that rapidly reconfigure event subscriptions.
///
/// # Returns
///
/// `Ok(())` on success or an error if event control calls fail.
///
pub fn run() -> Result<(), StressError> {
    let mut cap_guard: CapabilityGuard = CapabilityGuard::enable(Capability::ExceptionControl)?;

    for cycle in 0..EVENT_REGISTRATION_ROUNDS {
        for (index, ev) in ExceptionEvent::VALUES.iter().enumerate() {
            if index >= EVENT_SAMPLE_SIZE {
                break;
            }

            let target: Event = Event::Exception(*ev);
            evctrl(target, EventCtrlRequest::Register)?;
            evctrl(target, EventCtrlRequest::Unregister)?;

            if (cycle + index) & 0x3 == 0 {
                sched_yield()?;
            }
        }
    }

    cap_guard.disable()?;
    Ok(())
}
