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
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        self,
        Address,
        VirtualAddress,
    },
    pm::{
        SigSet,
        SIG_SETMASK,
    },
};
use ::talc::*;

//==================================================================================================
//  Allocator
//==================================================================================================

#[cfg(not(feature = "rustc-dep-of-std"))]
struct Allocator;

static HEAP: Mutex<Option<Talc<NanvixOomHandler>>> = Mutex::new(None);

#[cfg_attr(not(feature = "rustc-dep-of-std"), global_allocator)]
#[cfg(not(feature = "rustc-dep-of-std"))]
static mut ALLOCATOR: Allocator = Allocator;

/// Holds the heap mutex across a process duplication.
pub struct HeapForkGuard {
    _private: (),
}

impl HeapForkGuard {
    /// Locks the heap before duplicating the calling process.
    ///
    /// # Safety
    ///
    /// Signal delivery must be blocked, and the caller must not allocate or deallocate until this
    /// guard is dropped in both the parent and child.
    pub unsafe fn acquire() -> Self {
        let guard = HEAP.lock();
        core::mem::forget(guard);
        Self { _private: () }
    }
}

impl Drop for HeapForkGuard {
    fn drop(&mut self) {
        // SAFETY: `acquire()` forgot the unique guard that locked this process's copy. After fork,
        // the parent and child each unlock their own private copy exactly once.
        unsafe {
            HEAP.force_unlock();
        }
    }
}

/// Blocks signal delivery while the heap mutex is held across kernel calls.
struct SignalMaskGuard {
    previous: SigSet,
}

impl SignalMaskGuard {
    fn acquire() -> Result<Self, Error> {
        let blocked: SigSet = !0;
        let mut previous: SigSet = 0;
        unsafe {
            ::sys::kcall::pm::__kcall_sigprocmask(SIG_SETMASK, &blocked, &mut previous)?;
        }
        Ok(Self { previous })
    }
}

impl Drop for SignalMaskGuard {
    fn drop(&mut self) {
        unsafe {
            let _result: Result<(), Error> = ::sys::kcall::pm::__kcall_sigprocmask(
                SIG_SETMASK,
                &self.previous,
                core::ptr::null_mut(),
            );
        }
    }
}

//==================================================================================================
// Out-of-Memory Handler
//==================================================================================================

struct NanvixOomHandler {
    heap: Heap,
    span: Option<Span>,
    growth_enabled: bool,
}

impl NanvixOomHandler {
    fn new(base: VirtualAddress, size: usize, capacity: usize) -> Result<Talc<Self>, Error> {
        let heap: Heap = Heap::new(base, size, capacity)?;

        let oom_handler: NanvixOomHandler = Self {
            heap,
            span: None,
            growth_enabled: false,
        };

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
        if !talc.oom_handler.growth_enabled {
            return Err(());
        }

        let old_heap: Span = talc
            .oom_handler
            .span
            .expect("heap should have an initial span");

        // If the Talc span does not yet cover all committed backing memory, extend the span
        // without growing. This reclaims committed pages that are not yet visible to Talc and
        // avoids unnecessary heap growth.
        if old_heap.size() < talc.oom_handler.heap.size() {
            let req_heap: Span = Span::from_base_size(
                talc.oom_handler.heap.base().as_mut_ptr(),
                talc.oom_handler.heap.size(),
            );

            unsafe {
                let span: Span = talc.extend(old_heap, req_heap);
                #[cfg(feature = "warn")]
                if span.size() != req_heap.size() {
                    let diff: usize = req_heap.size().abs_diff(span.size());
                    let _ = writeln!(
                        &mut Logger::get(module_path!(), LogLevel::Warn),
                        "handle_oom(): span reclamation claimed {} fewer bytes",
                        diff
                    );
                }
                talc.oom_handler.span = Some(span);
            }

            return Ok(());
        }

        // The span already covers all committed memory — grow the backing heap.
        //
        // Round up to page alignment and add one extra page so that the allocator's per-chunk
        // metadata overhead never causes the growth to fall just short of the required chunk
        // size. Without this margin, a page-aligned layout.size() produces a growth of exactly
        // layout.size() bytes, but the allocator needs layout.size() + TAG_SIZE for the chunk,
        // triggering a redundant second OOM call that doubles heap consumption per allocation.
        let aligned: usize = match mm::align_up(layout.size(), PAGE_ALIGNMENT) {
            Some(v) => v,
            None => {
                #[cfg(feature = "warn")]
                let _ = writeln!(
                    &mut Logger::get(module_path!(), LogLevel::Warn),
                    "handle_oom(): align_up overflow (layout_size={})",
                    layout.size()
                );
                return Err(());
            },
        };
        let increment: usize = aligned.saturating_add(PAGE_SIZE);

        // Attempt to grow with the overhead page. If that fails (near capacity), fall back to
        // the exact aligned increment — existing free space inside Talc may supply the missing
        // metadata bytes.
        if talc.oom_handler.heap.grow(increment).is_err()
            && talc.oom_handler.heap.grow(aligned).is_err()
        {
            #[cfg(feature = "warn")]
            let _ = writeln!(
                &mut Logger::get(module_path!(), LogLevel::Warn),
                "failed to grow heap by {} bytes",
                increment
            );
            return Err(());
        }

        let req_heap: Span = Span::from_base_size(
            talc.oom_handler.heap.base().as_mut_ptr(),
            talc.oom_handler.heap.size(),
        );

        unsafe {
            let span: Span = talc.extend(old_heap, req_heap);
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
/// - `base`: Base virtual address for the heap. Must be page-aligned and reside within a
///   valid mmap region.
/// - `capacity`: Maximum size of the heap in bytes.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
#[allow(static_mut_refs)]
pub fn init(base: VirtualAddress, capacity: usize) -> Result<(), Error> {
    let size: usize = PAGE_SIZE;

    let _signal_mask: SignalMaskGuard = SignalMaskGuard::acquire()?;
    let mut locked_heap: MutexGuard<'_, Option<Talc<NanvixOomHandler>>> = HEAP.lock();
    // Check if the heap was already initialized.
    if locked_heap.is_some() {
        return Err(Error::new(ErrorCode::ResourceBusy, "heap already initialized"));
    }

    *locked_heap = Some(NanvixOomHandler::new(base, size, capacity)?);

    Ok(())
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    // Signals are delivered only at kernel-call return, so a heap-only attempt cannot be
    // interrupted by a handler.
    {
        let mut locked_heap: MutexGuard<'_, Option<Talc<NanvixOomHandler>>> = HEAP.lock();
        let Some(heap): Option<&mut Talc<NanvixOomHandler>> = locked_heap.as_mut() else {
            return core::ptr::null_mut();
        };
        if let Ok(ptr) = heap.malloc(layout) {
            return ptr.as_ptr();
        }
    }

    // OOM handling may call mmap while holding the heap lock.
    let Ok(_signal_mask): Result<SignalMaskGuard, Error> = SignalMaskGuard::acquire() else {
        return core::ptr::null_mut();
    };
    let mut locked_heap: MutexGuard<'_, Option<Talc<NanvixOomHandler>>> = HEAP.lock();
    let Some(heap): Option<&mut Talc<NanvixOomHandler>> = locked_heap.as_mut() else {
        return core::ptr::null_mut();
    };
    heap.oom_handler.growth_enabled = true;
    let result: Result<ptr::NonNull<u8>, ()> = heap.malloc(layout);
    heap.oom_handler.growth_enabled = false;
    match result {
        Ok(ptr) => ptr.as_ptr(),
        Err(_) => core::ptr::null_mut(),
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
    let nn_ptr = match ptr::NonNull::new(ptr) {
        Some(p) => p,
        None => return,
    };

    let should_reclaim: Result<bool, Error> = {
        let mut locked_heap: MutexGuard<'_, Option<Talc<NanvixOomHandler>>> = HEAP.lock();
        let Some(heap): Option<&mut Talc<NanvixOomHandler>> = locked_heap.as_mut() else {
            return;
        };
        heap.free(nn_ptr, layout);
        reclaim_target(heap).map(|target| target.is_some())
    };

    match should_reclaim {
        Ok(true) => {},
        Ok(false) => return,
        Err(error) => {
            ::syslog::warn!(
                "dealloc(): failed to inspect heap for reclamation (error={:?})",
                error
            );
            return;
        },
    }

    // Recompute the target under the lock because another thread may have changed the heap after
    // the unmasked probe.
    let Ok(_signal_mask): Result<SignalMaskGuard, Error> = SignalMaskGuard::acquire() else {
        return;
    };
    let mut locked_heap: MutexGuard<'_, Option<Talc<NanvixOomHandler>>> = HEAP.lock();
    if let Some(heap) = locked_heap.as_mut() {
        if let Err(error) = try_reclaim(heap) {
            ::syslog::warn!("dealloc(): failed to reclaim heap (error={:?})", error);
        }
    }
}

/// Finds the smallest committed heap size that retains every live allocation.
///
/// Only activates when the heap has reached its maximum capacity (`heap_size >= capacity`).
/// After a successful reclaim the committed size drops below capacity, so subsequent
/// `dealloc` calls skip reclamation cheaply. The next time `grow()` pushes the heap
/// back to capacity the guard fires again, giving a natural "reclaim every time we
/// reach capacity" cadence without per-dealloc overhead.
fn reclaim_target(talc: &Talc<NanvixOomHandler>) -> Result<Option<usize>, Error> {
    // Only reclaim when the heap has reached its maximum capacity.
    let heap_size: usize = talc.oom_handler.heap.size();
    let capacity: usize = talc.oom_handler.heap.capacity();
    if heap_size < capacity || heap_size <= PAGE_SIZE {
        return Ok(None);
    }

    let current_span: Span = talc
        .oom_handler
        .span
        .ok_or_else(|| Error::new(ErrorCode::BadAddress, "heap span is unavailable"))?;

    // Find the minimum span that contains all live allocations.
    let allocated_span: Span = unsafe { talc.get_allocated_span(current_span) };

    let base_raw: usize = talc.oom_handler.heap.base().into_raw_value();

    // Compute the page-aligned high-water mark of live allocations.
    let alloc_end: usize = if allocated_span.is_empty() {
        // No live allocations — shrink to the initial page.
        PAGE_SIZE
    } else {
        // Round up end of live allocations to a page boundary.
        let (_, acme) = allocated_span.get_base_acme().ok_or_else(|| {
            Error::new(ErrorCode::BadAddress, "non-empty heap span has no bounds")
        })?;
        let raw_end: usize = acme as usize;
        let relative_end: usize = raw_end
            .checked_sub(base_raw)
            .ok_or_else(|| Error::new(ErrorCode::BadAddress, "heap span precedes heap base"))?;
        mm::align_up(relative_end, PAGE_ALIGNMENT)
            .ok_or_else(|| Error::new(ErrorCode::ValueOutOfRange, "heap span end overflows"))?
    };

    let current_size: usize = talc.oom_handler.heap.size();

    // Only reclaim if we can free at least one page.
    if alloc_end >= current_size {
        return Ok(None);
    }

    Ok(Some(alloc_end))
}

/// Truncates the allocator span and unmaps reclaimable tail pages.
fn try_reclaim(talc: &mut Talc<NanvixOomHandler>) -> Result<(), Error> {
    let Some(alloc_end): Option<usize> = reclaim_target(talc)? else {
        return Ok(());
    };

    let current_span: Span = talc
        .oom_handler
        .span
        .ok_or_else(|| Error::new(ErrorCode::BadAddress, "heap span is unavailable"))?;

    // Truncate Talc's span BEFORE unmapping pages. The truncate() call reads
    // metadata (gap sizes, tags) from the old heap region. If we unmap first,
    // those reads hit unmapped pages and cause a guest page fault.
    let new_span: Span = Span::from_base_size(talc.oom_handler.heap.base().as_mut_ptr(), alloc_end);
    let span: Span = unsafe { talc.truncate(current_span, new_span) };
    talc.oom_handler.span = Some(span);

    // Now unmap the freed tail pages. If this fails, shrink() stops at the
    // first failing page and updates heap.size() to reflect the actual mapped
    // extent. Talc's span is already truncated so unmapped pages won't be
    // reused; the still-mapped (but now outside Talc's view) pages are a
    // benign leak that the next OOM cycle can reclaim.
    talc.oom_handler.heap.shrink(alloc_end)
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

/// Returns the committed size of the heap in bytes.
///
/// This is the number of bytes currently backed by physical pages. It increases when the OOM
/// handler grows the heap via `mmap` and decreases when `try_reclaim` shrinks it via `munmap`.
pub fn heap_committed_size() -> usize {
    let Ok(_signal_mask): Result<SignalMaskGuard, Error> = SignalMaskGuard::acquire() else {
        return 0;
    };
    let locked_heap: MutexGuard<'_, Option<Talc<NanvixOomHandler>>> = HEAP.lock();
    match locked_heap.as_ref() {
        Some(heap) => heap.oom_handler.heap.size(),
        None => 0,
    }
}

/// Returns the maximum capacity of the heap in bytes.
pub fn heap_capacity() -> usize {
    let Ok(_signal_mask): Result<SignalMaskGuard, Error> = SignalMaskGuard::acquire() else {
        return 0;
    };
    let locked_heap: MutexGuard<'_, Option<Talc<NanvixOomHandler>>> = HEAP.lock();
    match locked_heap.as_ref() {
        Some(heap) => heap.oom_handler.heap.capacity(),
        None => 0,
    }
}

/// C-callable wrapper for [`heap_committed_size`].
#[unsafe(no_mangle)]
pub extern "C" fn sysalloc_heap_committed_size() -> usize {
    heap_committed_size()
}

/// C-callable wrapper for [`heap_capacity`].
#[unsafe(no_mangle)]
pub extern "C" fn sysalloc_heap_capacity() -> usize {
    heap_capacity()
}
