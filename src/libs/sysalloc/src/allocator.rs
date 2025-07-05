// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

// The following imports are used only when any logging feature is enabled.
#[allow(unused_imports)]
use ::core::fmt::Write;

#[cfg(not(feature = "rustc-dep-of-std"))]
use ::core::alloc::GlobalAlloc;

use crate::heap::Heap;
use ::alloc::alloc::Layout;
use ::arch::mem::{
    PAGE_ALIGNMENT,
    PAGE_SIZE,
};
use ::core::ptr;
use ::spin::{
    Mutex,
    MutexGuard,
};
use ::sys::{
    config::memory_layout::USER_HEAP_BASE,
    error::{
        Error,
        ErrorCode,
    },
    kcall,
    mm::{
        self,
        Address,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
};
use ::talc::*;

//==================================================================================================
// Constants
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(feature = "staticlib")] {
        /// Heap size for Rust runtime.
        const RUST_HEAP_SIZE: usize = config::memory_layout::USER_HEAP_SIZE/2;
        /// Heap size for C runtime.
        pub const C_HEAP_SIZE: usize = config::memory_layout::USER_HEAP_SIZE/2;
    } else  {
        /// Heap size for Rust runtime.
        const RUST_HEAP_SIZE: usize = config::memory_layout::USER_HEAP_SIZE;
        /// Heap size for C runtime.
        pub const C_HEAP_SIZE: usize = 0;
    }
}

/// Based address for break address.
pub const BREAK_BASE_RAW: usize = config::memory_layout::USER_HEAP_BASE_RAW + RUST_HEAP_SIZE;

//==================================================================================================
//  Allocator
//==================================================================================================

#[cfg(not(feature = "rustc-dep-of-std"))]
struct Allocator;

static HEAP: Mutex<Option<Talc<NanvixOomHandler>>> = Mutex::new(None);

#[cfg_attr(not(feature = "rustc-dep-of-std"), global_allocator)]
#[cfg(not(feature = "rustc-dep-of-std"))]
static mut ALLOCATOR: Allocator = Allocator;

//==================================================================================================
// Out-of-Memory Handler
//==================================================================================================

struct NanvixOomHandler {
    heap: Heap,
    span: Option<Span>,
}

impl NanvixOomHandler {
    fn new(
        pid: ProcessIdentifier,
        base: VirtualAddress,
        size: usize,
        capacity: usize,
    ) -> Result<Talc<Self>, Error> {
        let heap: Heap = Heap::new(pid, base, size, capacity)?;

        let oom_handler: NanvixOomHandler = Self { heap, span: None };

        let mut talc: Talc<NanvixOomHandler> = Talc::new(oom_handler);

        let memory: Span = Span::from_base_size(base.as_mut_ptr(), size);

        unsafe {
            // Attempt to claim initial memory.
            match talc.claim(memory) {
                Ok(span) => {
                    if span.size() != size {
                        let _diff: usize = size.abs_diff(span.size());
                        #[cfg(feature = "warn")]
                        let _ = writeln!(
                            &mut Logger::get(module_path!(), LogLevel::Warn),
                            "new(): claimed {} fewer bytes",
                            _diff
                        );
                    }

                    // Save claimed memory.
                    talc.oom_handler.span = Some(span);
                },
                Err(_) => return Err(Error::new(ErrorCode::BadAddress, "failed to claim memory")),
            }
        }

        Ok(talc)
    }
}

impl OomHandler for NanvixOomHandler {
    fn handle_oom(talc: &mut Talc<Self>, layout: core::alloc::Layout) -> Result<(), ()> {
        let increment: usize = mm::align_up(layout.size(), PAGE_ALIGNMENT);

        let old_heap: Span = talc
            .oom_handler
            .span
            .expect("heap should have an initial span");

        // Check if we have to grow the heap.
        if old_heap.size() + increment > talc.oom_handler.heap.size() {
            // let increment: usize = mm::align_up(increment, PAGE_ALIGNMENT);
            // Attempt to grow the heap.
            if talc.oom_handler.heap.grow(increment).is_err() {
                #[cfg(feature = "warn")]
                let _ = writeln!(
                    &mut Logger::get(module_path!(), LogLevel::Warn),
                    "failed to grow heap by {} bytes",
                    increment
                );
                return Err(());
            }
        }

        let req_heap: Span = Span::from_base_size(
            talc.oom_handler.heap.base().as_mut_ptr(),
            talc.oom_handler.heap.size(),
        );

        unsafe {
            let span = talc.extend(old_heap, req_heap);
            if span.size() != req_heap.size() {
                let _diff: usize = req_heap.size().abs_diff(span.size());
                #[cfg(feature = "warn")]
                let _ = writeln!(
                    &mut Logger::get(module_path!(), LogLevel::Warn),
                    "handle_oom(): claimed {} fewer bytes",
                    _diff
                );
            }

            // Save claimed memory.
            talc.oom_handler.span = Some(span);
        }

        Ok(())
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the heap.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead
///
#[allow(static_mut_refs)]
pub fn init() -> Result<(), Error> {
    let pid: ProcessIdentifier = kcall::pm::getpid()?;

    let addr: VirtualAddress = USER_HEAP_BASE;
    let size: usize = PAGE_SIZE;
    let capacity: usize = RUST_HEAP_SIZE;

    let mut locked_heap: MutexGuard<'_, Option<Talc<NanvixOomHandler>>> = HEAP.lock();
    // Check if the heap was already initialized.
    if locked_heap.is_some() {
        return Err(Error::new(ErrorCode::ResourceBusy, "heap already initialized"));
    }

    *locked_heap = Some(NanvixOomHandler::new(pid, addr, size, capacity)?);

    Ok(())
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    let mut locked_heap: MutexGuard<'_, Option<Talc<NanvixOomHandler>>> = HEAP.lock();
    if let Some(heap) = locked_heap.as_mut() {
        match heap.malloc(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => core::ptr::null_mut(),
        }
    } else {
        // Heap is not initialized.
        core::ptr::null_mut()
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
    let mut locked_heap: MutexGuard<'_, Option<Talc<NanvixOomHandler>>> = HEAP.lock();
    if let Some(heap) = locked_heap.as_mut() {
        if let Some(ptr) = ptr::NonNull::new(ptr) {
            heap.free(ptr, layout)
        }
    }
}

#[cfg(not(feature = "rustc-dep-of-std"))]
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        dealloc(ptr, layout)
    }
}

/// Cleanups the memory management runtime.
pub fn cleanup() -> Result<(), Error> {
    Ok(())
}
