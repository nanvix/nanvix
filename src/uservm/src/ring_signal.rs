// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::{
    DirectRingSignal,
    vmm::IkcNotifier,
};
use ::log::{
    error,
    trace,
};
use ::std::sync::atomic::{
    AtomicU32,
    Ordering,
};
use ::tokio::sync::mpsc::UnboundedSender;
use ::vmm_sys_util::eventfd::EventFd;

fn futex_wait(word: *mut u32, expected: u32) -> Result<(), std::io::Error> {
    // SAFETY: `word` points to a shared u32 inside the ring mapping and remains valid while the
    // helper threads are alive.
    let ret: libc::c_long = unsafe {
        libc::syscall(
            libc::SYS_futex,
            word,
            libc::FUTEX_WAIT,
            expected,
            core::ptr::null::<libc::timespec>(),
        )
    };
    if ret == 0 {
        return Ok(());
    }

    let err: std::io::Error = std::io::Error::last_os_error();
    match err.raw_os_error() {
        Some(libc::EAGAIN) | Some(libc::EINTR) => Ok(()),
        _ => Err(err),
    }
}

fn futex_wake(word: *mut u32) -> Result<(), std::io::Error> {
    // SAFETY: `word` points to a shared u32 inside the ring mapping and remains valid while the
    // helper threads are alive.
    let ret: libc::c_long =
        unsafe { libc::syscall(libc::SYS_futex, word, libc::FUTEX_WAKE, i32::MAX) };
    if ret < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn signal(word: *mut u32) -> Result<(), std::io::Error> {
    let atomic: &AtomicU32 =
        // SAFETY: the notification word is 4-byte aligned and points into the mapped ring region.
        unsafe { &*(word.cast_const().cast::<AtomicU32>()) };
    atomic.fetch_add(1, Ordering::AcqRel);
    futex_wake(word)
}

fn wait_for_signal(word: *mut u32, observed: &mut u32) -> Result<(), std::io::Error> {
    let atomic: &AtomicU32 =
        // SAFETY: the notification word is 4-byte aligned and points into the mapped ring region.
        unsafe { &*(word.cast_const().cast::<AtomicU32>()) };
    loop {
        let current: u32 = atomic.load(Ordering::Acquire);
        if current != *observed {
            *observed = current;
            return Ok(());
        }
        futex_wait(word, current)?;
    }
}

pub fn run_sq_signal_thread(evtfd: EventFd, signal_word_addr: usize) {
    let signal_word: *mut u32 = signal_word_addr as *mut u32;
    loop {
        if let Err(e) = evtfd.read() {
            error!("run_sq_signal_thread(): doorbell eventfd read failed (error={e:?})");
            break;
        }

        if let Err(e) = signal(signal_word) {
            error!("run_sq_signal_thread(): failed to wake linuxd SQ waiter (error={e:?})");
            break;
        }
    }

    trace!("run_sq_signal_thread(): exiting");
}

pub fn run_cq_signal_thread(signal_word_addr: usize, notifier: IkcNotifier) {
    let signal_word: *mut u32 = signal_word_addr as *mut u32;
    let atomic: &AtomicU32 =
        // SAFETY: the notification word is 4-byte aligned and points into the mapped ring region.
        unsafe { &*(signal_word.cast_const().cast::<AtomicU32>()) };
    let mut observed: u32 = atomic.load(Ordering::Acquire);

    loop {
        if let Err(e) = wait_for_signal(signal_word, &mut observed) {
            error!("run_cq_signal_thread(): failed waiting for CQ signal (error={e:?})");
            break;
        }

        if let Err(e) = notifier.notify_unconditional() {
            error!("run_cq_signal_thread(): failed to inject IKC IRQ (error={e:?})");
            break;
        }
    }

    trace!("run_cq_signal_thread(): exiting");
}

pub fn run_sq_socket_doorbell_thread(
    evtfd: EventFd,
    signal_tx: UnboundedSender<DirectRingSignal>,
) {
    loop {
        if let Err(e) = evtfd.read() {
            error!("run_sq_socket_doorbell_thread(): doorbell eventfd read failed (error={e:?})");
            break;
        }

        if signal_tx.send(DirectRingSignal::SqDoorbell).is_err() {
            error!("run_sq_socket_doorbell_thread(): direct-ring signal channel closed");
            break;
        }
    }

    trace!("run_sq_socket_doorbell_thread(): exiting");
}

pub fn run_cq_socket_doorbell_thread(
    doorbell_rx: ::std::sync::mpsc::Receiver<()>,
    notifier: IkcNotifier,
) {
    while doorbell_rx.recv().is_ok() {
        if let Err(e) = notifier.notify_unconditional() {
            error!(
                "run_cq_socket_doorbell_thread(): failed to inject IKC IRQ (error={e:?})"
            );
            break;
        }
    }

    trace!("run_cq_socket_doorbell_thread(): exiting");
}
