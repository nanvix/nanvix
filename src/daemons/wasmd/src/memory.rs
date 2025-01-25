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

pub struct ReadBytesError;

pub trait ReadBytes {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError>
    where
        Self: Sized;
}

impl ReadBytes for u8 {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        if from.len() < mem::size_of::<u8>() {
            return Err(ReadBytesError);
        }
        Ok(from[0])
    }
}

impl ReadBytes for u16 {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        if from.len() < mem::size_of::<u16>() {
            return Err(ReadBytesError);
        }
        let mut bytes: [u8; mem::size_of::<u16>()] = [0; mem::size_of::<u16>()];
        bytes.copy_from_slice(&from[..mem::size_of::<u16>()]);
        Ok(Self::from_le_bytes(bytes))
    }
}

impl ReadBytes for u32 {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        if from.len() < mem::size_of::<u32>() {
            return Err(ReadBytesError);
        }
        let mut bytes: [u8; mem::size_of::<u32>()] = [0; mem::size_of::<u32>()];
        bytes.copy_from_slice(&from[..mem::size_of::<u32>()]);
        Ok(Self::from_le_bytes(bytes))
    }
}

impl ReadBytes for u64 {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        if from.len() < mem::size_of::<u64>() {
            return Err(ReadBytesError);
        }
        let mut bytes: [u8; mem::size_of::<u64>()] = [0; mem::size_of::<u64>()];
        bytes.copy_from_slice(&from[..mem::size_of::<u64>()]);
        Ok(Self::from_le_bytes(bytes))
    }
}

impl ReadBytes for usize {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        if from.len() < mem::size_of::<usize>() {
            return Err(ReadBytesError);
        }
        let mut bytes: [u8; mem::size_of::<usize>()] = [0; mem::size_of::<usize>()];
        bytes.copy_from_slice(&from[..mem::size_of::<usize>()]);
        Ok(Self::from_le_bytes(bytes))
    }
}

impl ReadBytes for i8 {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        if from.len() < mem::size_of::<i8>() {
            return Err(ReadBytesError);
        }
        Ok(from[0] as i8)
    }
}

impl ReadBytes for i16 {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        if from.len() < mem::size_of::<i16>() {
            return Err(ReadBytesError);
        }
        let mut bytes: [u8; mem::size_of::<i16>()] = [0; mem::size_of::<i16>()];
        bytes.copy_from_slice(&from[..mem::size_of::<i16>()]);
        Ok(Self::from_le_bytes(bytes))
    }
}

impl ReadBytes for i32 {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        if from.len() < mem::size_of::<i32>() {
            return Err(ReadBytesError);
        }
        let mut bytes: [u8; mem::size_of::<i32>()] = [0; mem::size_of::<i32>()];
        bytes.copy_from_slice(&from[..mem::size_of::<i32>()]);
        Ok(Self::from_le_bytes(bytes))
    }
}

impl ReadBytes for i64 {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        if from.len() < mem::size_of::<i64>() {
            return Err(ReadBytesError);
        }
        let mut bytes: [u8; mem::size_of::<i64>()] = [0; mem::size_of::<i64>()];
        bytes.copy_from_slice(&from[..mem::size_of::<i64>()]);
        Ok(Self::from_le_bytes(bytes))
    }
}

impl ReadBytes for isize {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        if from.len() < mem::size_of::<isize>() {
            return Err(ReadBytesError);
        }
        let mut bytes: [u8; mem::size_of::<isize>()] = [0; mem::size_of::<isize>()];
        bytes.copy_from_slice(&from[..mem::size_of::<isize>()]);
        Ok(Self::from_le_bytes(bytes))
    }
}
