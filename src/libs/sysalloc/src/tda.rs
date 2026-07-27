// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_ALIGNMENT;
use ::core::{
    alloc::Layout,
    cmp::max,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::align_up,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Offset of the cached thread ID within the TDA header, in bytes.
pub const TDA_TID_OFFSET: usize = core::mem::size_of::<*mut u8>();

/// Size of the cached thread ID field in the TDA header, in bytes.
pub const TDA_TID_SIZE: usize = core::mem::size_of::<u32>();

/// Offset of the cancellation state within the TDA header, in bytes.
pub const TDA_CANCEL_STATE_OFFSET: usize = TDA_TID_OFFSET + TDA_TID_SIZE;

/// Size of the cancellation state field in the TDA header, in bytes.
pub const TDA_CANCEL_STATE_SIZE: usize = core::mem::size_of::<u32>();

/// Default cancellation state for a newly allocated thread data area.
///
/// This value must match `sysapi::pthread::PTHREAD_CANCEL_ENABLE`.
const TDA_CANCEL_STATE_DEFAULT: u32 = 0;

/// Total size of the TDA header, in bytes.
const TDA_HEADER_SIZE: usize = TDA_CANCEL_STATE_OFFSET + TDA_CANCEL_STATE_SIZE;

/// Sentinel value indicating the cached thread ID has not been populated yet.
pub const TDA_TID_UNSET: u32 = 0;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Tracks whether the calling thread's TDA has been fully initialized.
///
/// # Safety Invariant
///
/// This flag is set to `true` by [`mark_initialized`] after `set_thread_data_area()` succeeds
/// during runtime init. Code that reads the TDA via a segment register must check this flag and
/// verify that the calling thread has a non-null TDA first.
///
/// This flag is process-wide and only indicates that TDA support was initialized. Raw kernel
/// threads may still have no TDA.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Marks the TDA subsystem as initialized.
///
/// Must be called after `set_thread_data_area()` succeeds during runtime init.
pub fn mark_initialized() {
    INITIALIZED.store(true, Ordering::Release);
}

/// Returns `true` if [`mark_initialized`] has been called.
pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::Acquire)
}

extern "C" {
    /// Linker symbol that marks the beginning of the TLS segment.
    static __TLS_START: u8;
    /// Linker symbol that marks the end of the TLS segment.
    static __TLS_END: u8;
}

//==================================================================================================
// Functions
//==================================================================================================

///
/// # Description
///
/// Allocates a new thread data area and initializes it with the contents of the main executable's
/// thread-local storage segment.
///
/// # Return Value
///
/// On successful completion, this function returns a pointer to the newly allocated thread data
/// area. It is the caller's responsibility to deallocate this memory using the [`dealloc`] function.
/// On failure, this function returns an error that contains the reason for the failure.
///
/// # Notes
///
/// This function uses the `__TLS_START` and `__TLS_END` linker symbols to determine the size of the
/// thread-local storage segment.
///
/// The thread data area will have the following layout:
///
/// ```text
/// High Address
///     ↑
/// +---------------------+ <- tda_ptr + tda_header_size (end of allocation)
/// |                     |
/// | Cancellation State  | c_int slot for pthread cancellation state
/// |                     | Initialized to PTHREAD_CANCEL_ENABLE
/// +---------------------+ <- tda_ptr + TDA_CANCEL_STATE_OFFSET
/// |                     |
/// | Cached Thread ID    | u32 slot for fast pthread_self()
/// |                     | Zero-initialized; lazily populated by pthread_self()
/// |                     | Read via segment register at TDA_TID_OFFSET
/// +---------------------+ <- tda_ptr + size_of::<*mut u8>()
/// |                     |
/// | Self-Reference      | Pointer to Thread Data Area (TDA)
/// | Pointer             | Value: tda_ptr (points to itself)
/// |                     |                (size_of::<*mut u8>() bytes)
/// +---------------------+ <- tda_ptr (returned to caller, segment base)
/// |                     |
/// | Padding (optional)  | Padding to meet alignment requirements between TLS and TDA pointer
/// |                     |
/// +---------------------+ <- tls_ptr + tls_size
/// |                     |
/// | Thread-Local        | Copy of __TLS_START to __TLS_END
/// | Storage (TLS)       | Size: tls_size bytes
/// |                     |
/// +---------------------+ <- tls_ptr = tda_ptr - align_up(tls_size, PAGE_ALIGNMENT)
/// |                     |
/// | Padding (optional)  | Padding to meet page alignment requirements for TLS segment
/// |                     |
/// +---------------------+ <- allocation (pointer returned by allocator)
///     ↓
/// Low Address
///
/// Where:
/// - TDA_HEADER_SIZE = TDA_CANCEL_STATE_OFFSET + TDA_CANCEL_STATE_SIZE
/// - allocation_size = allocation_padding_size + TDA_HEADER_SIZE
/// - allocation_padding_size = align_up(tls_size, max(PAGE_ALIGNMENT, align_of::<*mut u8>()))
/// - tda_ptr = allocation + allocation_padding_size
/// ```
///
pub fn alloc() -> Result<Option<*mut u8>, Error> {
    let tls_start_addr: usize = unsafe { &__TLS_START as *const u8 as usize };
    let tls_end_addr: usize = unsafe { &__TLS_END as *const u8 as usize };
    let tls_size: usize = tls_end_addr - tls_start_addr;
    let tls_alignment: usize = PAGE_ALIGNMENT.into();
    let allocation_alignment: usize = max(tls_alignment, core::mem::align_of::<*mut u8>());
    let allocation_padding_size: usize = align_up(tls_size, allocation_alignment.try_into()?)
        .ok_or_else(|| {
            ::syslog::warn!(
                "alloc(): align_up overflow (tls_size={tls_size}, \
                 allocation_alignment={allocation_alignment})"
            );
            Error::new(ErrorCode::OutOfMemory, "align_up overflow")
        })?;
    let allocation_size: usize = allocation_padding_size + TDA_HEADER_SIZE;

    ::syslog::trace!(
        "alloc(): tls_start_addr={tls_start_addr:x?}, tls_end_addr={tls_end_addr:x?}, \
         tls_size={tls_size}, tls_alignment={tls_alignment}, tda_header_size={TDA_HEADER_SIZE}, \
         allocation_alignment={allocation_alignment}, \
         allocation_padding_size={allocation_padding_size}, allocation_size={allocation_size}",
    );

    // Check if thread-local storage has an invalid alignment.
    if tls_size != 0 && !tls_start_addr.is_multiple_of(tls_alignment) {
        let reason: &'static str = "tls start address is not page-aligned";
        ::syslog::warn!(
            "alloc(): {reason} (tls_start_addr={tls_start_addr:x?}, tls_alignment={tls_alignment})",
        );
        return Err(Error::new(ErrorCode::ValueOutOfRange, reason));
    }

    // Compute allocation layout.
    let layout: Layout = match Layout::from_size_align(allocation_size, allocation_alignment) {
        Ok(layout) => layout,
        Err(_error) => {
            let reason: &'static str = "invalid layout for thread-local storage";
            ::syslog::warn!(
                "alloc(): {reason} (allocation_size={allocation_size}, \
                 allocation_alignment={allocation_alignment})",
            );
            return Err(Error::new(ErrorCode::ValueOutOfRange, reason));
        },
    };

    // Attempt to allocate thread data area and check for errors.
    // SAFETY: layout is non-zero and properly aligned.
    let allocation: *mut u8 = unsafe { crate::alloc(layout) };
    if allocation.is_null() {
        let reason: &'static str = "out of memory";
        ::syslog::warn!("init(): {reason}");
        return Err(Error::new(ErrorCode::OutOfMemory, reason));
    }

    // Compute pointer to thread data area.
    // SAFETY: `allocation` is non-null and pointer arithmetic is within bounds.
    let tda_ptr: *mut u8 = unsafe { allocation.add(allocation_padding_size) };

    // Store pointer to thread data area at the beginning of the allocation.
    // SAFETY: `tda_ptr` is non-null and properly aligned.
    let self_ptr: *mut *mut u8 = tda_ptr as *mut *mut u8;
    unsafe {
        *self_ptr = tda_ptr;
    }

    // Zero-initialize the cached thread ID at TDA_TID_OFFSET. The actual TID is populated lazily.
    let tid_ptr: *mut u32 = unsafe { tda_ptr.add(TDA_TID_OFFSET) as *mut u32 };
    unsafe {
        *tid_ptr = TDA_TID_UNSET;
    }

    // Initialize the calling thread's cancellation state.
    let cancellation_state_ptr: *mut u32 =
        unsafe { tda_ptr.add(TDA_CANCEL_STATE_OFFSET).cast::<u32>() };
    unsafe {
        *cancellation_state_ptr = TDA_CANCEL_STATE_DEFAULT;
    }

    // Compute pointer to thread-local storage.
    // SAFETY: `tda_ptr` is non-null and pointer arithmetic is within bounds.
    let tls_aligned_size: usize = align_up(tls_size, PAGE_ALIGNMENT).ok_or_else(|| {
        ::syslog::warn!("alloc(): align_up overflow (tls_size={tls_size}, tda_ptr={tda_ptr:?})");
        Error::new(ErrorCode::OutOfMemory, "align_up overflow")
    })?;
    let tls_ptr: *mut u8 = unsafe { tda_ptr.sub(tls_aligned_size) };

    // Initialize thread-local storage with the contents of the main executable's TLS segment.
    // SAFETY: `tls_ptr` is non-null and properly aligned. The copy is within bounds.
    unsafe {
        core::ptr::copy_nonoverlapping(&__TLS_START as *const u8, tls_ptr, tls_size);
    }

    Ok(Some(tda_ptr))
}

///
/// # Description
///
/// Cleans up the thread data area of the calling thread.
///
/// # Return Value
///
/// On success, this function returns empty. Otherwise, it returns an error object that contains the
/// reason for the failure.
///
pub fn cleanup() -> Result<(), sys::error::Error> {
    // Get the base address for thread data area.
    let tcb_ptr: *mut u8 = match sys::kcall::pm::__kcall_get_thread_data_area() {
        Ok(ptr) => ptr,
        Err(error) => {
            ::syslog::warn!("cleanup_tda(): {error:?}");
            return Err(error);
        },
    };

    // Check if thread-data area was not set.
    if tcb_ptr.is_null() {
        return Ok(());
    }

    // Clear the thread-local storage pointer before deallocating it to avoid a dangling segment
    // base.
    match sys::kcall::pm::__kcall_set_thread_data_area(core::ptr::null_mut()) {
        Ok(()) => unsafe { dealloc(tcb_ptr) },
        Err(error) => {
            ::syslog::warn!("cleanup_tda(): failed to clear tda pointer (error={error:?})");
            Err(error)
        },
    }
}

///
/// # Description
///
/// Deallocates a thread data area previously allocated using the [`alloc`] function.
///
/// # Parameters
///
/// - `tda_ptr`: Pointer to the thread data area to deallocate.
///
/// # Return Value
///
/// On success, this function returns empty. Otherwise, it returns an error object that contains the
/// reason for the failure.
///
/// # Safety
///
/// `tda_ptr` must have been returned by [`alloc`] and must not have been deallocated previously.
///
pub unsafe fn dealloc(tda_ptr: *mut u8) -> Result<(), Error> {
    extern "C" {
        static __TLS_START: u8;
        static __TLS_END: u8;
    }

    // Compute thread-local storage alignment and size based on _TLS_START and _TLS_END linker symbols.
    let tls_start_addr: usize = unsafe { &__TLS_START as *const u8 as usize };
    let tls_end_addr: usize = unsafe { &__TLS_END as *const u8 as usize };
    let tls_size: usize = tls_end_addr - tls_start_addr;
    let tls_alignment: usize = PAGE_ALIGNMENT.into();
    let allocation_alignment: usize = max(tls_alignment, core::mem::align_of::<*mut u8>());
    let allocation_padding_size: usize = align_up(tls_size, allocation_alignment.try_into()?)
        .ok_or_else(|| {
            ::syslog::warn!(
                "dealloc(): align_up overflow (tls_size={tls_size}, \
                 allocation_alignment={allocation_alignment})"
            );
            Error::new(ErrorCode::OutOfMemory, "align_up overflow")
        })?;
    let allocation_size: usize = allocation_padding_size + TDA_HEADER_SIZE;

    ::syslog::trace!(
        "cleanup(): tls_start_addr={tls_start_addr:x?}, tls_end_addr={tls_end_addr:x?}, \
         tls_size={tls_size}, tls_alignment={tls_alignment}, tda_header_size={TDA_HEADER_SIZE}, \
         allocation_alignment={allocation_alignment}, \
         allocation_padding_size={allocation_padding_size}, allocation_size={allocation_size}, \
         tda_ptr={tda_ptr:p}",
    );

    // Compute allocation layout.
    let layout: Layout = match Layout::from_size_align(allocation_size, allocation_alignment) {
        Ok(layout) => layout,
        Err(_error) => {
            let reason: &'static str = "invalid layout";
            ::syslog::warn!(
                "cleanup(): {reason} (allocation_size={allocation_size}, \
                 allocation_align={allocation_alignment})",
            );
            return Err(Error::new(ErrorCode::ValueOutOfRange, reason));
        },
    };

    // Compute pointer to the beginning of the allocation.
    // SAFETY: `tda_ptr` is non-null and pointer arithmetic is within bounds.
    let allocation: *mut u8 = unsafe { tda_ptr.sub(allocation_padding_size) };

    // Deallocate the memory.
    // SAFETY: `allocation` was allocated using the same layout.
    unsafe {
        crate::dealloc(allocation, layout);
    }

    Ok(())
}
