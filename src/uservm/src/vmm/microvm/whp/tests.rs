// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::sync::atomic::AtomicUsize;

#[test]
fn run_pending_guard_tracks_vcpu_entry() {
    let run_state: RunState = RunState::new();
    {
        let _guard: RunPendingGuard<'_> = RunPendingGuard::new(&run_state);
        assert!(run_state.is_pending());
    }
    assert!(!run_state.is_pending());
}

#[test]
fn cancellation_is_skipped_outside_vcpu_entry() {
    let run_state: RunState = RunState::new();
    let mut attempts: usize = 0;

    let cancelled: bool = run_state.cancel_if_pending(|| attempts += 1);

    assert!(!cancelled);
    assert_eq!(attempts, 0);
}

#[test]
fn cancellation_runs_under_cancel_gate() {
    // The crash fixed here came from cancellation overlapping snapshot/teardown. This locks in
    // the invariant that prevents it: the cancel closure runs while `cancel_gate` is held, so
    // it is mutually exclusive with `finish_run`. A cross-thread `try_lock` on the held gate
    // must fail for the whole closure.
    let run_state: Arc<RunState> = Arc::new(RunState::new());
    run_state.begin_run();

    let mut gate_held_during_cancel: bool = false;
    run_state.cancel_if_pending(|| {
        let probe: Arc<RunState> = run_state.clone();
        gate_held_during_cancel = std::thread::spawn(move || probe.cancel_gate.try_lock().is_err())
            .join()
            .expect("prober thread panicked");
    });

    assert!(gate_held_during_cancel, "cancel closure must run while holding cancel_gate");
}

#[test]
fn cancellation_retries_until_run_finishes() {
    let run_state: Arc<RunState> = Arc::new(RunState::new());
    let attempts: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let guard: RunPendingGuard<'_> = RunPendingGuard::new(run_state.as_ref());

    let worker_state: Arc<RunState> = run_state.clone();
    let worker_attempts: Arc<AtomicUsize> = attempts.clone();
    let worker: std::thread::JoinHandle<()> = std::thread::spawn(move || {
        cancel_while_run_pending(worker_state.as_ref(), || {
            worker_attempts.fetch_add(1, Ordering::SeqCst);
        });
    });

    while attempts.load(Ordering::SeqCst) < 2 {
        std::thread::yield_now();
    }

    // Ends the run through `cancel_gate`, exactly as the VMM loop does.
    drop(guard);
    worker.join().expect("cancel worker panicked");

    assert!(attempts.load(Ordering::SeqCst) >= 2);
    assert!(!run_state.is_pending());
}
