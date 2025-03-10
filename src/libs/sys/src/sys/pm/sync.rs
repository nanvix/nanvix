// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mm::{
    Address,
    VirtualAddress,
};

//==================================================================================================
// MutexAddress
//==================================================================================================

///
/// # Description
///
/// A type that represents the address of a mutex.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MutexAddress {
    addr: VirtualAddress,
}

impl From<usize> for MutexAddress {
    fn from(raw_addr: usize) -> Self {
        MutexAddress {
            addr: VirtualAddress::from_raw_value(raw_addr),
        }
    }
}

impl From<MutexAddress> for usize {
    fn from(addr: MutexAddress) -> usize {
        addr.addr.into_raw_value()
    }
}

//==================================================================================================
// ConditionAddress
//==================================================================================================

///
/// # Description
///
/// A type that represents the address of a condition variable.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConditionAddress {
    addr: VirtualAddress,
}

impl From<usize> for ConditionAddress {
    fn from(raw_addr: usize) -> Self {
        ConditionAddress {
            addr: VirtualAddress::from_raw_value(raw_addr),
        }
    }
}

impl From<ConditionAddress> for usize {
    fn from(addr: ConditionAddress) -> usize {
        addr.addr.into_raw_value()
    }
}
