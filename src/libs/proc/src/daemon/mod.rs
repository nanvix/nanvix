// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    identity::ProcessIdentity,
    message,
    ForkSyncAckMessage,
    ForkSyncMessage,
    LookupMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    SignupMessage,
    WaitMessage,
    WaitTarget,
};
use ::alloc::{
    collections::btree_map::BTreeMap,
    string::{
        String,
        ToString,
    },
    vec::Vec,
};
use ::core::ffi::CStr;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    event::{
        Event,
        EventCtrlRequest,
        ProcessCreationInfo,
        SchedulingEvent,
    },
    ipc::{
        Message,
        MessageType,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::{
        Capability,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

/// `WNOHANG` wait option carried in the `options` field of a `Wait` protocol message. A request
/// carrying this flag makes `waitpid()` poll without blocking. This is the daemon's local copy of
/// the wire-protocol flag; its value is POSIX-compatible (matching the `WNOHANG` constant exposed
/// to user space), kept in sync by convention rather than by a shared definition, as this crate
/// does not depend on the user-space API crate.
const WNOHANG: i32 = 1;

///
/// # Description
///
/// Bookkeeping record for a process tracked by the process manager daemon.
///
struct ProcessRecord {
    /// Process name.
    name: String,
    /// Process identity (credentials).
    #[allow(dead_code)]
    identity: Option<ProcessIdentity>,
    /// Process identifier of the parent (`None` for daemons and the init process).
    parent: Option<ProcessIdentifier>,
    /// Process identifiers of the live children.
    children: Vec<ProcessIdentifier>,
    /// Whether the fork-clone of this process has already been dispatched to the filesystem daemon.
    /// Used to acknowledge a fork-sync request regardless of whether it races ahead of the
    /// process-creation event.
    fork_clone_done: bool,
    /// Whether the fork-clone of this process could not be dispatched to the filesystem daemon (the
    /// notification failed to build or send). Used to release a blocked fork-sync waiter with a
    /// failure acknowledgement instead of leaving it deadlocked on a snapshot that will never be
    /// taken.
    fork_clone_failed: bool,
    /// Termination status once the process has terminated and is awaiting reap by `waitpid()`.
    /// `Some(status)` marks a zombie; `None` marks a live (or not-yet-terminated) process.
    zombie: Option<i32>,
}

impl ProcessRecord {
    /// Instantiates a new process record.
    fn new(name: String, parent: Option<ProcessIdentifier>) -> Self {
        Self {
            name,
            identity: None,
            parent,
            children: Vec::new(),
            fork_clone_done: false,
            fork_clone_failed: false,
            zombie: None,
        }
    }
}

///
/// # Description
///
/// Selects which child(ren) a blocked waiter is waiting for.
///
enum WaitSelector {
    /// Any child of the waiter.
    Any,
    /// A specific child of the waiter.
    Pid(ProcessIdentifier),
}

impl WaitSelector {
    /// Returns `true` if `child` matches this selector.
    fn matches(&self, child: ProcessIdentifier) -> bool {
        match self {
            WaitSelector::Any => true,
            WaitSelector::Pid(pid) => *pid == child,
        }
    }
}

///
/// # Description
///
/// A parent process blocked in a `Wait` operation, awaiting a deferred reply.
///
struct BlockedWaiter {
    /// Process identifier of the blocked waiter.
    waiter: ProcessIdentifier,
    /// Children that the waiter is waiting for.
    selector: WaitSelector,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub struct ProcessDaemon {
    // FIXME: auto-signup process on process creation.
    processes: BTreeMap<ProcessIdentifier, ProcessRecord>,
    /// Process identifier of the init process, recorded when the non-daemon boot workload signs
    /// up. Remains `None` when that workload never signs up, in which case the termination handler
    /// decides shutdown from lineage (the process whose parent is the kernel).
    init_proc: Option<ProcessIdentifier>,
    /// Fork-sync requests awaiting the fork-clone dispatch, stored as `(child, parent)` pairs that
    /// map a child to the blocked parent. Populated when a fork-sync request arrives before the
    /// child's process-creation event; drained when that event dispatches the clone. A `Vec` is
    /// used rather than a map because only a handful of fork operations are ever pending
    /// concurrently, so a linear scan is cheaper than the overhead of an ordered map.
    pending_fork_syncs: Vec<(ProcessIdentifier, ProcessIdentifier)>,
    /// Parents currently blocked in a `Wait` operation. A blocking `waitpid()` is parked here and
    /// answered later, when a `ProcessTermination` event for a matching child arrives.
    blocked: Vec<BlockedWaiter>,
    /// Terminations observed before the corresponding process-creation event, stored as
    /// `(pid, status)` pairs. The kernel publishes creation events ahead of termination events in
    /// its main loop, so lineage is usually recorded before the termination arrives. A termination
    /// can still be observed first for two reasons: the creation event may not have been
    /// delivered yet (e.g. the scheduling-event queue was momentarily full and the creation was
    /// requeued), and even when both are already queued the kernel scans the creation and
    /// termination sub-queues round-robin, so it may deliver the termination first. In either case
    /// procd can learn that a child died while the child is still unknown. The status is buffered
    /// here and replayed once the creation event records the child's lineage, so the exit status
    /// is never dropped and a parent blocked in `waitpid()` is always answered. A `Vec` suffices
    /// because only a handful of terminations are ever pending reconciliation at once.
    early_terminations: Vec<(ProcessIdentifier, i32)>,
}

impl ProcessDaemon {
    /// Initializes the process manager daemon.
    pub fn init() -> Result<Self, Error> {
        ::syslog::info!("running process manager daemon...");
        let mypid: ProcessIdentifier = ::sys::kcall::pm::getpid()?;
        assert_eq!(mypid, ProcessIdentifier::PROCD, "process daemon has unexpected pid");

        // Acquire process management capabilities.
        ::syslog::info!("acquiring process management capabilities...");
        ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, true)?;

        // Subscribe to scheduling events. Scheduling events are owned as a single class: a single
        // registration claims ownership of every scheduling event, so this one call subscribes the
        // daemon to both process-termination and process-creation events. The kernel publishes a
        // creation event whenever a process forks a child, which the daemon uses to record the
        // parent/child relationship without the parent having to register the child explicitly.
        ::syslog::info!("subscribing to scheduling events...");
        ::sys::kcall::event::__kcall_evctrl(
            Event::Scheduling(SchedulingEvent::ProcessTermination),
            EventCtrlRequest::Register,
        )?;

        Ok(Self {
            processes: BTreeMap::new(),
            init_proc: None,
            pending_fork_syncs: Vec::new(),
            blocked: Vec::new(),
            early_terminations: Vec::new(),
        })
    }

    /// Runs the process manager daemon.
    /// Returns the exit status of the non-daemon process that triggered shutdown.
    pub fn run(&mut self) -> i32 {
        loop {
            match ::sys::kcall::ipc::__kcall_recv() {
                Ok(message) => {
                    ::syslog::info!("received message from={:?}", { message.source });
                    match message.message_type {
                        MessageType::Exception => unreachable!("should not receive exceptions"),
                        MessageType::Ipc => {
                            if let Err(e) = self.handle_ipc_message(message) {
                                ::syslog::error!("failed to handle IPC message (error={:?})", e);
                            }
                        },
                        MessageType::Interrupt => unreachable!("should not receive interrupts"),
                        MessageType::Ikc => unreachable!("should not receive IKC messages"),
                        MessageType::ProcessTerminationEvent => {
                            match self.handle_process_termination_event(message) {
                                Ok(Some(status)) => return status,
                                Ok(None) => continue,
                                Err(e) => {
                                    ::syslog::error!(
                                        "failed to handle scheduling event (error={:?})",
                                        e
                                    )
                                },
                            }
                        },
                        MessageType::PullResponse => {
                            ::syslog::error!("received unexpected pull response, ignoring");
                            continue;
                        },
                        MessageType::ProcessCreationEvent => {
                            match self.handle_process_creation_event(message) {
                                Ok(Some(status)) => return status,
                                Ok(None) => continue,
                                Err(e) => {
                                    ::syslog::error!(
                                        "failed to handle process creation event (error={:?})",
                                        e
                                    );
                                },
                            }
                        },
                    }
                },
                Err(e) => ::syslog::error!("failed to receive exception message (error={:?})", e),
            }
        }
    }

    /// Handles a process-creation scheduling event published by the kernel. The kernel emits this
    /// event whenever a process forks a child, allowing the daemon to record the parent/child
    /// relationship without the parent having to register the child explicitly.
    fn handle_process_creation_event(&mut self, message: Message) -> Result<Option<i32>, Error> {
        // Deserialize the process-creation information.
        let raw_info: [u8; ::core::mem::size_of::<ProcessCreationInfo>()] = message.payload
            [0..::core::mem::size_of::<ProcessCreationInfo>()]
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, "invalid process creation event payload")
            })?;
        let info: ProcessCreationInfo = ProcessCreationInfo::from_ne_bytes(raw_info);
        let child: ProcessIdentifier = info.pid;
        let parent: ProcessIdentifier = info.parent;

        ::syslog::info!("process created (child={:?}, parent={:?})", child, parent);

        self.record_child_lineage(parent, child);

        // The kernel spawns daemons and the init process directly (parent is the kernel), so
        // there is no parent filesystem state to inherit: skip the fork-clone notification for
        // them. This avoids needless boot-time traffic to the filesystem daemon and a phantom
        // per-process state keyed by the kernel. Only genuine user-space forks (parent is another
        // process) require duplication. The clone is remembered as dispatched only if the
        // notification was actually delivered to the filesystem daemon: marking it done on a failed
        // send would let a later fork-sync request be acknowledged without a snapshot ever having
        // been taken, letting parent and child proceed past unduplicated filesystem state.
        if parent != ProcessIdentifier::KERNEL {
            let clone_dispatched: bool = self.notify_fork_clone(parent, child);
            if let Some(record) = self.processes.get_mut(&child) {
                if clone_dispatched {
                    record.fork_clone_done = true;
                } else {
                    // The fork-clone notification could not be dispatched. Mark the failure so that
                    // a fork-sync waiter (whether already pending below or arriving later) is
                    // released with a failure acknowledgement rather than left blocked forever.
                    record.fork_clone_failed = true;
                }
            }
        }

        // Release a parent (and its child) that is already blocked awaiting fork synchronization.
        // Two conditions must hold before the waiter is acknowledged with success:
        //
        // 1. The waiter must match the kernel-attributed real parent of this child. A pending entry
        //    whose waiter differs was injected by a process that named a `child` that is not
        //    actually its own (the `child` field of a fork-sync request is untrusted): drop it
        //    without acknowledging, so it cannot inject a spurious acknowledgement into a victim's
        //    mailbox or displace the genuine waiter.
        // 2. The fork-clone must have actually been dispatched to the filesystem daemon, tracked by
        //    `fork_clone_done`. If the notification failed to send, the waiter is instead released
        //    with a failure acknowledgement so that `fork()` aborts rather than acknowledged with
        //    success: a success acknowledgement would let parent and child proceed past a filesystem
        //    snapshot that was never taken, mirroring the `fork_clone_done` gating on the fork-sync
        //    fast path in `handle_fork_sync()`.
        if let Some(pos) = self
            .pending_fork_syncs
            .iter()
            .position(|(c, _)| *c == child)
        {
            let (_, waiting_parent) = self.pending_fork_syncs[pos];
            if waiting_parent != parent {
                // Forged waiter: drop it without acknowledging.
                self.pending_fork_syncs.swap_remove(pos);
                ::syslog::warn!(
                    "dropping forged fork-sync (waiter={:?}, child={:?}, real_parent={:?})",
                    waiting_parent,
                    child,
                    parent
                );
            } else if self
                .processes
                .get(&child)
                .map(|record| record.fork_clone_done)
                .unwrap_or(false)
            {
                // Genuine waiter and the fork-clone has been dispatched: acknowledge it.
                self.pending_fork_syncs.swap_remove(pos);
                self.release_fork_sync(waiting_parent, child);
            } else {
                // Genuine waiter but the fork-clone was not dispatched (the notification failed to
                // build or send). Release it with a failure acknowledgement so that `fork()` aborts
                // in both parent and child instead of deadlocking forever on a snapshot that was
                // never taken.
                self.pending_fork_syncs.swap_remove(pos);
                ::syslog::warn!(
                    "fork-clone not dispatched, failing fork-sync waiter (parent={:?}, child={:?})",
                    waiting_parent,
                    child
                );
                self.fail_fork_sync(waiting_parent, child);
            }
        }

        // Replay a termination that was observed before this creation event. The kernel normally
        // publishes creations ahead of terminations, but a termination can still arrive first when
        // the creation event could not be delivered yet (e.g. the scheduling-event queue was
        // momentarily full and the creation was requeued), so procd may have seen this child die
        // while it was still unknown and buffered the exit status. Now that the child's lineage is
        // recorded and its fork-clone has been dispatched to the filesystem daemon above, reclaim
        // its filesystem state (ordered after the fork-clone, so the snapshot is taken before it is
        // torn down) and finalize the termination through the same decision logic as
        // `handle_process_termination_event()`. Routing through the shared helper is essential: a
        // kernel-spawned process (the init process or a daemon) can be buffered too — its `Signup`
        // may not have been processed when its termination arrived — so the buffered termination
        // must trigger init-driven shutdown or daemon deregistration/crash shutdown rather than be
        // auto-reaped as if it were an ordinary forked child. This also answers a parent blocked in
        // `waitpid()` instead of leaving it blocked forever on a child that can never terminate
        // again. A returned shutdown status is propagated to the caller so the daemon can bring the
        // system down.
        if let Some(pos) = self
            .early_terminations
            .iter()
            .position(|(p, _)| *p == child)
        {
            let (_, status) = self.early_terminations.swap_remove(pos);
            ::syslog::info!(
                "reconciling buffered termination (child={:?}, status={:?})",
                child,
                status
            );
            return self.finalize_known_termination(child, status);
        }

        Ok(None)
    }

    /// Handles a process-termination scheduling event.
    ///
    /// Determines whether the terminating process is the boot/init workload (created directly by
    /// the kernel) or a runtime-spawned child, so that only termination of the former triggers
    /// system shutdown. Re-parents any surviving children to the init process and notifies the
    /// filesystem daemon to reclaim the process's per-process state.
    fn handle_process_termination_event(&mut self, message: Message) -> Result<Option<i32>, Error> {
        // Deserialize process identifier.
        let raw_pid_bytes: [u8; 4] = match message.payload[0..4].try_into() {
            Ok(bytes) => bytes,
            Err(_) => {
                let reason: &str = "invalid process termination message payload";
                ::syslog::error!("handle_process_termination_event(): {reason:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        let pid: ProcessIdentifier = ProcessIdentifier::from(i32::from_le_bytes(raw_pid_bytes));

        ::syslog::info!("received scheduling event (pid={:?})", pid);

        // Deserialize process status.
        let status: i32 = i32::from_le_bytes(message.payload[4..8].try_into().unwrap());
        ::syslog::info!("process terminated (pid={:?}, status={:?})", pid, status);

        // The kernel normally publishes a process's creation event before its termination event
        // (the `ProcessCreation` scheduling slot is drained ahead of `ProcessTermination`), so a
        // termination usually arrives with the child's lineage already recorded. It can still
        // arrive while the lineage is unknown in the residual case where the creation event could
        // not be delivered yet (e.g. the scheduling-event queue was momentarily full and the
        // creation was requeued). The same applies when the child sent its `Signup` before its
        // creation event was processed: such a record exists (with its name) yet still has no
        // recorded parent, so re-parenting, reaping, and waking a blocked waiter cannot be resolved
        // correctly yet, and processing the termination immediately would let the later creation
        // event recreate the record with no buffered status to replay. Buffer the `(pid, status)`
        // and defer every side effect — filesystem-state reclamation, re-parenting, reaping, and
        // waking a blocked waiter — until the creation event records the child's lineage and
        // `handle_process_creation_event()` replays it. This guarantees the exit status is never
        // dropped and a parent blocked in `waitpid()` is always answered. The creation event is
        // guaranteed to follow: the kernel buffers it at fork time and the main loop re-publishes
        // it until it is delivered. Deferring the filesystem-exit notification also keeps it
        // ordered after the fork-clone that the creation handler dispatches, so the child's
        // filesystem snapshot is taken before it is reclaimed.
        //
        // A process whose identity is already established without lineage is exempt: the init
        // process (tracked in `init_proc`) and daemons (identified by name) make their termination
        // decisions — shutdown and deregistration — without a recorded parent, so they are handled
        // immediately rather than buffered.
        let lineage_pending: bool = match self.processes.get(&pid) {
            None => true,
            Some(record) => {
                record.parent.is_none()
                    && self.init_proc != Some(pid)
                    && !Self::is_daemon(&record.name)
            },
        };
        if lineage_pending {
            ::syslog::info!(
                "termination observed before creation (pid={:?}, status={:?}) — buffering",
                pid,
                status
            );
            // Replace any stale entry for this pid before recording the new status, preserving an
            // at-most-one-entry-per-pid invariant across pid reuse.
            self.early_terminations.retain(|(p, _)| *p != pid);
            self.early_terminations.push((pid, status));
            return Ok(None);
        }

        // Lineage is known: dispatch the termination through the shared decision logic, which
        // routes init-process and daemon terminations to shutdown/deregistration and ordinary
        // forked children to auto-reaping. The same helper is used by the early-termination replay
        // in `handle_process_creation_event()`, so a buffered termination of a kernel-spawned
        // process (init or a daemon) is handled identically rather than being mistaken for a
        // forked child.
        self.finalize_known_termination(pid, status)
    }

    /// Applies the termination decision for a process `pid` whose lineage is already known, sharing
    /// one code path between the termination handler and the early-termination replay in
    /// `handle_process_creation_event()`. Reclaims the process's filesystem state, drops its stale
    /// fork-sync and blocked-wait bookkeeping, and then routes the termination according to the
    /// process's role: the init process triggers system shutdown (propagating its exit status), a
    /// daemon is deregistered (triggering a crash shutdown only on a non-zero status), and an
    /// ordinary forked child is finalized via [`Self::finalize_forked_child_termination`]. Returns
    /// `Some(status)` when the termination must bring the system down, otherwise `None`.
    fn finalize_known_termination(
        &mut self,
        pid: ProcessIdentifier,
        status: i32,
    ) -> Result<Option<i32>, Error> {
        // Drop any stale fork-sync bookkeeping and notify the filesystem daemon to reclaim the
        // process's per-process state (open file descriptors and working directory). Unlike the
        // fork-clone notification — which is skipped for kernel-spawned processes that have no
        // parent state to inherit — this is sent for every terminating process: daemons and the
        // init process accumulate their own filesystem state lazily as they open files, and that
        // state must be reclaimed too. The notification is a no-op in the filesystem daemon for a
        // process that never registered any state, so the extra message is harmless.
        self.pending_fork_syncs.retain(|(child, _)| *child != pid);
        // Drop any blocked-wait bookkeeping owned by the terminating process. A process that was
        // itself parked in `waitpid()` can never be answered once it is gone, so leaving its entry
        // behind would leak memory and strand a stale waiter.
        self.blocked.retain(|waiter| waiter.waiter != pid);
        self.notify_process_exit(pid);

        // Look up the terminating process in the registry.
        if let Some(record) = self.processes.get(&pid) {
            let name: String = record.name.clone();
            // The boot/init workload is the non-daemon process created directly by the kernel.
            // If it has signed up, its identity is recorded reliably in `init_proc`, so prefer
            // that: only its termination triggers shutdown. Otherwise — most workloads never sign
            // up — fall back to lineage: the process whose recorded parent is the kernel is the
            // init process. Requiring the parent to be the kernel (rather than merely absent)
            // prevents a forked child from spuriously triggering a system-wide shutdown. The
            // well-known daemon PIDs are excluded explicitly because a daemon is also spawned
            // directly by the kernel, and the name-based `is_daemon` check above is unreliable
            // until the daemon signs up: a buffered termination replayed before the daemon's
            // `Signup` was processed has an empty name, so without this exclusion it would match
            // the lineage fallback and spuriously bring the system down. This mirrors the
            // daemon-PID exclusion in `adoptive_init()`.
            let is_init: bool = match self.init_proc {
                Some(init_proc) => init_proc == pid,
                None => {
                    record.parent == Some(ProcessIdentifier::KERNEL)
                        && !matches!(
                            pid,
                            ProcessIdentifier::PROCD
                                | ProcessIdentifier::MEMD
                                | ProcessIdentifier::VFSD
                        )
                },
            };

            // A daemon terminated — not a shutdown trigger (unless it crashed).
            if Self::is_daemon(&name) {
                ::syslog::info!("deregistering daemon (pid={:?}, name={:?})", pid, name);
                self.processes.remove(&pid);
                if status != 0 {
                    ::syslog::error!(
                        "critical daemon {:?} terminated with non-zero status {} — triggering \
                         shutdown",
                        name,
                        status
                    );
                    return Ok(Some(status));
                }
                return Ok(None);
            }

            // The init process terminated — initiate shutdown and propagate its exit status.
            if is_init {
                ::syslog::info!("init process terminated (pid={:?}, status={:?})", pid, status);
                self.processes.remove(&pid);
                self.init_proc = None;
                return Ok(Some(status));
            }

            // A forked child terminated. Finalize it through the shared helper, which auto-reaps
            // its zombie children, re-parents its live children, and either retains it as a reapable
            // zombie (waking a blocked parent) or drops it. The same finalization runs when a
            // child's creation event is observed only after its termination event, so it lives in a
            // shared helper rather than being duplicated here.
            self.finalize_forked_child_termination(pid, status)?;

            return Ok(None);
        }

        // Unreachable in practice: a termination is finalized only once the process's lineage is
        // recorded, so the process is always present in the registry and was handled by one of the
        // branches above. Return without action as a safe guard rather than panicking.
        Ok(None)
    }

    /// Finalizes the termination of a forked child `pid` whose lineage is known. Auto-reaps any of
    /// its own children that are already zombies (only this terminating process could ever have
    /// reaped them, so re-homing them to init — which never calls `waitpid()` — would leak them
    /// until shutdown), re-parents its surviving live children to the init process, then decides
    /// reapability: if a live parent can still reap it, it is retained as a zombie and a parent
    /// already blocked in `waitpid()` is woken; otherwise it is dropped rather than left as an
    /// unreapable zombie. Shared by the termination handler and the early-termination reconciliation
    /// in `handle_process_creation_event()`.
    fn finalize_forked_child_termination(
        &mut self,
        pid: ProcessIdentifier,
        status: i32,
    ) -> Result<(), Error> {
        let parent: Option<ProcessIdentifier> =
            self.processes.get(&pid).and_then(|record| record.parent);

        self.reap_zombie_children(pid);
        self.reparent_children(pid);

        // Determine whether any live process can ever reap this one. Only a parent that is still
        // alive and has not itself terminated can call `waitpid()`, so a process whose parent is
        // unknown, is the kernel (which never waits), is gone, or is already a zombie can never
        // be reaped. Retaining such a process as a zombie would leak it forever — which is what
        // would happen to a grandchild whose parent terminated and no init process exists to adopt
        // it — so auto-reap it instead.
        let reaper: Option<ProcessIdentifier> = parent.filter(|parent| {
            *parent != ProcessIdentifier::KERNEL
                && self
                    .processes
                    .get(parent)
                    .map(|record| record.zombie.is_none())
                    .unwrap_or(false)
        });

        match reaper {
            // A live parent may reap it: retain it as a zombie (clearing its now re-parented
            // children list) and wake the parent if it is already blocked in `waitpid()`, which
            // reaps the zombie immediately. Otherwise the zombie is kept until a future
            // `waitpid()` collects it.
            Some(parent) => {
                if let Some(record) = self.processes.get_mut(&pid) {
                    record.children.clear();
                    record.zombie = Some(status);
                }
                self.wake_waiter(parent, pid, status)?;
            },
            // No live process can ever reap it: drop it rather than leak an unreapable zombie.
            None => {
                self.processes.remove(&pid);
            },
        }

        Ok(())
    }

    /// Returns the process that should adopt orphaned children. Prefers the explicitly tracked init
    /// process recorded at signup; when that workload never signed up (`init_proc` is `None`), falls
    /// back to the init process identified by lineage — the non-daemon process whose parent is the
    /// kernel. Returns `None` only when no such process exists, in which case orphans cannot be
    /// adopted and their bookkeeping is dropped on their own termination.
    ///
    /// The well-known daemon PIDs are excluded explicitly rather than relying solely on the
    /// name-based `is_daemon` check, because that check is unreliable here: a daemon's record name
    /// is only populated when it signs up, and `procd` itself never signs up. `procd` observes its
    /// own kernel-spawned creation event, so it holds a record whose parent is the kernel and whose
    /// name is empty — which would otherwise make it the lowest-PID match and let it adopt orphans
    /// it can never reap (`procd` does not call `waitpid()`), leaking them until shutdown.
    fn adoptive_init(&self) -> Option<ProcessIdentifier> {
        if let Some(init_proc) = self.init_proc {
            return Some(init_proc);
        }

        self.processes
            .iter()
            .find(|(pid, record)| {
                record.parent == Some(ProcessIdentifier::KERNEL)
                    && record.zombie.is_none()
                    && !matches!(
                        **pid,
                        ProcessIdentifier::PROCD
                            | ProcessIdentifier::MEMD
                            | ProcessIdentifier::VFSD
                    )
                    && !Self::is_daemon(&record.name)
            })
            .map(|(pid, _)| *pid)
    }

    /// Re-parents the surviving children of `pid` to the init process. The adoptive parent is the
    /// init process recorded at signup, or — when that workload never signed up — the init process
    /// identified by lineage, so surviving children are re-homed consistently and no stale parent
    /// pointers or child lists are left behind.
    fn reparent_children(&mut self, pid: ProcessIdentifier) {
        let init_proc: ProcessIdentifier = match self.adoptive_init() {
            Some(init_proc) => init_proc,
            None => return,
        };

        // Nothing to do if the terminating process is the init process itself.
        if init_proc == pid {
            return;
        }

        let children: Vec<ProcessIdentifier> = match self.processes.get(&pid) {
            Some(record) => record.children.clone(),
            None => return,
        };

        for child in children {
            if let Some(record) = self.processes.get_mut(&child) {
                record.parent = Some(init_proc);
            }
            if let Some(record) = self.processes.get_mut(&init_proc) {
                if !record.children.contains(&child) {
                    record.children.push(child);
                }
            }
        }
    }

    /// Auto-reaps the zombie children of a terminating process `pid`. A zombie can only be reaped by
    /// its parent, but `pid` is itself terminating and will never call `waitpid()`, so its zombie
    /// children would otherwise linger until shutdown (re-homing them to init does not help, as init
    /// never reaps). A process clears its own children list when it becomes a zombie, so a zombie
    /// has no descendants and dropping it orphans nothing.
    fn reap_zombie_children(&mut self, pid: ProcessIdentifier) {
        let children: Vec<ProcessIdentifier> = match self.processes.get(&pid) {
            Some(record) => record.children.clone(),
            None => return,
        };

        for child in children {
            let is_zombie: bool = self
                .processes
                .get(&child)
                .map(|record| record.zombie.is_some())
                .unwrap_or(false);
            if is_zombie {
                self.reap(pid, child);
            }
        }
    }

    /// Handles a `Wait` request from `caller` selecting `(pid, options)`. Returns
    /// `Ok(Some(reply))` for an immediate reply, or `Ok(None)` when the waiter blocks (the reply is
    /// then produced later by the process-termination handler).
    fn handle_wait(
        &mut self,
        caller: ProcessIdentifier,
        message: WaitMessage,
    ) -> Result<Option<Message>, Error> {
        let options: i32 = message.options;

        // Resolve the strongly-typed selector. The wire encoding folds the POSIX process-group
        // selectors (`pid == 0` and `pid < -1`) into `WaitTarget::Any` because Nanvix has no
        // process groups yet (see the `waitpid()` limitations). Since only parent/child lineage is
        // tracked, every eligible child is already a child of the caller, so this is a harmless
        // superset.
        let selector: WaitSelector = match message.target() {
            WaitTarget::Any => WaitSelector::Any,
            WaitTarget::Pid(pid) => WaitSelector::Pid(pid),
        };

        // Enumerate the eligible children of the caller.
        let children: Vec<ProcessIdentifier> = match self.processes.get(&caller) {
            Some(record) => record
                .children
                .iter()
                .copied()
                .filter(|child| selector.matches(*child))
                .collect(),
            None => Vec::new(),
        };

        // The caller has no eligible children: report `ECHILD`.
        if children.is_empty() {
            let reply: Message = message::wait_response(
                caller,
                ProcessIdentifier::from(0),
                0,
                ErrorCode::NoChildProcess.get(),
            )?;
            return Ok(Some(reply));
        }

        // Look for a ready zombie among the eligible children (lowest pid for determinism).
        let mut zombies: Vec<(ProcessIdentifier, i32)> = children
            .iter()
            .filter_map(|child| {
                self.processes
                    .get(child)
                    .and_then(|record| record.zombie.map(|status| (*child, status)))
            })
            .collect();
        zombies.sort_by_key(|(child, _)| *child);

        if let Some((child, status)) = zombies.first().copied() {
            // Reap the zombie and reply immediately.
            self.reap(caller, child);
            let reply: Message = message::wait_response(caller, child, status, 0)?;
            return Ok(Some(reply));
        }

        // No zombie is ready.
        if options & WNOHANG != 0 {
            // Non-blocking poll: report that no child is ready (child pid of zero, no error).
            let reply: Message = message::wait_response(caller, ProcessIdentifier::from(0), 0, 0)?;
            return Ok(Some(reply));
        }

        // Block the waiter; the reply is deferred until a matching child terminates.
        // Keep at most one blocked wait per waiter to avoid unbounded growth / stale deferred replies.
        self.blocked.retain(|w| w.waiter != caller);
        self.blocked.push(BlockedWaiter {
            waiter: caller,
            selector,
        });

        Ok(None)
    }

    /// Wakes a parent blocked in `waitpid()` that is waiting for `child` (a child of `parent`) and
    /// reaps the zombie. If no waiter is blocked, the zombie is left in place until a future
    /// `waitpid()` reaps it.
    fn wake_waiter(
        &mut self,
        parent: ProcessIdentifier,
        child: ProcessIdentifier,
        status: i32,
    ) -> Result<(), Error> {
        if let Some(index) = self
            .blocked
            .iter()
            .position(|waiter| waiter.waiter == parent && waiter.selector.matches(child))
        {
            let waiter_pid: ProcessIdentifier = self.blocked[index].waiter;
            let reply: Message = message::wait_response(waiter_pid, child, status, 0)?;
            ::sys::kcall::ipc::__kcall_send(&reply)?;

            self.blocked.swap_remove(index);
            self.reap(parent, child);
        }

        Ok(())
    }

    /// Reaps a zombie `child` of `parent`, removing it from the registry and from the parent's list
    /// of children.
    fn reap(&mut self, parent: ProcessIdentifier, child: ProcessIdentifier) {
        if let Some(record) = self.processes.get_mut(&parent) {
            record.children.retain(|c| *c != child);
        }
        self.processes.remove(&child);
    }

    /// Returns `true` if `name` belongs to a system daemon that should not trigger shutdown.
    fn is_daemon(name: &str) -> bool {
        ::config::daemons::DAEMON_NAMES.contains(&name)
    }

    fn handle_ipc_message(&mut self, message: Message) -> Result<(), Error> {
        let destination: ProcessIdentifier = match { message.source }.as_id() {
            Ok(pid) => pid,
            Err(tid) => {
                let reason: &str = "invalid IPC message source";
                ::syslog::error!("handle_ipc_message(): {reason:?} (tid={:?})", tid);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        let message: SystemMessage = SystemMessage::from_bytes(message.payload)?;

        ::syslog::info!("received system message (header={:?})", message.header);

        // Parse message.
        if let SystemMessageHeader::ProcessManagement = message.header {
            let message: ProcessManagementMessage =
                ProcessManagementMessage::from_bytes(message.payload)?;

            // Parse operation.
            match message.header {
                ProcessManagementMessageHeader::Signup => {
                    let message: SignupMessage = SignupMessage::from_bytes(message.payload);
                    let message: Message = self.handle_signup(destination, message)?;
                    ::sys::kcall::ipc::__kcall_send(&message)?;
                },
                ProcessManagementMessageHeader::Lookup => {
                    let message: LookupMessage = LookupMessage::from_bytes(message.payload);
                    let message: Message = self.handle_lookup(destination, message)?;
                    ::sys::kcall::ipc::__kcall_send(&message)?;
                },
                ProcessManagementMessageHeader::ForkSync => {
                    let message: ForkSyncMessage = ForkSyncMessage::from_bytes(message.payload);
                    self.handle_fork_sync(destination, message.child);
                },
                ProcessManagementMessageHeader::Wait => {
                    let message: WaitMessage = WaitMessage::from_bytes(message.payload);
                    // The reply is deferred (no send here) when the waiter blocks; it is produced
                    // later by the process-termination handler.
                    if let Some(reply) = self.handle_wait(destination, message)? {
                        ::sys::kcall::ipc::__kcall_send(&reply)?;
                    }
                },
                // Ignore all other messages.
                _ => {},
            }
        }

        Ok(())
    }

    // Handles a signup message.
    fn handle_signup(
        &mut self,
        destination: ProcessIdentifier,
        message: SignupMessage,
    ) -> Result<Message, Error> {
        let pid: ProcessIdentifier = message.pid;
        match CStr::from_bytes_until_nul(&message.name) {
            Ok(cstr) => match cstr.to_str() {
                Ok(name) => {
                    let s: String = name.to_string();

                    if s == ::config::daemons::MEMD_NAME {
                        ::syslog::info!("signup memory daemon");
                    } else {
                        ::syslog::info!("signup other process = {:?}", name);
                    }

                    ::syslog::info!("signing up process (pid={:?}, name={:?})", pid, s.as_bytes());
                    let is_daemon: bool = Self::is_daemon(&s);
                    // Preserve any existing lineage (parent/children) that may have been recorded
                    // by a process-creation event before this signup. Only update the process name.
                    match self.processes.get_mut(&pid) {
                        Some(record) => {
                            record.name = s;
                        },
                        None => {
                            self.processes.insert(pid, ProcessRecord::new(s, None));
                        },
                    }
                    // The boot/init workload is the non-daemon process spawned directly by the
                    // kernel. Its name is known reliably here (unlike at process-creation time), so
                    // record it as the init process. This gives the termination handler an explicit
                    // identity to match, rather than relying on the parent==kernel lineage fallback.
                    //
                    // Only record the first such process and never overwrite an init identity that
                    // is already known: a later non-daemon signup (e.g. a forked child that signs
                    // up before its process-creation event is observed, when its parent is not yet
                    // recorded) must not displace the real init PID, or the termination handler
                    // would make incorrect shutdown decisions.
                    if !is_daemon && self.init_proc.is_none() {
                        let parented_by_kernel: bool = self
                            .processes
                            .get(&pid)
                            .map(|record| {
                                record.parent.is_none()
                                    || record.parent == Some(ProcessIdentifier::KERNEL)
                            })
                            .unwrap_or(true);
                        if parented_by_kernel {
                            self.init_proc = Some(pid);
                        }
                    }
                    message::signup_response(destination, pid, 0)
                },
                Err(_) => {
                    message::signup_response(destination, pid, ErrorCode::InvalidArgument.get())
                },
            },
            Err(_) => message::signup_response(destination, pid, ErrorCode::InvalidArgument.get()),
        }
    }

    /// Records the parent/child relationship for a newly created `child` of `parent`, updating the
    /// process registry.
    fn record_child_lineage(&mut self, parent: ProcessIdentifier, child: ProcessIdentifier) {
        ::syslog::info!("registering child (child={:?}, parent={:?})", child, parent);
        // Ensure that a record exists for the parent. When the parent is the kernel, this inserts a
        // placeholder record (empty name) for it: it is intentionally benign — it never matches a
        // name lookup, and `shutdown()` drops all non-daemon records, so the kernel placeholder is
        // never broadcast a shutdown message.
        self.processes
            .entry(parent)
            .or_insert_with(|| ProcessRecord::new(String::new(), None));

        // A forked child inherits the name of its parent.
        let child_name: String = self
            .processes
            .get(&parent)
            .map(|record| record.name.clone())
            .unwrap_or_default();

        // Insert or update the child record.
        match self.processes.get_mut(&child) {
            Some(record) => {
                record.parent = Some(parent);
            },
            None => {
                self.processes
                    .insert(child, ProcessRecord::new(child_name, Some(parent)));
            },
        }

        // A process that signs up before its process-creation event is observed has no recorded
        // parent yet, so a non-daemon signup may have tentatively recorded it as the init process.
        // If this creation event now reveals a genuine (non-kernel) parent, the process was in fact
        // forked and is not init: clear the tentative identity so it is not later misclassified as
        // init on termination and made to trigger a spurious system-wide shutdown.
        if parent != ProcessIdentifier::KERNEL && self.init_proc == Some(child) {
            ::syslog::info!(
                "clearing tentative init identity for forked child (child={:?}, parent={:?})",
                child,
                parent
            );
            self.init_proc = None;
        }

        // Append the child to the parent's list of children.
        if let Some(record) = self.processes.get_mut(&parent) {
            if !record.children.contains(&child) {
                record.children.push(child);
            }
        }
    }

    /// Notifies the filesystem daemon that the filesystem resources of `parent` must be cloned onto
    /// the freshly forked `child`. The notification is sent in a fire-and-forget fashion: the
    /// process manager daemon does not block waiting for an acknowledgement, so that it never risks
    /// swallowing an unrelated message (such as a process-termination event) on its receive path.
    ///
    /// Returns `true` if the fork-clone request was successfully handed to the kernel for delivery
    /// to the filesystem daemon, and `false` if it could not be built or sent. The caller uses this
    /// to decide whether to mark the child's fork-clone as dispatched: a fork-sync request must not
    /// be acknowledged on the basis of a clone that was never delivered.
    fn notify_fork_clone(&self, parent: ProcessIdentifier, child: ProcessIdentifier) -> bool {
        match message::fork_clone_request(parent, child) {
            Ok(request) => match ::sys::kcall::ipc::__kcall_send(&request) {
                Ok(()) => true,
                Err(e) => {
                    ::syslog::warn!(
                        "notify_fork_clone: failed to notify vfsd to clone resources \
                         (parent={:?}, child={:?}, error={:?})",
                        parent,
                        child,
                        e
                    );
                    false
                },
            },
            Err(e) => {
                ::syslog::warn!(
                    "notify_fork_clone: failed to build fork-clone request (parent={:?}, \
                     child={:?}, error={:?})",
                    parent,
                    child,
                    e
                );
                false
            },
        }
    }

    /// Handles a fork-sync request from a freshly forked `parent` awaiting confirmation that the
    /// filesystem state of `child` has been duplicated.
    ///
    /// If the child's fork-clone has already been dispatched to the filesystem daemon, the parent
    /// and child are released immediately with a success acknowledgement. If the fork-clone could
    /// not be dispatched, they are released with a failure acknowledgement so that `fork()` aborts
    /// instead of hanging. Otherwise the request is recorded and resolved once the child's
    /// process-creation event dispatches (or fails to dispatch) the clone. A success release is
    /// always ordered after the fork-clone on the filesystem daemon's receive path, so neither
    /// process can race ahead of the snapshot.
    fn handle_fork_sync(&mut self, parent: ProcessIdentifier, child: ProcessIdentifier) {
        ::syslog::info!("fork-sync request (parent={:?}, child={:?})", parent, child);

        // The request's source (`parent`) is attributed by the kernel and therefore trustworthy,
        // but the named `child` is taken from the untrusted request payload. When the child's
        // lineage is already known, reject the request unless the requester is the recorded parent
        // of `child`. Honoring a request whose `child` is not actually the requester's child would
        // let a malicious process inject a spurious fork-sync acknowledgement into an arbitrary
        // victim's mailbox. Requests that arrive before the child's process-creation event (lineage
        // not yet known) are validated later, when that event is processed.
        if let Some(record) = self.processes.get(&child) {
            if record.parent != Some(parent) {
                ::syslog::warn!(
                    "rejecting forged fork-sync (requester={:?}, child={:?}, real_parent={:?})",
                    parent,
                    child,
                    record.parent
                );
                return;
            }
        }

        let (cloned, failed): (bool, bool) = self
            .processes
            .get(&child)
            .map(|record| (record.fork_clone_done, record.fork_clone_failed))
            .unwrap_or((false, false));
        if cloned {
            self.release_fork_sync(parent, child);
        } else if failed {
            // The fork-clone could not be dispatched: abort the fork rather than block forever.
            ::syslog::warn!(
                "fork-clone not dispatched, failing fork-sync request (parent={:?}, child={:?})",
                parent,
                child
            );
            self.fail_fork_sync(parent, child);
        } else {
            // Replace any stale entry for this child before recording the new waiter, preserving
            // the at-most-one-waiter-per-child invariant that the map previously guaranteed.
            self.pending_fork_syncs.retain(|(c, _)| *c != child);
            self.pending_fork_syncs.push((child, parent));
        }
    }

    /// Releases a parent and its freshly forked child that are blocked awaiting fork
    /// synchronization, by acknowledging both with success. The fork-clone has already been
    /// dispatched to the filesystem daemon, so these acknowledgements are necessarily ordered after
    /// it.
    fn release_fork_sync(&self, parent: ProcessIdentifier, child: ProcessIdentifier) {
        self.send_fork_sync_ack(parent, child, ForkSyncAckMessage::STATUS_SUCCESS);
    }

    /// Releases a parent and its freshly forked child that are blocked awaiting fork
    /// synchronization, by acknowledging both with a failure status. Used when the fork-clone could
    /// not be dispatched to the filesystem daemon, so that `fork()` aborts in both processes instead
    /// of deadlocking on a snapshot that will never be taken.
    fn fail_fork_sync(&self, parent: ProcessIdentifier, child: ProcessIdentifier) {
        self.send_fork_sync_ack(parent, child, ErrorCode::TryAgain.get());
    }

    /// Sends a fork-sync acknowledgement carrying `status` to both the `parent` and the `child`.
    fn send_fork_sync_ack(&self, parent: ProcessIdentifier, child: ProcessIdentifier, status: i32) {
        for pid in [parent, child] {
            match message::fork_sync_ack(pid, status) {
                Ok(ack) => {
                    if let Err(e) = ::sys::kcall::ipc::__kcall_send(&ack) {
                        ::syslog::warn!(
                            "send_fork_sync_ack: failed to acknowledge (pid={:?}, status={:?}, \
                             error={:?})",
                            pid,
                            status,
                            e
                        );
                    }
                },
                Err(e) => {
                    ::syslog::warn!(
                        "send_fork_sync_ack: failed to build acknowledgement (pid={:?}, \
                         status={:?}, error={:?})",
                        pid,
                        status,
                        e
                    );
                },
            }
        }
    }

    /// Notifies the filesystem daemon that `pid` has terminated, so that it can reclaim the
    /// process's per-process state (open file descriptors and current working directory). Sent in a
    /// fire-and-forget fashion: the process manager daemon does not wait for an acknowledgement.
    fn notify_process_exit(&self, pid: ProcessIdentifier) {
        match message::process_exit_request(pid) {
            Ok(request) => {
                if let Err(e) = ::sys::kcall::ipc::__kcall_send(&request) {
                    ::syslog::warn!(
                        "notify_process_exit: failed to notify vfsd (pid={:?}, error={:?})",
                        pid,
                        e
                    );
                }
            },
            Err(e) => {
                ::syslog::warn!(
                    "notify_process_exit: failed to build process-exit request (pid={:?}, \
                     error={:?})",
                    pid,
                    e
                );
            },
        }
    }

    // Handles a lookup message.
    pub fn handle_lookup(
        &self,
        destination: ProcessIdentifier,
        message: LookupMessage,
    ) -> Result<Message, Error> {
        let name: &str = match CStr::from_bytes_until_nul(&message.name) {
            Ok(name) => match name.to_str() {
                Ok(s) => s,
                Err(_) => {
                    let message: Message = message::lookup_response(
                        destination,
                        ProcessIdentifier::from(i32::MAX),
                        ErrorCode::InvalidArgument.get(),
                    )?;
                    return Ok(message);
                },
            },
            Err(_) => {
                let message: Message = message::lookup_response(
                    destination,
                    ProcessIdentifier::from(i32::MAX),
                    ErrorCode::InvalidArgument.get(),
                )?;
                return Ok(message);
            },
        };

        // Search the registry for a live process whose name matches the requested name. Zombies are
        // skipped: a terminated process awaiting reap is not a valid lookup target, and a forked
        // zombie still carries its parent's inherited name, which would otherwise alias the parent.
        for (pid, record) in self.processes.iter() {
            if record.zombie.is_some() {
                continue;
            }

            ::syslog::info!("looking up process (name={:?}, pname={:?})", name, record.name);

            if record.name == name {
                let message: Message = message::lookup_response(destination, *pid, 0)?;
                return Ok(message);
            }
        }
        let message: Message = message::lookup_response(
            destination,
            ProcessIdentifier::from(i32::MAX),
            ErrorCode::NoSuchEntry.get(),
        )?;

        Ok(message)
    }

    pub fn shutdown(&mut self) {
        ::syslog::info!("shutting down process manager daemon...");

        // Drop bookkeeping for forked children: only live daemons need to be shut down cleanly,
        // and dead processes will never produce further termination events. Any parent still
        // blocked in `waitpid()` is torn down with the VM, so its blocked-waiter entry is dropped.
        self.blocked.clear();
        self.processes
            .retain(|_pid, record| Self::is_daemon(&record.name));

        for (pid, record) in self.processes.iter() {
            ::syslog::info!("shutting down process (pid={:?}, name={:?})", pid, record.name);
            let message: Message =
                message::shutdown_request(*pid, 0).expect("failed to broadcast shutdown message");
            ::sys::kcall::ipc::__kcall_send(&message)
                .expect("failed to broadcast shutdown message");
        }

        // Wait for memory daemon to terminate.
        while !self.processes.is_empty() {
            match ::sys::kcall::ipc::__kcall_recv() {
                Ok(message) => {
                    if message.message_type == MessageType::ProcessTerminationEvent {
                        // Deserialize process identifier.
                        let pid: ProcessIdentifier = ProcessIdentifier::from(i32::from_le_bytes(
                            message.payload[0..4].try_into().unwrap(),
                        ));

                        // Deserialize process status.
                        let status: i32 =
                            i32::from_le_bytes(message.payload[4..8].try_into().unwrap());

                        // De-register process.
                        if let Some(record) = self.processes.remove(&pid) {
                            ::syslog::info!(
                                "process terminated (name={:?}, pid={:?}, status={:?})",
                                record.name,
                                pid,
                                status
                            );
                        } else {
                            ::syslog::info!(
                                "unknown process terminated (pid={:?}, status={:?})",
                                pid,
                                status
                            );
                        }
                    }
                },
                Err(e) => ::syslog::error!("failed to receive exception message (error={:?})", e),
            }
        }
    }
}

impl Drop for ProcessDaemon {
    fn drop(&mut self) {
        // Unsubscribe from scheduling events. Scheduling events are owned as a single class, so a
        // single unregistration relinquishes ownership of every scheduling event (mirroring the
        // single registration performed in `init()`).
        ::syslog::info!("unsubscribing from scheduling events...");
        if let Err(e) = ::sys::kcall::event::__kcall_evctrl(
            Event::Scheduling(SchedulingEvent::ProcessTermination),
            EventCtrlRequest::Unregister,
        ) {
            ::syslog::error!("failed to unsubscribe from scheduling events (error={:?})", e);
        }

        ::syslog::info!("shutting down process manager daemon...");
        if let Err(e) = ::sys::kcall::pm::__kcall_capctl(Capability::ProcessManagement, false) {
            ::syslog::error!("failed to release process management capabilities (error={:?})", e);
        }
    }
}
