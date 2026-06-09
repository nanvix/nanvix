// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::ErrorCode,
    event::{
        Event,
        EventCtrlRequest,
        ExceptionEvent,
        SchedulingEvent,
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, true) {
        Ok(()) => (),
        _ => return false,
    }

    // Debug exception.
    let debug_exception: ExceptionEvent = ExceptionEvent::Exception1;

    // Attempt to subscribe to event.
    match ::sys::kcall::event::__kcall_evctrl(
        Event::Exception(debug_exception),
        EventCtrlRequest::Register,
    ) {
        Ok(()) => (),
        _ => return false,
    }

    // Attempt to unsubscribe from event.
    match ::sys::kcall::event::__kcall_evctrl(
        Event::Exception(debug_exception),
        EventCtrlRequest::Unregister,
    ) {
        Ok(()) => (),
        _ => return false,
    }

    // Release exception control capability.
    matches!(::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, false), Ok(()))
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
    // Debug exception.
    let debug_exception: ExceptionEvent = ExceptionEvent::Exception1;

    // Attempt to subscribe to event.
    !matches!(
        ::sys::kcall::event::__kcall_evctrl(
            Event::Exception(debug_exception),
            EventCtrlRequest::Register
        ),
        Ok(())
    )
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, true) {
        Ok(()) => (),
        _ => return false,
    }

    // Debug exception.
    let debug_exception: ExceptionEvent = ExceptionEvent::Exception1;

    // Attempt to unsubscribe from event.
    if let Ok(()) = ::sys::kcall::event::__kcall_evctrl(
        Event::Exception(debug_exception),
        EventCtrlRequest::Unregister,
    ) {
        return false;
    }

    // Release exception control capability.
    matches!(::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, false), Ok(()))
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, true) {
        Ok(()) => (),
        _ => return false,
    }

    // Debug exception.
    let debug_exception: ExceptionEvent = ExceptionEvent::Exception1;

    // Subscribe to event.
    match ::sys::kcall::event::__kcall_evctrl(
        Event::Exception(debug_exception),
        EventCtrlRequest::Register,
    ) {
        Ok(()) => (),
        _ => return false,
    }

    // Release exception control capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::ExceptionControl, false) {
        Ok(()) => (),
        _ => return false,
    }

    // Attempt to unsubscribe from event.
    matches!(
        ::sys::kcall::event::__kcall_evctrl(
            Event::Exception(debug_exception),
            EventCtrlRequest::Unregister
        ),
        Ok(())
    )
}

///
/// # Description
///
/// Attempts to subscribe to a scheduling event that is already owned by the process manager daemon.
///
/// In the standalone deployment, `procd` is always spawned and subscribes to
/// [`SchedulingEvent::ProcessTermination`] for its entire lifetime. Because the kernel owns
/// scheduling events as a single, class-wide ownership (one owner for all scheduling events), any
/// other process that holds the required capability must be rejected with
/// [`ErrorCode::ResourceBusy`] when it attempts to register for the same event.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_scheduling_event_single_owner() -> bool {
    // Acquire process management capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    // Attempt to subscribe to an event that procd already owns.
    let result: Result<(), ::sys::error::Error> = ::sys::kcall::event::__kcall_evctrl(
        Event::Scheduling(SchedulingEvent::ProcessTermination),
        EventCtrlRequest::Register,
    );

    // Release process management capability regardless of the outcome above.
    let released: bool =
        matches!(::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, false), Ok(()));

    // The registration must have been rejected because procd already owns the event.
    matches!(result, Err(e) if e.code == ErrorCode::ResourceBusy) && released
}

///
/// # Description
///
/// Attempts to subscribe to a scheduling event without owning the required capability.
///
/// Registering for a scheduling event requires [`Capability::ProcessManagement`]. The capability
/// check is performed before the ownership check, so the attempt must fail with
/// [`ErrorCode::PermissionDenied`] even though the event is already owned by `procd`.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_scheduling_event_requires_capability() -> bool {
    // Attempt to subscribe without holding the process management capability.
    matches!(
        ::sys::kcall::event::__kcall_evctrl(
            Event::Scheduling(SchedulingEvent::ProcessTermination),
            EventCtrlRequest::Register,
        ),
        Err(e) if e.code == ErrorCode::PermissionDenied
    )
}

///
/// # Description
///
/// Attempts to subscribe to a scheduling event other than the one `procd` explicitly registered
/// for at startup.
///
/// Scheduling events are owned as a single, indivisible class: a process either owns every
/// scheduling event or none of them. Because `procd` claims the whole class when it registers for
/// [`SchedulingEvent::ProcessTermination`], a capable process attempting to register for the
/// distinct [`SchedulingEvent::ProcessCreation`] must still be rejected with
/// [`ErrorCode::ResourceBusy`]. This guards against a split-brain ownership where two different
/// processes own different scheduling events.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_scheduling_event_class_is_exclusive() -> bool {
    // Acquire process management capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    // Attempt to subscribe to a *different* scheduling event than the one procd registered for.
    let result: Result<(), ::sys::error::Error> = ::sys::kcall::event::__kcall_evctrl(
        Event::Scheduling(SchedulingEvent::ProcessCreation),
        EventCtrlRequest::Register,
    );

    // Release process management capability regardless of the outcome above.
    let released: bool =
        matches!(::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, false), Ok(()));

    // The registration must have been rejected because procd owns the entire scheduling class.
    matches!(result, Err(e) if e.code == ErrorCode::ResourceBusy) && released
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
    crate::test!(test_scheduling_event_single_owner());
    crate::test!(test_scheduling_event_requires_capability());
    crate::test!(test_scheduling_event_class_is_exclusive());
}
