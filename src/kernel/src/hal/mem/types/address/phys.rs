// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("phys.spec.rs");
#[cfg(verus_keep_ghost)]
include!("phys.proof.rs");

use crate::hal::mem::types::address::{
    Address,
    FrameAddress,
    VirtualAddress,
};
use ::arch::mem::{
    self,
    paging::FrameNumber,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::Alignment,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a physical address.
///
#[verus_verify(external_derive)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(VirtualAddress);

//==================================================================================================
// Implementations
//==================================================================================================

impl PhysicalAddress {
    pub fn from_virtual_address(addr: VirtualAddress) -> Result<Self, Error> {
        // Delegate to the per-platform validator to support sparse physical memory layouts.
        if !crate::hal::platform::is_valid_physical_address(addr) {
            return Err(Error::new(
                ErrorCode::BadAddress,
                "address out of bounds of physical memory",
            ));
        }

        Ok(Self(addr))
    }

    pub fn into_virtual_address(self) -> VirtualAddress {
        self.0
    }

    ///
    /// # Description
    ///
    /// Constructs a [`PhysicalAddress`] from a [`FrameAddress`].
    ///
    /// # Parameters
    ///
    /// - `frame_addr`: The frame address.
    ///
    /// # Returns
    ///
    /// A [`PhysicalAddress`] associated with the given `frame_addr`.
    ///
    pub fn from_frame_address(frame_addr: FrameAddress) -> Self {
        let raw_addr: usize = frame_addr.into_raw_value() << mem::FRAME_SHIFT;
        Self(VirtualAddress::new(raw_addr))
    }

    pub fn from_into_frame_address(frame_addr: FrameAddress) -> Self {
        let raw_addr: usize = frame_addr.into_raw_value() << mem::FRAME_SHIFT;
        Self(VirtualAddress::new(raw_addr))
    }
}

// Verified conversions between a physical address, frame numbers, and MMIO addresses.
#[verus_verify]
impl PhysicalAddress {
    ///
    /// # Description
    ///
    /// Constructs a physical address from a memory-mapped I/O address.
    ///
    /// # Parameters
    ///
    /// - `addr`: The memory-mapped I/O address.
    ///
    /// # Return Values
    ///
    /// Upon success, a physical address associated with the given memory-mapped I/O address is
    /// returned. Upon failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if the provided memory-mapped I/O address is invalid.
    ///
    // Identity wrapping that deliberately bypasses the physical-RAM-range validator: MMIO
    // addresses may legally lie outside tracked RAM. On success the abstract address is unchanged.
    #[verus_spec(result =>
        requires
            spec_frame_number(addr@) <= spec_max_frame_number(),
        ensures
            result is Ok,
            (result->Ok_0)@ == addr@,
            (result->Ok_0).inv(),
    )]
    pub unsafe fn from_mmio_address(addr: VirtualAddress) -> Result<Self, Error> {
        Ok(Self(addr))
    }

    ///
    /// # Description
    ///
    /// Constructs a [`PhysicalAddress`] from a [`FrameNumber`].
    ///
    /// # Parameters
    ///
    /// - `frame`: The frame number.
    ///
    /// # Returns
    ///
    /// A [`PhysicalAddress`] associated with the given `frame_number`.
    ///
    // Total constructor: the result is the frame's base address, hence `FRAME_SIZE`-aligned.
    #[verus_spec(result =>
        ensures
            result@ == spec_from_number(spec_frame_raw_value(frame)),
    )]
    // VERUS REWRITE: the original `frame.into_raw_value() * mem::FRAME_SIZE` is split so the
    // `into_raw_value()` postcondition (`0 <= self@ <= spec_max()`) lands in context *before* the
    // overflow-bearing multiply, and `lemma_from_number_no_overflow` can be invoked between them.
    // The bound cannot be obtained via `use_type_invariant(frame)` because `FrameNumber`'s type
    // invariant is private to the `arch` crate (Verus: "missing type invariant function"), so the
    // intermediate `addr_raw` binding is mandatory. Same value, same operations, same complexity.
    // Reproducer: verus-ai-logs/nanvix-phys-hal-phys-address/cheating-elimination/repro/from_number.rs
    pub fn from_number(frame: FrameNumber) -> Self {
        let addr_raw: usize = frame.into_raw_value();
        proof! {
            lemma_from_number_no_overflow(frame);
        }
        let addr: usize = addr_raw * mem::FRAME_SIZE;
        Self(VirtualAddress::new(addr))
    }

    // Total projection (under `inv()`): identifies the frame containing the address,
    // `self@ / FRAME_SIZE` (equivalently `self@ >> FRAME_SHIFT`). `inv()` underwrites the unwrap.
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            spec_frame_raw_value(result) == spec_frame_number(self@),
    )]
    pub fn into_frame_number(self) -> FrameNumber {
        let raw_addr: usize = self.0.into_raw_value();
        let frame_number: usize = raw_addr >> mem::FRAME_SHIFT;
        proof! {
            vstd::arithmetic::power2::lemma2_to64();
            lemma_frame_index(self, raw_addr, mem::FRAME_SHIFT, frame_number);
        }
        // Safety: the following unwrap is safe because a physical address has a valid frame number.
        FrameNumber::from_raw_value(frame_number).unwrap()
    }
}

impl Address for PhysicalAddress {
    ///
    /// # Description
    ///
    /// Instantiates a new [`PhysicalAddress`] from a raw value.
    ///
    /// # Parameters
    ///
    /// - `raw_addr`: The raw value.
    ///
    /// # Returns
    ///
    /// - `Ok(Self)`: The new address.
    /// - `Err(Error::BadAddress)`: If the provided address is invalid.
    ///
    fn from_raw_value(value: usize) -> Result<Self, Error> {
        Self::from_virtual_address(VirtualAddress::from_raw_value(value))
    }

    ///
    /// # Description
    ///
    ///  Aligns the target [`PhysicalAddress`] to the provided `alignment`. If the address is already
    ///  aligned, it is returned as is.
    ///
    /// # Parameters
    ///
    /// - `alignment`: The alignment to align the target address to.
    ///
    /// # Returns
    ///
    /// Upon success, the aligned address is returned. Upon failure, an error is returned instead.
    ///
    fn align_up(&self, align: Alignment) -> Result<Self, Error> {
        let aligned: VirtualAddress = self.0.align_up(align).ok_or_else(|| {
            let reason: &str = "align_up overflow";
            error!(
                "PhysicalAddress::align_up(): {reason} (addr={:#x}, align={:?})",
                self.0.into_raw_value(),
                align
            );
            Error::new(ErrorCode::BadAddress, reason)
        })?;
        Self::from_virtual_address(aligned)
    }

    ///
    /// # Description
    ///
    /// Aligns the target [`PhysicalAddress`] down to the provided `alignment`. If the address is
    /// already aligned, it is returned as is.
    ///
    /// # Parameters
    ///
    /// - `alignment`: The alignment to align the target address to.
    ///
    /// # Returns
    ///
    /// Upon success, the aligned address is returned. Upon failure, an error is returned instead.
    ///
    fn align_down(&self, align: Alignment) -> Result<Self, Error> {
        Self::from_virtual_address(self.0.align_down(align))
    }

    ///
    /// # Description
    ///
    /// Checks if the target [`PhysicalAddress`] is aligned to the provided `alignment`.
    ///
    /// # Parameters
    ///
    /// - `alignment`: The alignment to check.
    ///
    /// # Returns
    ///
    /// Upon success, `true` is returned if the address is aligned, otherwise `false`. Upon failure,
    /// an error is returned instead.
    ///
    fn is_aligned(&self, align: Alignment) -> Result<bool, Error> {
        Ok(self.0.is_aligned(align))
    }

    ///
    /// # Description
    ///
    /// Returns the maximum address for [`PhysicalAddress`].
    ///
    /// # Returns
    ///
    /// The maximum [`PhysicalAddress`].
    ///
    fn max_addr() -> usize {
        crate::hal::platform::max_physical_address()
    }

    fn into_raw_value(self) -> usize {
        self.0.into_raw_value()
    }

    // VERUS REWRITE (interface addition): `clone_address` is a *required* method of the
    // `sys::mm::Address` trait, which gained it during the verus pipeline (it carries a verified
    // contract `result@ == self@` that the bare `derive(Clone)`/`Clone::clone` supertrait cannot
    // express — `Clone` has no Verus spec, so generic `Address` callers could not duplicate an
    // address while retaining the abstract-value guarantee). The trait method lives in the
    // out-of-scope `sys` crate (`src/libs/sys/src/sys/mm/address/mod.rs:88`); because
    // `PhysicalAddress` implements `Address`, this impl method is mandatory and cannot be removed
    // here. It is a view-preserving clone — same value, same complexity as a `Copy`. Recorded in
    // verus-ai-logs/nanvix-phys-hal-phys-address/verification_todo.md.
    fn clone_address(&self) -> Self {
        PhysicalAddress(self.0)
    }

    fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }

    fn as_mut_ptr(&self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

impl core::fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

//==================================================================================================
// Material for verification
//==================================================================================================

verus! {

impl View for PhysicalAddress
{
    type V = int;

    closed spec fn view(&self) -> int
    {
        self.0@
    }
}

} // end verus!
