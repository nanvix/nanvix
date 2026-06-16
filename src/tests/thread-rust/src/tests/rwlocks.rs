// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::KernelThread;
use ::core::{
    ptr,
    sync::atomic::{
        AtomicU8,
        Ordering,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    pthread::PTHREAD_RWLOCK_INITIALIZER,
    sys_types::{
        pthread_rwlock_t,
        pthread_rwlockattr_t,
    },
};
use ::syscall::pthread::{
    pthread_rwlock_destroy,
    pthread_rwlock_init,
    pthread_rwlock_rdlock,
    pthread_rwlock_unlock,
    pthread_rwlock_wrlock,
};

//==================================================================================================
// Globals — Static Init Test
//==================================================================================================

static mut STATIC_RWLOCK: pthread_rwlock_t = PTHREAD_RWLOCK_INITIALIZER;

/// Stage state machine: 0→initial, 1→reader A acquired, 2→reader B acquired, 3→A released, 4→B
/// released.
static STAGE: AtomicU8 = AtomicU8::new(0);

/// Set to 1 when reader B acquires the read lock while reader A still holds it.
static CONCURRENCY_OBSERVED: AtomicU8 = AtomicU8::new(0);

//==================================================================================================
// Globals — Dynamic Init Test
//==================================================================================================

static mut DYNAMIC_RWLOCK: pthread_rwlock_t = 0;
static WRITER_INSIDE: AtomicU8 = AtomicU8::new(0);
static READER_INSIDE: AtomicU8 = AtomicU8::new(0);
static READER_STARTED_AFTER_WRITER: AtomicU8 = AtomicU8::new(0);

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Exercises read-write lock static and dynamic initialization.
pub fn run() -> Result<(), Error> {
    test_rwlock_static_init()?;
    test_rwlock_dynamic_init()?;
    Ok(())
}

//==================================================================================================
// Static Init — Concurrent Readers (ports rwlock_static_init.c)
//==================================================================================================

fn test_rwlock_static_init() -> Result<(), Error> {
    unsafe {
        STATIC_RWLOCK = PTHREAD_RWLOCK_INITIALIZER;
    }
    STAGE.store(0, Ordering::Relaxed);
    CONCURRENCY_OBSERVED.store(0, Ordering::Relaxed);

    let reader_a = KernelThread::spawn(reader_a_entry, 0)?;
    let reader_b = KernelThread::spawn(reader_b_entry, 0)?;

    let ret_a = reader_a.join()?;
    let ret_b = reader_b.join()?;

    assert_eq!(ret_a, 0, "reader_a returned unexpected status");
    assert_eq!(ret_b, 0, "reader_b returned unexpected status");
    assert_eq!(STAGE.load(Ordering::Acquire), 4, "stage machine did not reach final state");
    assert_eq!(CONCURRENCY_OBSERVED.load(Ordering::Acquire), 1, "concurrent readers not observed");

    // Destroy the lock now that both readers have released.
    // SAFETY: no other thread holds or references the rwlock.
    unsafe {
        pthread_rwlock_destroy(&mut *ptr::addr_of_mut!(STATIC_RWLOCK))?;
    }

    Ok(())
}

extern "C" fn reader_a_entry(_arg: usize) -> usize {
    reader_a_impl().unwrap_or_else(|err| panic!("reader_a: {err:?}"))
}

fn reader_a_impl() -> Result<usize, Error> {
    // SAFETY: rwlock functions use the address as a lookup key; the u32 value is not mutated
    // concurrently by the implementation.
    unsafe {
        pthread_rwlock_rdlock(&mut *ptr::addr_of_mut!(STATIC_RWLOCK))?;
    }
    STAGE.store(1, Ordering::Release);

    // Wait until reader B also acquires the read lock.
    while STAGE.load(Ordering::Acquire) < 2 {
        ::sys::kcall::sched::__kcall_sched_yield()?;
    }

    unsafe {
        pthread_rwlock_unlock(&mut *ptr::addr_of_mut!(STATIC_RWLOCK))?;
    }
    STAGE.store(3, Ordering::Release);
    Ok(0)
}

extern "C" fn reader_b_entry(_arg: usize) -> usize {
    reader_b_impl().unwrap_or_else(|err| panic!("reader_b: {err:?}"))
}

fn reader_b_impl() -> Result<usize, Error> {
    // Wait until reader A has acquired.
    while STAGE.load(Ordering::Acquire) < 1 {
        ::sys::kcall::sched::__kcall_sched_yield()?;
    }

    // SAFETY: see reader_a_impl.
    unsafe {
        pthread_rwlock_rdlock(&mut *ptr::addr_of_mut!(STATIC_RWLOCK))?;
    }

    // Reader A still holds the lock (stage==1). A successful rdlock proves concurrency.
    if STAGE.load(Ordering::Acquire) == 1 {
        CONCURRENCY_OBSERVED.store(1, Ordering::Release);
    }

    STAGE.store(2, Ordering::Release);

    // Wait until reader A releases before we release.
    while STAGE.load(Ordering::Acquire) < 3 {
        ::sys::kcall::sched::__kcall_sched_yield()?;
    }

    unsafe {
        pthread_rwlock_unlock(&mut *ptr::addr_of_mut!(STATIC_RWLOCK))?;
    }
    STAGE.store(4, Ordering::Release);
    Ok(0)
}

//==================================================================================================
// Dynamic Init — Writer/Reader Exclusion (ports rwlock_dynamic_init.c)
//==================================================================================================

fn test_rwlock_dynamic_init() -> Result<(), Error> {
    unsafe {
        DYNAMIC_RWLOCK = 0;
    }
    WRITER_INSIDE.store(0, Ordering::Relaxed);
    READER_INSIDE.store(0, Ordering::Relaxed);
    READER_STARTED_AFTER_WRITER.store(0, Ordering::Relaxed);

    let attr: pthread_rwlockattr_t = pthread_rwlockattr_t::default();

    // SAFETY: single-threaded access during initialization.
    unsafe {
        pthread_rwlock_init(&mut *ptr::addr_of_mut!(DYNAMIC_RWLOCK), &attr)?;
    }

    // Verify that destroying while write-locked fails.
    unsafe {
        pthread_rwlock_wrlock(&mut *ptr::addr_of_mut!(DYNAMIC_RWLOCK))?;
    }
    let destroy_result: Result<(), Error> =
        unsafe { pthread_rwlock_destroy(&mut *ptr::addr_of_mut!(DYNAMIC_RWLOCK)) };
    assert!(destroy_result.is_err(), "destroy while locked must fail");
    if let Err(err) = destroy_result {
        assert_eq!(
            err.code,
            ErrorCode::ResourceBusy,
            "expected ResourceBusy on destroy of locked rwlock"
        );
    }
    unsafe {
        pthread_rwlock_unlock(&mut *ptr::addr_of_mut!(DYNAMIC_RWLOCK))?;
    }

    // Launch writer that holds the lock for a while.
    let writer = KernelThread::spawn(dyn_writer_entry, 0)?;

    // Wait until writer is inside.
    while WRITER_INSIDE.load(Ordering::Acquire) == 0 {
        ::sys::kcall::sched::__kcall_sched_yield()?;
    }

    // Launch reader which must block until writer releases.
    let reader = KernelThread::spawn(dyn_reader_entry, 0)?;

    // While writer holds the lock, reader must not be inside.
    while WRITER_INSIDE.load(Ordering::Acquire) == 1 {
        assert_eq!(
            READER_INSIDE.load(Ordering::Acquire),
            0,
            "reader entered while writer holds lock"
        );
        ::sys::kcall::sched::__kcall_sched_yield()?;
    }

    let ret_w = writer.join()?;
    let ret_r = reader.join()?;
    assert_eq!(ret_w, 0, "writer returned unexpected status");
    assert_eq!(ret_r, 0, "reader returned unexpected status");
    assert_eq!(
        READER_STARTED_AFTER_WRITER.load(Ordering::Acquire),
        1,
        "reader must start only after writer exits"
    );

    // Destroy must succeed now that no one holds the lock.
    unsafe {
        pthread_rwlock_destroy(&mut *ptr::addr_of_mut!(DYNAMIC_RWLOCK))?;
    }

    Ok(())
}

extern "C" fn dyn_writer_entry(_arg: usize) -> usize {
    dyn_writer_impl().unwrap_or_else(|err| panic!("dyn_writer: {err:?}"))
}

fn dyn_writer_impl() -> Result<usize, Error> {
    // SAFETY: see reader_a_impl.
    unsafe {
        pthread_rwlock_wrlock(&mut *ptr::addr_of_mut!(DYNAMIC_RWLOCK))?;
    }
    WRITER_INSIDE.store(1, Ordering::Release);

    // Hold the lock for a while.
    for _ in 0..4000_u32 {
        ::sys::kcall::sched::__kcall_sched_yield()?;
    }

    WRITER_INSIDE.store(0, Ordering::Release);
    unsafe {
        pthread_rwlock_unlock(&mut *ptr::addr_of_mut!(DYNAMIC_RWLOCK))?;
    }
    Ok(0)
}

extern "C" fn dyn_reader_entry(_arg: usize) -> usize {
    dyn_reader_impl().unwrap_or_else(|err| panic!("dyn_reader: {err:?}"))
}

fn dyn_reader_impl() -> Result<usize, Error> {
    // SAFETY: see reader_a_impl.
    unsafe {
        pthread_rwlock_rdlock(&mut *ptr::addr_of_mut!(DYNAMIC_RWLOCK))?;
    }

    // By the time the reader acquires the lock the writer should have left.
    if WRITER_INSIDE.load(Ordering::Acquire) == 0 {
        READER_STARTED_AFTER_WRITER.store(1, Ordering::Release);
    }
    READER_INSIDE.store(1, Ordering::Release);

    // Hold the read lock briefly.
    for _ in 0..2000_u32 {
        ::sys::kcall::sched::__kcall_sched_yield()?;
    }

    READER_INSIDE.store(0, Ordering::Release);
    unsafe {
        pthread_rwlock_unlock(&mut *ptr::addr_of_mut!(DYNAMIC_RWLOCK))?;
    }
    Ok(0)
}
