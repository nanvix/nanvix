// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::types::{
        blkcnt_t,
        blksize_t,
        dev_t,
        gid_t,
        ino_t,
        mode_t,
        nlink_t,
        off_t,
        uid_t,
    },
    time::timespec,
};
use ::core::mem;
use ::nvx::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

/// File status structure.
#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct stat {
    /// Device ID.
    pub st_dev: dev_t,
    /// File serial number.
    pub st_ino: ino_t,
    /// File mode.
    pub st_mode: mode_t,
    /// Link count.
    pub st_nlink: nlink_t,
    /// User ID.
    pub st_uid: uid_t,
    /// Group ID.
    pub st_gid: gid_t,
    /// Device ID (if special file).
    pub st_rdev: dev_t,
    /// File size.
    pub st_size: off_t,
    /// Last access time.
    pub st_atim: timespec,
    /// Last modification time.,
    pub st_mtim: timespec,
    /// Last status change time.
    pub st_ctim: timespec,
    /// Block size.
    pub st_blksize: blksize_t,
    /// Number of blocks allocated.
    pub st_blocks: blkcnt_t,
}
::nvx::sys::static_assert_size!(stat, stat::SIZE);

impl stat {
    /// Size of the device ID field.
    const SIZE_OF_ST_DEV: usize = mem::size_of::<dev_t>();
    /// Size of the file serial number field.
    const SIZE_OF_ST_INO: usize = mem::size_of::<ino_t>();
    /// Size of the file mode field.
    const SIZE_OF_ST_MODE: usize = mem::size_of::<mode_t>();
    /// Size of the link count field.
    const SIZE_OF_ST_NLINK: usize = mem::size_of::<nlink_t>();
    /// Size of the user ID field.
    const SIZE_OF_ST_UID: usize = mem::size_of::<uid_t>();
    /// Size of the group ID field.
    const SIZE_OF_ST_GID: usize = mem::size_of::<gid_t>();
    /// Size of the device ID field (if special file).
    const SIZE_OF_ST_RDEV: usize = mem::size_of::<dev_t>();
    /// Size of the file size field.
    const SIZE_OF_ST_SIZE: usize = mem::size_of::<off_t>();
    /// Size of the last access time field.
    const SIZE_OF_ST_ATIM: usize = mem::size_of::<timespec>();
    /// Size of the last modification time field.
    const SIZE_OF_ST_MTIM: usize = mem::size_of::<timespec>();
    /// Size of the last status change time field.
    const SIZE_OF_ST_CTIM: usize = mem::size_of::<timespec>();
    /// Size of the block size field.
    const SIZE_OF_ST_BLKSIZE: usize = mem::size_of::<blksize_t>();
    /// Size of the number of blocks allocated field.
    const SIZE_OF_ST_BLOCKS: usize = mem::size_of::<blkcnt_t>();
    /// Offset of the device ID field.
    const OFFSET_OF_ST_DEV: usize = 0;
    /// Offset of the file serial number field.
    const OFFSET_OF_ST_INO: usize = Self::OFFSET_OF_ST_DEV + Self::SIZE_OF_ST_DEV;
    /// Offset of the file mode field.
    const OFFSET_OF_ST_MODE: usize = Self::OFFSET_OF_ST_INO + Self::SIZE_OF_ST_INO;
    /// Offset of the link count field.
    const OFFSET_OF_ST_NLINK: usize = Self::OFFSET_OF_ST_MODE + Self::SIZE_OF_ST_MODE;
    /// Offset of the user ID field.
    const OFFSET_OF_ST_UID: usize = Self::OFFSET_OF_ST_NLINK + Self::SIZE_OF_ST_NLINK;
    /// Offset of the group ID field.
    const OFFSET_OF_ST_GID: usize = Self::OFFSET_OF_ST_UID + Self::SIZE_OF_ST_UID;
    /// Offset of the device ID field (if special file).
    const OFFSET_OF_ST_RDEV: usize = Self::OFFSET_OF_ST_GID + Self::SIZE_OF_ST_GID;
    /// Offset of the file size field.
    const OFFSET_OF_ST_SIZE: usize = Self::OFFSET_OF_ST_RDEV + Self::SIZE_OF_ST_RDEV;
    /// Offset of the last access time field.
    const OFFSET_OF_ST_ATIM: usize = Self::OFFSET_OF_ST_SIZE + Self::SIZE_OF_ST_SIZE;
    /// Offset of the last modification time field.
    const OFFSET_OF_ST_MTIM: usize = Self::OFFSET_OF_ST_ATIM + Self::SIZE_OF_ST_ATIM;
    /// Offset of the last status change time field.
    const OFFSET_OF_ST_CTIM: usize = Self::OFFSET_OF_ST_MTIM + Self::SIZE_OF_ST_MTIM;
    /// Offset of the block size field.
    const OFFSET_OF_ST_BLKSIZE: usize = Self::OFFSET_OF_ST_CTIM + Self::SIZE_OF_ST_CTIM;
    /// Offset of the number of blocks allocated field.
    const OFFSET_OF_ST_BLOCKS: usize = Self::OFFSET_OF_ST_BLKSIZE + Self::SIZE_OF_ST_BLKSIZE;

    /// Size of the structure.
    const SIZE: usize = Self::SIZE_OF_ST_DEV
        + Self::SIZE_OF_ST_INO
        + Self::SIZE_OF_ST_MODE
        + Self::SIZE_OF_ST_NLINK
        + Self::SIZE_OF_ST_UID
        + Self::SIZE_OF_ST_GID
        + Self::SIZE_OF_ST_RDEV
        + Self::SIZE_OF_ST_SIZE
        + Self::SIZE_OF_ST_ATIM
        + Self::SIZE_OF_ST_MTIM
        + Self::SIZE_OF_ST_CTIM
        + Self::SIZE_OF_ST_BLKSIZE
        + Self::SIZE_OF_ST_BLOCKS;

    /// Converts a file status structure to a byte array.
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes: [u8; Self::SIZE] = [0; Self::SIZE];

        // Convert device ID field.
        bytes[Self::OFFSET_OF_ST_DEV..Self::OFFSET_OF_ST_DEV + Self::SIZE_OF_ST_DEV]
            .copy_from_slice(&self.st_dev.to_ne_bytes());

        // Convert file serial number field.
        bytes[Self::OFFSET_OF_ST_INO..Self::OFFSET_OF_ST_INO + Self::SIZE_OF_ST_INO]
            .copy_from_slice(&self.st_ino.to_ne_bytes());

        // Convert file mode field.
        bytes[Self::OFFSET_OF_ST_MODE..Self::OFFSET_OF_ST_MODE + Self::SIZE_OF_ST_MODE]
            .copy_from_slice(&self.st_mode.to_ne_bytes());

        // Convert link count field.
        bytes[Self::OFFSET_OF_ST_NLINK..Self::OFFSET_OF_ST_NLINK + Self::SIZE_OF_ST_NLINK]
            .copy_from_slice(&self.st_nlink.to_ne_bytes());

        // Convert user ID field.
        bytes[Self::OFFSET_OF_ST_UID..Self::OFFSET_OF_ST_UID + Self::SIZE_OF_ST_UID]
            .copy_from_slice(&self.st_uid.to_ne_bytes());

        // Convert group ID field.
        bytes[Self::OFFSET_OF_ST_GID..Self::OFFSET_OF_ST_GID + Self::SIZE_OF_ST_GID]
            .copy_from_slice(&self.st_gid.to_ne_bytes());

        // Convert device ID field (if special file).
        bytes[Self::OFFSET_OF_ST_RDEV..Self::OFFSET_OF_ST_RDEV + Self::SIZE_OF_ST_RDEV]
            .copy_from_slice(&self.st_rdev.to_ne_bytes());

        // Convert file size field.
        bytes[Self::OFFSET_OF_ST_SIZE..Self::OFFSET_OF_ST_SIZE + Self::SIZE_OF_ST_SIZE]
            .copy_from_slice(&self.st_size.to_ne_bytes());

        // Convert last access time field.
        bytes[Self::OFFSET_OF_ST_ATIM..Self::OFFSET_OF_ST_ATIM + Self::SIZE_OF_ST_ATIM]
            .copy_from_slice(&self.st_atim.to_bytes());

        // Convert last modification time field.
        bytes[Self::OFFSET_OF_ST_MTIM..Self::OFFSET_OF_ST_MTIM + Self::SIZE_OF_ST_MTIM]
            .copy_from_slice(&self.st_mtim.to_bytes());

        // Convert last status change time field.
        bytes[Self::OFFSET_OF_ST_CTIM..Self::OFFSET_OF_ST_CTIM + Self::SIZE_OF_ST_CTIM]
            .copy_from_slice(&self.st_ctim.to_bytes());

        // Convert block size field.
        bytes[Self::OFFSET_OF_ST_BLKSIZE..Self::OFFSET_OF_ST_BLKSIZE + Self::SIZE_OF_ST_BLKSIZE]
            .copy_from_slice(&self.st_blksize.to_ne_bytes());

        // Convert number of blocks allocated field.
        bytes[Self::OFFSET_OF_ST_BLOCKS..Self::OFFSET_OF_ST_BLOCKS + Self::SIZE_OF_ST_BLOCKS]
            .copy_from_slice(&self.st_blocks.to_ne_bytes());

        bytes
    }

    /// Tries to convert a file status structure from a byte array.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if the array has the correct size.
        if bytes.len() != Self::SIZE {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid array size"));
        }

        // Parse device ID field.
        let st_dev: dev_t = dev_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_ST_DEV..Self::OFFSET_OF_ST_DEV + Self::SIZE_OF_ST_DEV]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse st_dev"))?,
        );

        // Parse file serial number field.
        let st_ino: ino_t = ino_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_ST_INO..Self::OFFSET_OF_ST_INO + Self::SIZE_OF_ST_INO]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse st_ino"))?,
        );

        // Parse file mode field.
        let st_mode: mode_t = mode_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_ST_MODE..Self::OFFSET_OF_ST_MODE + Self::SIZE_OF_ST_MODE]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse st_mode"))?,
        );

        // Parse link count field.
        let st_nlink: nlink_t = nlink_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_ST_NLINK..Self::OFFSET_OF_ST_NLINK + Self::SIZE_OF_ST_NLINK]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse st_nlink"))?,
        );

        // Parse user ID field.
        let st_uid: uid_t = uid_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_ST_UID..Self::OFFSET_OF_ST_UID + Self::SIZE_OF_ST_UID]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse st_uid"))?,
        );

        // Parse group ID field.
        let st_gid: gid_t = gid_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_ST_GID..Self::OFFSET_OF_ST_GID + Self::SIZE_OF_ST_GID]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse st_gid"))?,
        );

        // Parse device ID field (if special file).
        let st_rdev: dev_t = dev_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_ST_RDEV..Self::OFFSET_OF_ST_RDEV + Self::SIZE_OF_ST_RDEV]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse st_rdev"))?,
        );

        // Parse file size field.
        let st_size: off_t = off_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_ST_SIZE..Self::OFFSET_OF_ST_SIZE + Self::SIZE_OF_ST_SIZE]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse st_size"))?,
        );

        // Parse last access time field.
        let st_atim: timespec = timespec::try_from_bytes(
            &bytes[Self::OFFSET_OF_ST_ATIM..Self::OFFSET_OF_ST_ATIM + Self::SIZE_OF_ST_ATIM],
        )?;

        // Parse last modification time field.
        let st_mtim: timespec = timespec::try_from_bytes(
            &bytes[Self::OFFSET_OF_ST_MTIM..Self::OFFSET_OF_ST_MTIM + Self::SIZE_OF_ST_MTIM],
        )?;

        // Parse last status change time field.
        let st_ctim: timespec = timespec::try_from_bytes(
            &bytes[Self::OFFSET_OF_ST_CTIM..Self::OFFSET_OF_ST_CTIM + Self::SIZE_OF_ST_CTIM],
        )?;

        // Parse block size field.
        let st_blksize: blksize_t = blksize_t::from_ne_bytes(
            bytes
                [Self::OFFSET_OF_ST_BLKSIZE..Self::OFFSET_OF_ST_BLKSIZE + Self::SIZE_OF_ST_BLKSIZE]
                .try_into()
                .map_err(|_| {
                    Error::new(ErrorCode::InvalidArgument, "failed to parse st_blksize")
                })?,
        );

        // Parse number of blocks allocated field.
        let st_blocks: blkcnt_t = blkcnt_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_ST_BLOCKS..Self::OFFSET_OF_ST_BLOCKS + Self::SIZE_OF_ST_BLOCKS]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse st_blocks"))?,
        );

        Ok(Self {
            st_dev,
            st_ino,
            st_mode,
            st_nlink,
            st_uid,
            st_gid,
            st_rdev,
            st_size,
            st_atim,
            st_mtim,
            st_ctim,
            st_blksize,
            st_blocks,
        })
    }
}
