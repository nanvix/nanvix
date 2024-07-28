// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use nvx::{
    event::{
        Event,
        EventCtrlRequest,
        ExceptionEvent,
    },
    pm::Capability,
};

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Attempts to subscribe and then unsubscribe to an event.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_subscribe_unsubscribe() -> bool {
    // Acquire exception control capability.
    match nvx::pm::capctl(Capability::ExceptionControl, true) {
        Ok(()) => (),
        _ => return false,
    }

    // Page fault exception.
    let page_fault_exception: ExceptionEvent = ExceptionEvent::Exception14;

    // Attempt to subscribe to event.
    match ::nvx::event::evctrl(Event::Exception(page_fault_exception), EventCtrlRequest::Register) {
        Ok(()) => (),
        _ => return false,
    }

    // Attempt to unsubscribe from event.
    match ::nvx::event::evctrl(Event::Exception(page_fault_exception), EventCtrlRequest::Unregister)
    {
        Ok(()) => (),
        _ => return false,
    }

    // Release exception control capability.
    match nvx::pm::capctl(Capability::ExceptionControl, false) {
        Ok(()) => true,
        _ => false,
    }
}

///
/// # Description
///
/// Attempts to subscribe to an event without owning the required capability.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_subscribe_without_capability() -> bool {
    // Page fault exception.
    let page_fault_exception: ExceptionEvent = ExceptionEvent::Exception14;

    // Attempt to subscribe to event.
    match ::nvx::event::evctrl(Event::Exception(page_fault_exception), EventCtrlRequest::Register) {
        Ok(()) => false,
        _ => true,
    }
}

///
/// # Description
///
/// Attempts to unsubscribe from an event without having subscribed to it.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_unsubscribe_without_subscription() -> bool {
    // Acquire exception control capability.
    match nvx::pm::capctl(Capability::ExceptionControl, true) {
        Ok(()) => (),
        _ => return false,
    }

    // Page fault exception.
    let page_fault_exception: ExceptionEvent = ExceptionEvent::Exception14;

    // Attempt to unsubscribe from event.
    match ::nvx::event::evctrl(Event::Exception(page_fault_exception), EventCtrlRequest::Unregister)
    {
        Ok(()) => return false,
        _ => (),
    }

    // Release exception control capability.
    match nvx::pm::capctl(Capability::ExceptionControl, false) {
        Ok(()) => true,
        _ => false,
    }
}

///
/// # Description
///
/// Attempts to unsubscribe from an event without owning the required capability.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_unsubscribe_without_capability() -> bool {
    // Acquire exception control capability.
    match nvx::pm::capctl(Capability::ExceptionControl, true) {
        Ok(()) => (),
        _ => return false,
    }

    // Page fault exception.
    let page_fault_exception: ExceptionEvent = ExceptionEvent::Exception14;

    // Subscribe to event.
    match ::nvx::event::evctrl(Event::Exception(page_fault_exception), EventCtrlRequest::Register) {
        Ok(()) => (),
        _ => return false,
    }

    // Release exception control capability.
    match nvx::pm::capctl(Capability::ExceptionControl, false) {
        Ok(()) => (),
        _ => return false,
    }

    // Attempt to unsubscribe from event.
    match ::nvx::event::evctrl(Event::Exception(page_fault_exception), EventCtrlRequest::Unregister)
    {
        Ok(()) => true,
        _ => false,
    }
}

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests kernel calls for event management.
///
pub fn test() {
    crate::test!(test_subscribe_unsubscribe());
    crate::test!(test_subscribe_without_capability());
    crate::test!(test_unsubscribe_without_subscription());
    crate::test!(test_unsubscribe_without_capability());
}
