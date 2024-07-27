// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Exceptions
//==================================================================================================

// Not all functions are used.
#![allow(dead_code)]

//==================================================================================================
// Constants
//==================================================================================================

// Constants for PIT (Programmable Interval Timer) configuration
pub const PIT_FREQUENCY: u32 = 1193182;

// Registers
pub const PIT_CTRL: u16 = 0x43; // Control
pub const PIT_DATA: u16 = 0x40; // Data

// Select Channels
pub const PIT_SEL0: u8 = 0x00; // Channel 0
pub const PIT_SEL1: u8 = 0x40; // Channel 1
pub const PIT_SEL2: u8 = 0x80; // Channel 2
pub const PIT_RB: u8 = 0xc0; // Read-back command

// Read-back Commands
pub const PIT_RB_CNTR0: u8 = 0x02; // Read-back channel 0
pub const PIT_RB_CNTR1: u8 = 0x04; // Read-back channel 1
pub const PIT_RB_CNTR2: u8 = 0x08; // Read-back channel 2
pub const PIT_RB_STAT: u8 = 0x10; // Don't latch status flag
pub const PIT_RB_COUNT: u8 = 0x20; // Don't latch count flag

// Status Byte
pub const PIT_STAT_OUT: u8 = 0x80; // OUT pin status
pub const PIT_STAT_NULL: u8 = 0x00; // NULL status

// Access Mode
pub const PIT_ACC_LATCH: u8 = 0x00; // Latch count value
pub const PIT_ACC_LO: u8 = 0x10; // Access mode: lobyte only
pub const PIT_ACC_HI: u8 = 0x20; // Access mode: hibyte only
pub const PIT_ACC_LOHI: u8 = 0x30; // Access mode: lobyte/hibyte

// Operating Mode
pub const PIT_MODE_TCOUNT: u8 = 0x00; // Mode 0: interrupt on terminal count
pub const PIT_MODE_HWSHOT: u8 = 0x02; // Mode 1: hardware re-triggerable one-shot
pub const PIT_MODE_RATE: u8 = 0x04; // Mode 2: rate generator
pub const PIT_MODE_WAVE: u8 = 0x06; // Mode 3: square wave generator
pub const PIT_MODE_SWSTROBE: u8 = 0x08; // Mode 4: software triggered strobe
pub const PIT_MODE_HWSTROBE: u8 = 0x0a; // Mode 5: hardware triggered strobe

// BCD/Binary Mode
pub const PIT_BINARY: u8 = 0x00; // Binary mode
pub const PIT_BCD: u8 = 0x01; // BCD mode
