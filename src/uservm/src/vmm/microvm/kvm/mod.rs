// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Linux KVM Backend
//!
//! This module provides the backend implementation of MicroVM for Linux KVM.
//!

//==================================================================================================
// Exports
//==================================================================================================

pub mod irqchip;
pub mod pmio;
pub mod timer;
pub mod vcpu;
pub mod vmem;

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::{
    guest::GuestState,
    kvm::{
        irqchip::IrqChipState,
        timer::TimerState,
        vcpu::VirtualProcessorState,
    },
};
use ::serde::{
    Deserialize,
    Serialize,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that holds all KVM components for snapshot serialization and deserialization.
///
#[derive(Serialize, Deserialize)]
pub struct KvmSnapshot {
    /// Guest component.
    guest_state: GuestState,
    /// Virtual processor component.
    vcpu_state: VirtualProcessorState,
    /// IrqChip component.
    irqchip_state: IrqChipState,
    /// Timer component.
    timer_state: TimerState,
}

impl KvmSnapshot {
    ///
    /// # Description
    ///
    /// Consolidates multiple states into a single one for writing snapshots.
    ///
    /// # Parameters
    ///
    /// - `guest_state`: Guest related state.
    /// - `vcpu_state`: vCPU related state.
    /// - `irqchip_state`: Interrupt controller related state.
    /// - `timer_state`: Timer related state.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it
    /// returns an error.
    ///
    pub fn new(
        guest_state: GuestState,
        vcpu_state: VirtualProcessorState,
        irqchip_state: IrqChipState,
        timer_state: TimerState,
    ) -> Self {
        Self {
            guest_state,
            vcpu_state,
            irqchip_state,
            timer_state,
        }
    }

    ///
    /// # Description
    ///
    /// Gets a reference to the underlying guest state.
    ///
    /// # Returns
    ///
    /// A reference to the guest state.
    ///
    pub fn get_guest_state(&self) -> &GuestState {
        &self.guest_state
    }

    ///
    /// # Description
    ///
    /// Gets a reference to the underlying vCPU state.
    ///
    /// # Returns
    ///
    /// A reference to the vCPU state.
    ///
    pub fn get_vcpu_state(&self) -> &VirtualProcessorState {
        &self.vcpu_state
    }

    ///
    /// # Description
    ///
    /// Gets a reference to the underlying interrupt controller state.
    ///
    /// # Returns
    ///
    /// A reference to the interrupt controller state.
    ///
    pub fn get_irqchip_state(&self) -> &IrqChipState {
        &self.irqchip_state
    }

    ///
    /// # Description
    ///
    /// Gets a reference to the underlying timer state.
    ///
    /// # Returns
    ///
    /// A reference to the timer state.
    ///
    pub fn get_timer_state(&self) -> &TimerState {
        &self.timer_state
    }
}
