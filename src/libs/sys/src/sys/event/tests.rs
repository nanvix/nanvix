// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn thread_termination_round_trips_through_event_conversions() {
    let event: Event = Event::Scheduling(SchedulingEvent::ThreadTermination);
    let raw_u32: u32 = u32::from(event);
    let raw_usize: usize = usize::from(event);

    assert_eq!(Event::try_from(raw_u32).expect("valid u32 event should parse"), event);
    assert_eq!(Event::try_from(raw_usize).expect("valid usize event should parse"), event);
}
