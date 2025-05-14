// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

// The following imports are used only when any logging feature is enabled.
#[allow(unused_imports)]
use ::core::fmt::Write;
#[allow(unused_imports)]
use ::syslog::{
    LogLevel,
    Logger,
};

use crate::mm::heap::Heap;
use ::alloc::alloc::{
    GlobalAlloc,
    Layout,
};
use ::arch::mem::PAGE_ALIGNMENT;
use ::core::ptr;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        self,
        Address,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
};
use ::talc::*;

//==================================================================================================
//  Allocator
//==================================================================================================

struct Allocator;

static mut HEAP: Option<Talc<NanvixOomHandler>> = None;

#[global_allocator]
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
/// # Parameters
///
/// - `pid` - ID of the current process.
/// - `addr` - Start address of the heap.
/// - `size` - Size of the heap.
/// - `capacity` - Capacity of the heap.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead
///
#[allow(static_mut_refs)]
pub unsafe fn init(
    pid: ProcessIdentifier,
    addr: VirtualAddress,
    size: usize,
    capacity: usize,
) -> Result<(), Error> {
    // Check if the heap was already initialized.
    if HEAP.is_some() {
        return Err(Error::new(ErrorCode::ResourceBusy, "heap already initialized"));
    }

    HEAP = Some(NanvixOomHandler::new(pid, addr, size, capacity)?);

    Ok(())
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let heap = ptr::addr_of_mut!(HEAP);
        if let Some(heap) = &mut *heap {
            match heap.malloc(layout) {
                Ok(ptr) => ptr.as_ptr(),
                Err(_) => core::ptr::null_mut(),
            }
        } else {
            // Heap is not initialized.
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let heap = ptr::addr_of_mut!(HEAP);
        if let Some(heap) = &mut *heap {
            if let Some(ptr) = ptr::NonNull::new(ptr) {
                heap.free(ptr, layout)
            }
        }
    }
}
