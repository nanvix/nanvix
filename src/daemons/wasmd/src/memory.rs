// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use core::mem;

pub trait WriteBytes {
    fn write_le_bytes(&self, to: &mut [u8]);
}

impl WriteBytes for u8 {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[0] = *self;
    }
}

impl WriteBytes for u16 {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[..mem::size_of::<Self>()].copy_from_slice(&self.to_le_bytes());
    }
}

impl WriteBytes for u32 {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[..mem::size_of::<Self>()].copy_from_slice(&self.to_le_bytes());
    }
}

impl WriteBytes for u64 {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[..mem::size_of::<Self>()].copy_from_slice(&self.to_le_bytes());
    }
}

impl WriteBytes for usize {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[..mem::size_of::<Self>()].copy_from_slice(&self.to_le_bytes());
    }
}

impl WriteBytes for i8 {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[0] = *self as u8;
    }
}

impl WriteBytes for i16 {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[..mem::size_of::<Self>()].copy_from_slice(&self.to_le_bytes());
    }
}

impl WriteBytes for i32 {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[..mem::size_of::<Self>()].copy_from_slice(&self.to_le_bytes());
    }
}

impl WriteBytes for i64 {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[..mem::size_of::<Self>()].copy_from_slice(&self.to_le_bytes());
    }
}

impl WriteBytes for isize {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[..mem::size_of::<Self>()].copy_from_slice(&self.to_le_bytes());
    }
}

impl<T> WriteBytes for [T]
where
    T: WriteBytes,
{
    fn write_le_bytes(&self, to: &mut [u8]) {
        let mut offset = 0;
        for item in self {
            item.write_le_bytes(&mut to[offset..]);
            offset += mem::size_of::<T>();
        }
    }
}
