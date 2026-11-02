// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::alloc::format;

#[test]
fn scheduling_event_discriminants_are_stable() {
    assert_eq!(u32::from(SchedulingEvent::ProcessTermination), 0);
    assert_eq!(u32::from(SchedulingEvent::ProcessCreation), 1);
    assert_eq!(u32::from(SchedulingEvent::ThreadTermination), 2);
}

#[test]
fn scheduling_events_round_trip_through_numeric_conversions() {
    for event in SchedulingEvent::VALUES {
        let raw_u32: u32 = u32::from(event);
        let raw_usize: usize = usize::from(event);

        assert_eq!(
            SchedulingEvent::try_from(raw_u32).expect("valid u32 scheduling event should parse"),
            event
        );
        assert_eq!(
            SchedulingEvent::try_from(raw_usize)
                .expect("valid usize scheduling event should parse"),
            event
        );
    }
}

#[test]
fn invalid_scheduling_event_discriminants_are_rejected() {
    assert!(SchedulingEvent::try_from(SchedulingEvent::NUMBER_EVENTS as u32).is_err());
    assert!(SchedulingEvent::try_from(SchedulingEvent::NUMBER_EVENTS).is_err());
    assert!(SchedulingEvent::try_from(u32::MAX).is_err());
    assert!(SchedulingEvent::try_from(usize::MAX).is_err());
}

#[test]
fn thread_termination_scheduling_event_is_formatted() {
    assert_eq!(format!("{:?}", SchedulingEvent::ThreadTermination), "ThreadTermination");
}
