// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    Error,
    ErrorCode,
    KcallNumber,
    ProcessIdentifier,
};
use ::core::fmt::Debug;

//==================================================================================================
// Interrupt Event
//==================================================================================================

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InterruptEvent {
    Interrupt0,
    Interrupt1,
    Interrupt2,
    Interrupt3,
    Interrupt4,
    Interrupt5,
    Interrupt6,
    Interrupt7,
    Interrupt8,
    Interrupt9,
    Interrupt10,
    Interrupt11,
    Interrupt12,
    Interrupt13,
    Interrupt14,
    Interrupt15,
    Interrupt16,
    Interrupt17,
    Interrupt18,
    Interrupt19,
    Interrupt20,
    Interrupt21,
    Interrupt22,
    Interrupt23,
    Interrupt24,
    Interrupt25,
    Interrupt26,
    Interrupt27,
    Interrupt28,
    Interrupt29,
    Interrupt30,
    Interrupt31,
}

impl InterruptEvent {
    pub const VALUES: [Self; 32] = [
        Self::Interrupt0,
        Self::Interrupt1,
        Self::Interrupt2,
        Self::Interrupt3,
        Self::Interrupt4,
        Self::Interrupt5,
        Self::Interrupt6,
        Self::Interrupt7,
        Self::Interrupt8,
        Self::Interrupt9,
        Self::Interrupt10,
        Self::Interrupt11,
        Self::Interrupt12,
        Self::Interrupt13,
        Self::Interrupt14,
        Self::Interrupt15,
        Self::Interrupt16,
        Self::Interrupt17,
        Self::Interrupt18,
        Self::Interrupt19,
        Self::Interrupt20,
        Self::Interrupt21,
        Self::Interrupt22,
        Self::Interrupt23,
        Self::Interrupt24,
        Self::Interrupt25,
        Self::Interrupt26,
        Self::Interrupt27,
        Self::Interrupt28,
        Self::Interrupt29,
        Self::Interrupt30,
        Self::Interrupt31,
    ];
}

impl From<InterruptEvent> for u32 {
    fn from(eventid: InterruptEvent) -> u32 {
        eventid as u32
    }
}

impl TryFrom<u32> for InterruptEvent {
    type Error = Error;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::Interrupt0),
            1 => Ok(Self::Interrupt1),
            2 => Ok(Self::Interrupt2),
            3 => Ok(Self::Interrupt3),
            4 => Ok(Self::Interrupt4),
            5 => Ok(Self::Interrupt5),
            6 => Ok(Self::Interrupt6),
            7 => Ok(Self::Interrupt7),
            8 => Ok(Self::Interrupt8),
            9 => Ok(Self::Interrupt9),
            10 => Ok(Self::Interrupt10),
            11 => Ok(Self::Interrupt11),
            12 => Ok(Self::Interrupt12),
            13 => Ok(Self::Interrupt13),
            14 => Ok(Self::Interrupt14),
            15 => Ok(Self::Interrupt15),
            16 => Ok(Self::Interrupt16),
            17 => Ok(Self::Interrupt17),
            18 => Ok(Self::Interrupt18),
            19 => Ok(Self::Interrupt19),
            20 => Ok(Self::Interrupt20),
            21 => Ok(Self::Interrupt21),
            22 => Ok(Self::Interrupt22),
            23 => Ok(Self::Interrupt23),
            24 => Ok(Self::Interrupt24),
            25 => Ok(Self::Interrupt25),
            26 => Ok(Self::Interrupt26),
            27 => Ok(Self::Interrupt27),
            28 => Ok(Self::Interrupt28),
            29 => Ok(Self::Interrupt29),
            30 => Ok(Self::Interrupt30),
            31 => Ok(Self::Interrupt31),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid interrupt event identifier")),
        }
    }
}

impl From<InterruptEvent> for usize {
    fn from(eventid: InterruptEvent) -> usize {
        eventid as usize
    }
}

impl TryFrom<usize> for InterruptEvent {
    type Error = Error;

    fn try_from(raw: usize) -> Result<Self, Self::Error> {
        Self::try_from(raw as u32)
    }
}

//==================================================================================================
// Exception Event
//==================================================================================================

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ExceptionEvent {
    Exception0,
    Exception1,
    Exception2,
    Exception3,
    Exception4,
    Exception5,
    Exception6,
    Exception7,
    Exception8,
    Exception9,
    Exception10,
    Exception11,
    Exception12,
    Exception13,
    Exception14,
    Exception15,
    Exception16,
    Exception17,
    Exception18,
    Exception19,
    Exception20,
    Exception21,
    Exception22,
    Exception23,
    Exception24,
    Exception25,
    Exception26,
    Exception27,
    Exception28,
    Exception29,
    Exception30,
    Exception31,
}

impl ExceptionEvent {
    pub const VALUES: [Self; 32] = [
        Self::Exception0,
        Self::Exception1,
        Self::Exception2,
        Self::Exception3,
        Self::Exception4,
        Self::Exception5,
        Self::Exception6,
        Self::Exception7,
        Self::Exception8,
        Self::Exception9,
        Self::Exception10,
        Self::Exception11,
        Self::Exception12,
        Self::Exception13,
        Self::Exception14,
        Self::Exception15,
        Self::Exception16,
        Self::Exception17,
        Self::Exception18,
        Self::Exception19,
        Self::Exception20,
        Self::Exception21,
        Self::Exception22,
        Self::Exception23,
        Self::Exception24,
        Self::Exception25,
        Self::Exception26,
        Self::Exception27,
        Self::Exception28,
        Self::Exception29,
        Self::Exception30,
        Self::Exception31,
    ];
}

impl From<ExceptionEvent> for u32 {
    fn from(eventid: ExceptionEvent) -> u32 {
        eventid as u32
    }
}

impl TryFrom<u32> for ExceptionEvent {
    type Error = Error;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::Exception0),
            1 => Ok(Self::Exception1),
            2 => Ok(Self::Exception2),
            3 => Ok(Self::Exception3),
            4 => Ok(Self::Exception4),
            5 => Ok(Self::Exception5),
            6 => Ok(Self::Exception6),
            7 => Ok(Self::Exception7),
            8 => Ok(Self::Exception8),
            9 => Ok(Self::Exception9),
            10 => Ok(Self::Exception10),
            11 => Ok(Self::Exception11),
            12 => Ok(Self::Exception12),
            13 => Ok(Self::Exception13),
            14 => Ok(Self::Exception14),
            15 => Ok(Self::Exception15),
            16 => Ok(Self::Exception16),
            17 => Ok(Self::Exception17),
            18 => Ok(Self::Exception18),
            19 => Ok(Self::Exception19),
            20 => Ok(Self::Exception20),
            21 => Ok(Self::Exception21),
            22 => Ok(Self::Exception22),
            23 => Ok(Self::Exception23),
            24 => Ok(Self::Exception24),
            25 => Ok(Self::Exception25),
            26 => Ok(Self::Exception26),
            27 => Ok(Self::Exception27),
            28 => Ok(Self::Exception28),
            29 => Ok(Self::Exception29),
            30 => Ok(Self::Exception30),
            31 => Ok(Self::Exception31),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid exception event identifier")),
        }
    }
}

impl From<ExceptionEvent> for usize {
    fn from(eventid: ExceptionEvent) -> usize {
        eventid as usize
    }
}

impl TryFrom<usize> for ExceptionEvent {
    type Error = Error;

    fn try_from(raw: usize) -> Result<Self, Self::Error> {
        Self::try_from(raw as u32)
    }
}

//==================================================================================================
// Event
//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Interrupt(InterruptEvent),
    Exception(ExceptionEvent),
}

impl Event {
    const BIT_LENGTH: usize = 6;

    pub fn is_interrupt(&self) -> bool {
        match self {
            Event::Interrupt(_) => true,
            _ => false,
        }
    }

    pub fn is_exception(&self) -> bool {
        match self {
            Event::Exception(_) => true,
            _ => false,
        }
    }
}

impl From<Event> for u32 {
    fn from(ev: Event) -> u32 {
        match ev {
            Event::Interrupt(ev) => u32::from(ev),
            Event::Exception(ev) => 32 + (u32::from(ev)),
        }
    }
}

impl TryFrom<u32> for Event {
    type Error = Error;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        if raw > 64 {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid event"));
        } else if raw >= 32 {
            Ok(Event::Exception(ExceptionEvent::try_from(raw - 32)?))
        } else {
            Ok(Event::Interrupt(InterruptEvent::try_from(raw)?))
        }
    }
}

impl From<Event> for usize {
    fn from(ev: Event) -> usize {
        u32::from(ev) as usize
    }
}

impl TryFrom<usize> for Event {
    type Error = Error;

    fn try_from(raw: usize) -> Result<Self, Self::Error> {
        Event::try_from(raw as u32)
    }
}

impl From<InterruptEvent> for Event {
    fn from(ev: InterruptEvent) -> Event {
        Event::Interrupt(ev)
    }
}

impl From<ExceptionEvent> for Event {
    fn from(ev: ExceptionEvent) -> Event {
        Event::Exception(ev)
    }
}

//==================================================================================================
// Event Descriptor
//==================================================================================================

#[derive(Default, Clone, Eq, PartialEq)]
pub struct EventDescriptor(usize);

impl EventDescriptor {
    const EVENT_MASKLEN: usize = Event::BIT_LENGTH;
    const EVENT_MASK: usize = (1 << Self::EVENT_MASKLEN) - 1;
    const EVENT_SHIFT: usize = 0;
    const ID_MASK: usize = !(1 << (usize::BITS - 1) | Self::EVENT_MASK);
    const ID_SHIFT: usize = Self::EVENT_SHIFT + Self::EVENT_MASKLEN;

    pub fn into_raw(&self) -> usize {
        self.0
    }

    pub fn new(id: usize, ev: Event) -> Self {
        let id: usize = (id << Self::ID_SHIFT) & Self::ID_MASK;
        let ev: usize = usize::from(ev) & Self::EVENT_MASK;
        Self(id | ev)
    }

    pub fn id(&self) -> usize {
        (self.0 & Self::ID_MASK) >> Self::ID_SHIFT
    }

    pub fn event(&self) -> Event {
        Event::try_from(self.0 & Self::EVENT_MASK).unwrap()
    }

    pub fn is_interrupt(&self) -> bool {
        self.event().is_interrupt()
    }

    pub fn is_exception(&self) -> bool {
        self.event().is_exception()
    }
}

impl Debug for EventDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "event_information (id={:?}, type={:?})", self.id(), self.event())
    }
}

impl From<EventDescriptor> for usize {
    fn from(eventid: EventDescriptor) -> usize {
        eventid.0
    }
}

impl TryFrom<usize> for EventDescriptor {
    type Error = Error;

    fn try_from(raw: usize) -> Result<Self, Self::Error> {
        let id: usize = (raw & EventDescriptor::ID_MASK) >> EventDescriptor::ID_SHIFT;
        let ev: Event = Event::try_from(raw & EventDescriptor::EVENT_MASK)?;
        Ok(Self::new(id, ev))
    }
}

//==================================================================================================
// Event Information
//==================================================================================================

#[derive(Default, Debug)]
pub struct EventInformation {
    pub id: EventDescriptor,
    pub pid: ProcessIdentifier,
    pub number: Option<usize>,
    pub code: Option<usize>,
    pub address: Option<usize>,
    pub instruction: Option<usize>,
}

//==================================================================================================
// Wait Event
//==================================================================================================

pub fn wait(
    info: &mut EventInformation,
    interrupts: usize,
    exceptions: usize,
) -> Result<(), Error> {
    let result: i32 = unsafe {
        crate::kcall3(
            KcallNumber::Wait.into(),
            info as *mut EventInformation as u32,
            interrupts as u32,
            exceptions as u32,
        )
    };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to wait()"))
    }
}

//==================================================================================================
// Resume Event
//==================================================================================================

pub fn resume(event: EventDescriptor) -> Result<(), Error> {
    let result: i32 =
        unsafe { crate::kcall1(KcallNumber::Resume.into(), usize::from(event) as u32) };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to resume()"))
    }
}

//==================================================================================================
// Event Control Request
//==================================================================================================

#[derive(Debug, Clone, Copy)]
pub enum EventCtrlRequest {
    Register,
    Unregister,
}

impl From<EventCtrlRequest> for u32 {
    fn from(req: EventCtrlRequest) -> u32 {
        match req {
            EventCtrlRequest::Register => 0,
            EventCtrlRequest::Unregister => 1,
        }
    }
}

impl TryFrom<u32> for EventCtrlRequest {
    type Error = Error;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::Register),
            1 => Ok(Self::Unregister),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid event control request")),
        }
    }
}

//==================================================================================================
// Controls an Event
//==================================================================================================

pub fn evctrl(ev: Event, req: EventCtrlRequest) -> Result<(), Error> {
    let result: i32 =
        unsafe { crate::kcall2(KcallNumber::EventCtrl.into(), u32::from(ev), u32::from(req)) };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to evctrl()"))
    }
}
