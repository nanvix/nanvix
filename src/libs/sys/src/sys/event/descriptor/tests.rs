// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use crate::event::{
    ExceptionEvent,
    InterruptEvent,
};

#[test]
fn from_ne_bytes_valid_event() {
    let desc: EventDescriptor =
        EventDescriptor::new(1, Event::Interrupt(InterruptEvent::Interrupt0));
    let bytes: [u8; core::mem::size_of::<usize>()] = desc.to_ne_bytes();
    let result: Result<EventDescriptor, Error> = EventDescriptor::from_ne_bytes(bytes);
    assert!(result.is_ok());
    let restored: EventDescriptor = result.expect("should be valid");
    assert_eq!(restored.id(), 1);
    assert_eq!(restored.event(), Event::Interrupt(InterruptEvent::Interrupt0));
}

#[test]
fn from_ne_bytes_invalid_event() {
    // Construct raw bytes with invalid event bits.
    let invalid_event_value: usize = EventDescriptor::EVENT_MASK;
    let bytes: [u8; core::mem::size_of::<usize>()] = invalid_event_value.to_ne_bytes();
    let result: Result<EventDescriptor, Error> = EventDescriptor::from_ne_bytes(bytes);
    assert!(result.is_err());
}

#[test]
fn try_from_usize_valid() {
    let desc: EventDescriptor =
        EventDescriptor::new(2, Event::Exception(ExceptionEvent::Exception0));
    let raw: usize = desc.into_raw();
    let result: Result<EventDescriptor, Error> = EventDescriptor::try_from(raw);
    assert!(result.is_ok());
}

#[test]
fn try_from_usize_invalid() {
    let invalid_raw: usize = EventDescriptor::EVENT_MASK;
    let result: Result<EventDescriptor, Error> = EventDescriptor::try_from(invalid_raw);
    assert!(result.is_err());
}
