// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Exceptions
//==================================================================================================

// Not all functions are used.
#![allow(dead_code)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::cpu::pit,
    error::Error,
    hal::io::{
        IoPortAllocator,
        ReadWriteIoPort,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct Pit {
    ctrl: ReadWriteIoPort,
    data: ReadWriteIoPort,
}

impl Pit {
    pub fn new(ioports: &mut IoPortAllocator, freq: u32) -> Result<Self, Error> {
        let ctrl = ioports.allocate_read_write(pit::PIT_CTRL)?;
        let data = ioports.allocate_read_write(pit::PIT_DATA)?;

        let mut pit = Self { ctrl, data };

        pit.init(freq);

        Ok(pit)
    }

    pub fn init(&mut self, freq: u32) {
        info!("initializing pit...");
        let freq_divisor = pit::PIT_FREQUENCY / freq;

        self.ctrl
            .write8(pit::PIT_SEL0 | pit::PIT_ACC_LOHI | pit::PIT_MODE_WAVE | pit::PIT_BINARY);

        // Send data byte: divisor low and high bytes.
        self.data.write8((freq_divisor & 0xff) as u8);
        self.data.write8(((freq_divisor >> 8) & 0xff) as u8);
    }
}
