// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    identity::ProcessIdentity,
    message,
    ExecAckMessage,
    ForkCloneAckMessage,
    ForkSyncAckMessage,
    ForkSyncMessage,
    JobControlOp,
    JobControlRequest,
    KillMessage,
    LookupMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    SignupMessage,
    TerminalAccessMessage,
    TerminalSignalMessage,
    WaitCancelMessage,
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
        ProcessRole,
        ProcessTerminationInfo,
        SchedulingEvent,
    },
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
        RequestIdentifier,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::{
        Capability,
        GroupIdentifier,
        ProcessIdentifier,
        ThreadIdentifier,
        UserIdentifier,
        SIGCHLD,
        SIGTTIN,
        SIGTTOU,
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

/// Maximum number of wait requests that may be blocked concurrently.
const MAX_BLOCKED_WAITERS: usize = ::config::kernel::MAX_THREADS * 4;

/// Maximum number of fork synchronizations that may await process creation or clone completion.
const MAX_PENDING_FORK_SYNCS: usize = ::config::kernel::MAX_PROCESSES;

///
/// # Description
///
/// Bookkeeping record for a process tracked by the process manager daemon.
///
struct ProcessRecord {
    /// Process name.
    name: String,
    /// Process identity (credentials).
    identity: Option<ProcessIdentity>,
    /// Process identifier of the parent (`None` for daemons and the init process).
    parent: Option<ProcessIdentifier>,
    /// Process identifiers of the live children.
    children: Vec<ProcessIdentifier>,
    /// Whether the fork-clone of this process has been acknowledged by the filesystem daemon, i.e.
    /// the parent's filesystem state has actually been duplicated onto this child. Used to release a
    /// fork-sync request regardless of whether it races ahead of the process-creation event or of
    /// the clone acknowledgement.
    fork_clone_done: bool,
    /// Whether the fork-clone of this process failed: either it could not be dispatched to the
    /// filesystem daemon (the notification failed to build or send) or the filesystem daemon
    /// acknowledged that it could not take the snapshot. Used to release a blocked fork-sync waiter
    /// with a failure acknowledgement instead of leaving it deadlocked on a snapshot that will never
    /// be taken.
    fork_clone_failed: bool,
    /// Identifier of the fork-clone request awaiting acknowledgement from the filesystem daemon.
    fork_clone_request_id: Option<RequestIdentifier>,
    /// Termination status once the process has terminated and is awaiting reap by `waitpid()`.
    /// `Some(status)` marks a zombie; `None` marks a live (or not-yet-terminated) process.
    zombie: Option<i32>,
    /// Session identifier: the process identifier of the leader of this process's session. A process
    /// born directly from the kernel (init and the daemons) starts in a session of its own; a forked
    /// child inherits its parent's session; `setsid()` starts a new one led by the caller.
    sid: ProcessIdentifier,
    /// Process-group identifier: the process identifier of the leader of this process's process
    /// group. Inherited from the parent on fork and changed by `setpgid()`/`setsid()`. Used for
    /// signal-to-process-group delivery and foreground/background terminal arbitration.
    pgid: ProcessIdentifier,
}

///
/// # Description
///
/// Routing metadata retained from a request until its response is sent.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResponseContext {
    /// Exact process and thread that issued the request.
    receiver: MessageReceiver,
    /// Identifier that correlates the response with the request.
    request_id: RequestIdentifier,
}

impl ResponseContext {
    /// Creates response routing metadata from a request sender and identifier.
    fn new(sender: MessageSender, request_id: RequestIdentifier) -> Self {
        Self {
            receiver: MessageReceiver::new(sender.pid, sender.tid),
            request_id,
        }
    }

    /// Applies this context to a response message.
    fn prepare_response(self, response: &mut Message) {
        response.destination = self.receiver;
        self.request_id.write_to(response);
    }

    /// Sends a response to the exact requesting thread with the matching request identifier.
    fn send(self, mut response: Message) -> Result<(), Error> {
        self.prepare_response(&mut response);
        ::sys::kcall::ipc::__kcall_send(&response)
    }
}

///
/// # Description
///
/// A fork-sync request awaiting completion of the child's filesystem clone.
///
#[derive(Clone, Copy)]
struct PendingForkSync {
    /// Process identifier of the freshly forked child.
    child: ProcessIdentifier,
    /// Process identifier of the parent waiting for the clone.
    parent: ProcessIdentifier,
    /// Routing metadata for the parent and child acknowledgements.
    response_context: ResponseContext,
}

///
/// # Description
///
/// An exec request awaiting completion of close-on-exec processing.
///
#[derive(Clone, Copy)]
struct PendingExec {
    /// Process held at the exec synchronization barrier.
    process: ProcessIdentifier,
    /// Identifier of the close-on-exec request awaiting acknowledgement from the filesystem daemon.
    request_id: RequestIdentifier,
    /// Routing metadata for the deferred acknowledgement.
    response_context: ResponseContext,
}

impl ProcessRecord {
    /// Instantiates a new process record. The process starts as the leader of its own session and
    /// process group (`sid == pgid == pid`); callers that know a parent override these to inherit the
    /// parent's session and group.
    fn new(pid: ProcessIdentifier, name: String, parent: Option<ProcessIdentifier>) -> Self {
        Self {
            name,
            identity: Some(ProcessIdentity::new(UserIdentifier::ROOT, GroupIdentifier::ROOT)),
            parent,
            children: Vec::new(),
            fork_clone_done: false,
            fork_clone_failed: false,
            fork_clone_request_id: None,
            zombie: None,
            sid: pid,
            pgid: pid,
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
    /// Routing metadata for the deferred response.
    response_context: ResponseContext,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub struct ProcessDaemon {
    // FIXME: auto-signup process on process creation.
    processes: BTreeMap<ProcessIdentifier, ProcessRecord>,
    /// Process identifier of the init process, recorded from the role the kernel carries in the
    /// process-creation event. Used only to re-parent orphaned children; the shutdown decision is
    /// made authoritatively from the role carried in the termination event, not from this field.
    init_proc: Option<ProcessIdentifier>,
    /// Fork-sync requests awaiting the filesystem daemon's acknowledgement that the child's
    /// fork-clone snapshot has been taken. Populated when a fork-sync request cannot be answered
    /// immediately -- either the child's process-creation event has not yet been observed, or the
    /// clone has been dispatched but not yet acknowledged; drained when the filesystem daemon
    /// acknowledges the clone (releasing or failing the waiter) or when the clone could not be
    /// dispatched. A `Vec` is used rather than a map because only a handful of fork operations are
    /// ever pending concurrently, so a linear scan is cheaper than the overhead of an ordered map.
    pending_fork_syncs: Vec<PendingForkSync>,
    /// Processes held at the exec synchronization barrier, awaiting the filesystem daemon's
    /// acknowledgement that close-on-exec has been applied to their inherited descriptor table.
    /// Populated when a freshly `exec`'d process requests the barrier and the close-on-exec
    /// notification is dispatched to the filesystem daemon; drained when that daemon acknowledges,
    /// at which point the process is released. A `Vec` is used rather than a map because only a
    /// handful of exec operations are ever pending concurrently, so a linear scan is cheaper than
    /// the overhead of an ordered map.
    pending_execs: Vec<PendingExec>,
    /// Next identifier for an asynchronous request sent by this daemon's event-loop thread.
    next_request_id: u32,
    /// Parents currently blocked in a `Wait` operation. A blocking `waitpid()` is parked here and
    /// answered later, when a `ProcessTermination` event for a matching child arrives.
    blocked: Vec<BlockedWaiter>,
    /// Foreground process group of the controlling terminal (the console), set by `tcsetpgrp()` and
    /// reported by `tcgetpgrp()`. Terminal-generated signals (`^C`/`^Z`) are delivered to this group,
    /// and console access by a process outside it is a background access that raises `SIGTTIN` or
    /// `SIGTTOU`. `None` until a foreground group is established.
    foreground_pgrp: Option<ProcessIdentifier>,
}

impl ProcessDaemon {
    /// Allocates an identifier that is not used by another asynchronous daemon request.
    fn allocate_request_id(&mut self) -> RequestIdentifier {
        loop {
            let request_id: RequestIdentifier = RequestIdentifier::from_raw(self.next_request_id);
            self.next_request_id = self.next_request_id.wrapping_add(1);
            if self.next_request_id == RequestIdentifier::NONE.raw() {
                self.next_request_id = 1;
            }

            let fork_request_active: bool = self
                .processes
                .values()
                .any(|record| record.fork_clone_request_id == Some(request_id));
            let exec_request_active: bool = self
                .pending_execs
                .iter()
                .any(|pending| pending.request_id == request_id);
            if !fork_request_active && !exec_request_active {
                return request_id;
            }
        }
    }

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
            pending_execs: Vec::new(),
            next_request_id: 1,
            blocked: Vec::new(),
            foreground_pgrp: None,
        })
    }

    /// Runs the process manager daemon.
    /// Returns the exit status of the non-daemon process that triggered shutdown.
    pub fn run(&mut self) -> i32 {
        loop {
            match ::sys::kcall::ipc::__kcall_recv() {
                Ok(message) => {
                    ::syslog::info!("received message from={:?}", { message.source });
                    let source_pid: ProcessIdentifier = { message.source }.pid;
                    match message.message_type {
                        MessageType::Exception => {
                            ::syslog::warn!("received unexpected exception message, ignoring");
                        },
                        MessageType::Ipc => {
                            if let Err(e) = self.handle_ipc_message(message) {
                                ::syslog::error!("failed to handle IPC message (error={:?})", e);
                            }
                        },
                        MessageType::Interrupt => {
                            ::syslog::warn!("received unexpected interrupt message, ignoring");
                        },
                        MessageType::Ikc => {
                            ::syslog::warn!("received unexpected IKC message, ignoring");
                        },
                        MessageType::ProcessTerminationEvent => {
                            if source_pid != ProcessIdentifier::KERNEL {
                                ::syslog::warn!(
                                    "dropping forged process termination event (source={:?})",
                                    source_pid
                                );
                                continue;
                            }
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
                            if source_pid != ProcessIdentifier::KERNEL {
                                ::syslog::warn!(
                                    "dropping forged process creation event (source={:?})",
                                    source_pid
                                );
                                continue;
                            }
                            if let Err(e) = self.handle_process_creation_event(message) {
                                ::syslog::error!(
                                    "failed to handle process creation event (error={:?})",
                                    e
                                );
                            }
                            continue;
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
    fn handle_process_creation_event(&mut self, message: Message) -> Result<(), Error> {
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

        ::syslog::info!(
            "process created (child={:?}, parent={:?}, role={:?})",
            child,
            parent,
            info.role
        );

        self.record_child_lineage(parent, child);

        // The kernel assigns the role authoritatively at spawn time. Record the init process the
        // first time its creation is observed, so orphaned children can be re-parented to it even
        // when the boot workload never signs up. Only the first init process is recorded; it is
        // cleared when that process terminates (which also triggers system shutdown).
        if info.role == ProcessRole::Init && self.init_proc.is_none() {
            ::syslog::info!("recording init process (pid={:?})", child);
            self.init_proc = Some(child);
        }

        // The kernel spawns daemons and the init process directly (parent is the kernel), so
        // there is no parent filesystem state to inherit: skip the fork-clone notification for
        // them. This avoids needless boot-time traffic to the filesystem daemon and a phantom
        // per-process state keyed by the kernel. Only genuine user-space forks (parent is another
        // process) require duplication. A failed dispatch is recorded so that a later fork-sync
        // request fails the fork instead of being acknowledged on the basis of a snapshot that will
        // never be taken, which would let parent and child proceed past unduplicated filesystem
        // state; a successful dispatch is confirmed separately by the filesystem daemon's
        // acknowledgement (see below).
        if parent != ProcessIdentifier::KERNEL {
            let request_id: Option<RequestIdentifier> = self.notify_fork_clone(parent, child);
            if let Some(request_id) = request_id {
                if let Some(record) = self.processes.get_mut(&child) {
                    record.fork_clone_request_id = Some(request_id);
                }
            } else {
                // The fork-clone notification could not be dispatched. Mark the failure so that a
                // fork-sync waiter (whether already pending below or arriving later) is released
                // with a failure acknowledgement rather than left blocked forever.
                if let Some(record) = self.processes.get_mut(&child) {
                    record.fork_clone_failed = true;
                }
            }
            // On a successful dispatch `fork_clone_done` stays false: it is set only when the
            // filesystem daemon acknowledges that it has actually taken the snapshot (see
            // `handle_fork_clone_ack`). Gating the fork-sync release on that acknowledgement keeps
            // the freshly forked child from issuing its first filesystem operation -- such as the
            // `execv()` image load, which opens a descriptor and makes its table active -- ahead of
            // the clone, which would otherwise refuse the clone and drop the inherited descriptors.
        }

        // Release a parent (and its child) that is already blocked awaiting fork synchronization.
        // Two conditions must hold before the waiter is acknowledged with success:
        //
        // 1. The waiter must match the kernel-attributed real parent of this child. A pending entry
        //    whose waiter differs was injected by a process that named a `child` that is not
        //    actually its own (the `child` field of a fork-sync request is untrusted): drop it
        //    without acknowledging, so it cannot inject a spurious acknowledgement into a victim's
        //    mailbox or displace the genuine waiter.
        // 2. The fork-clone must have actually been *taken* by the filesystem daemon, tracked by
        //    `fork_clone_done` (set when its acknowledgement arrives). If the clone could not even
        //    be dispatched, the waiter is released with a failure acknowledgement so that `fork()`
        //    aborts. If the clone was dispatched but is not yet acknowledged, the waiter is left
        //    pending here and released later by `handle_fork_clone_ack`, so neither process resumes
        //    before the snapshot is actually in place.
        if let Some(pos) = self
            .pending_fork_syncs
            .iter()
            .position(|pending| pending.child == child)
        {
            let pending: PendingForkSync = self.pending_fork_syncs[pos];
            if pending.parent != parent {
                // Forged waiter: drop it without acknowledging.
                self.pending_fork_syncs.swap_remove(pos);
                ::syslog::warn!(
                    "dropping forged fork-sync (waiter={:?}, child={:?}, real_parent={:?})",
                    pending.parent,
                    child,
                    parent
                );
            } else if self
                .processes
                .get(&child)
                .map(|record| record.fork_clone_done)
                .unwrap_or(false)
            {
                // Genuine waiter and the fork-clone has been acknowledged: release it.
                self.pending_fork_syncs.swap_remove(pos);
                self.release_fork_sync(pending.parent, child, pending.response_context);
            } else if self
                .processes
                .get(&child)
                .map(|record| record.fork_clone_failed)
                .unwrap_or(false)
            {
                // Genuine waiter but the fork-clone could not be dispatched (the notification failed
                // to build or send). Release it with a failure acknowledgement so that `fork()`
                // aborts in both parent and child instead of deadlocking forever on a snapshot that
                // was never taken.
                self.pending_fork_syncs.swap_remove(pos);
                ::syslog::warn!(
                    "fork-clone not dispatched, failing fork-sync waiter (parent={:?}, child={:?})",
                    pending.parent,
                    child
                );
                self.fail_fork_sync(pending.parent, child, pending.response_context);
            }
            // Otherwise the clone was dispatched but not yet acknowledged: leave the waiter pending;
            // `handle_fork_clone_ack` releases it once the filesystem daemon confirms the snapshot.
        }

        Ok(())
    }

    /// Handles a process-termination scheduling event.
    ///
    /// Routes the termination on the role assigned authoritatively by the kernel, which spawns the
    /// init process and the daemons directly and owns the well-known daemon process identifiers:
    /// the init process triggers system shutdown, a daemon is deregistered (a non-zero status is a
    /// crash and also triggers shutdown), and a forked user process is reaped. Because the role and
    /// parent are carried in the event, procd no longer reconstructs them from prior, race-prone
    /// state.
    fn handle_process_termination_event(&mut self, message: Message) -> Result<Option<i32>, Error> {
        // Deserialize the authoritative termination information published by the kernel.
        let raw_info: [u8; ::core::mem::size_of::<ProcessTerminationInfo>()] =
            match message.payload[0..::core::mem::size_of::<ProcessTerminationInfo>()].try_into() {
                Ok(bytes) => bytes,
                Err(_) => {
                    let reason: &str = "invalid process termination message payload";
                    ::syslog::error!("handle_process_termination_event(): {reason:?}");
                    return Err(Error::new(ErrorCode::InvalidArgument, reason));
                },
            };
        let info: ProcessTerminationInfo = ProcessTerminationInfo::from_ne_bytes(raw_info);
        let pid: ProcessIdentifier = info.pid;
        let status: i32 = info.status.as_u32() as i32;

        ::syslog::info!(
            "process terminated (pid={:?}, parent={:?}, role={:?}, status={:?})",
            pid,
            info.parent,
            info.role,
            status
        );

        match info.role {
            // The init process terminated — initiate shutdown and propagate its exit status.
            ProcessRole::Init => {
                ::syslog::info!("init process terminated (pid={:?}, status={:?})", pid, status);
                self.cleanup_terminated(pid);
                self.processes.remove(&pid);
                self.clear_foreground_pgrp_if_empty();
                self.init_proc = None;
                Ok(Some(status))
            },

            // A daemon terminated — deregister it. A non-zero status means the daemon crashed,
            // which triggers a system-wide shutdown.
            ProcessRole::Daemon => {
                ::syslog::info!("deregistering daemon (pid={:?}, status={:?})", pid, status);
                self.cleanup_terminated(pid);
                self.processes.remove(&pid);
                self.clear_foreground_pgrp_if_empty();
                if status != 0 {
                    ::syslog::error!(
                        "critical daemon (pid={:?}) terminated with non-zero status {} — \
                         triggering shutdown",
                        pid,
                        status
                    );
                    return Ok(Some(status));
                }
                Ok(None)
            },

            // A forked user process terminated. Finalizing it re-parents its surviving children,
            // reaps its zombie children, and either retains it as a reapable zombie or drops it —
            // all of which require the child's lineage to be recorded. The kernel guarantees that a
            // process's creation event is delivered before its termination event (a single FIFO
            // scheduling-event queue), so by the time this termination is handled the creation event
            // has already recorded the child's lineage. The filesystem-exit notification issued by
            // `cleanup_terminated()` is therefore ordered after the fork-clone that the creation
            // handler dispatched, so the child's filesystem snapshot is taken before it is reclaimed.
            ProcessRole::User => {
                self.cleanup_terminated(pid);
                self.finalize_forked_child_termination(pid, status)?;
                Ok(None)
            },
        }
    }

    /// Drops bookkeeping owned by a terminated process and notifies the filesystem daemon to
    /// reclaim its per-process state (open file descriptors and working directory). The filesystem
    /// notification is sent for every terminating process — daemons and the init process accumulate
    /// their own filesystem state lazily as they open files, and that state must be reclaimed too;
    /// it is a no-op in the filesystem daemon for a process that never registered any state.
    fn cleanup_terminated(&mut self, pid: ProcessIdentifier) {
        // Drop fork-sync bookkeeping owned by the terminating process. When the parent dies, fail
        // the child with the retained request ID so it cannot remain blocked forever. When the
        // child dies first, fail the surviving parent for the same reason.
        let mut orphaned_children: Vec<PendingForkSync> = Vec::new();
        let mut failed_parents: Vec<PendingForkSync> = Vec::new();
        self.pending_fork_syncs.retain(|pending| {
            if pending.parent == pid && pending.child != pid {
                orphaned_children.push(*pending);
                false
            } else if pending.child == pid {
                failed_parents.push(*pending);
                false
            } else {
                true
            }
        });
        for pending in orphaned_children {
            self.send_fork_sync_ack_to_child(
                pending.child,
                ErrorCode::TryAgain.get(),
                pending.response_context,
            );
        }
        for pending in failed_parents {
            self.send_fork_sync_ack_to_parent(
                pending.parent,
                ErrorCode::TryAgain.get(),
                pending.response_context,
            );
        }
        // Drop any exec-barrier bookkeeping owned by the terminating process. A process that died
        // while held at the exec barrier can never be released, so leaving its entry behind would
        // strand it and leak the slot across pid reuse.
        self.pending_execs.retain(|pending| pending.process != pid);
        // Drop any blocked-wait bookkeeping owned by the terminating process. A process that was
        // itself parked in `waitpid()` can never be answered once it is gone, so leaving its entry
        // behind would leak memory and strand a stale waiter.
        self.blocked.retain(|waiter| waiter.waiter != pid);
        self.notify_process_exit(pid);
    }

    /// Finalizes the termination of a forked child `pid` whose lineage is known. Auto-reaps any of
    /// its own children that are already zombies (only this terminating process could ever have
    /// reaped them, so re-homing them to init — which never calls `waitpid()` — would leak them
    /// until shutdown), re-parents its surviving live children to the init process, then decides
    /// reapability: if a live parent can still reap it, it is retained as a zombie and a parent
    /// already blocked in `waitpid()` is woken; otherwise it is dropped rather than left as an
    /// unreapable zombie.
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
                self.clear_foreground_pgrp_if_empty();
                // Notify the parent of the child's state change after servicing any blocked
                // `waitpid()` waiter. This keeps the synchronous reap path stable while still
                // generating the asynchronous `SIGCHLD` notification.
                self.notify_parent_sigchld(parent);
            },
            // No live process can ever reap it: drop it rather than leak an unreapable zombie.
            None => {
                self.processes.remove(&pid);
                self.clear_foreground_pgrp_if_empty();
            },
        }

        Ok(())
    }

    /// Returns the process that should adopt orphaned children: the init process, recorded from
    /// the role the kernel carries in its process-creation event. Returns `None` only before the
    /// init process's creation event has been observed, in which case orphans are not re-parented;
    /// each is dropped when it later terminates, since its parent is gone and no live process can
    /// reap it.
    fn adoptive_init(&self) -> Option<ProcessIdentifier> {
        self.init_proc
    }

    /// Re-parents the surviving children of `pid` to the init process, recorded from the role the
    /// kernel carries in its process-creation event. When no init process is known (its creation
    /// event has not been observed yet), the children are left in place; each is dropped when it
    /// later terminates, since its parent is gone and no one can reap it.
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
        response_context: ResponseContext,
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

        // Block this exact request; another thread or a nested signal handler may have an
        // independent wait in flight for the same process.
        if self.blocked.len() >= MAX_BLOCKED_WAITERS {
            let reply: Message = message::wait_response(
                caller,
                ProcessIdentifier::from(0),
                0,
                ErrorCode::NoBufferSpace.get(),
            )?;
            return Ok(Some(reply));
        }
        self.blocked.push(BlockedWaiter {
            waiter: caller,
            selector,
            response_context,
        });

        Ok(None)
    }

    /// Cancels one exact blocked wait request if completion has not already won.
    fn handle_wait_cancel(
        &mut self,
        caller: ProcessIdentifier,
        tid: ThreadIdentifier,
        request_id: RequestIdentifier,
    ) -> bool {
        let index: Option<usize> = self.blocked.iter().position(|waiter| {
            waiter.waiter == caller
                && waiter.response_context.receiver.tid == tid
                && waiter.response_context.request_id == request_id
        });
        if let Some(index) = index {
            self.blocked.swap_remove(index);
            true
        } else {
            false
        }
    }

    /// Removes every blocked wait owned by one exiting thread.
    fn handle_thread_exit(&mut self, caller: ProcessIdentifier, tid: ThreadIdentifier) {
        self.blocked.retain(|waiter| {
            waiter.waiter != caller || waiter.response_context.receiver.tid != tid
        });
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
        while let Some(index) = self
            .blocked
            .iter()
            .position(|waiter| waiter.waiter == parent && waiter.selector.matches(child))
        {
            let waiter_pid: ProcessIdentifier = self.blocked[index].waiter;
            let response_context: ResponseContext = self.blocked[index].response_context;
            let reply: Message = message::wait_response(waiter_pid, child, status, 0)?;
            match response_context.send(reply) {
                Ok(()) => {},
                Err(error) if error.code == ErrorCode::NoSuchEntry => {
                    self.blocked.swap_remove(index);
                    ::syslog::warn!(
                        "dropping unreachable wait request (waiter={:?}, tid={:?}, request_id={})",
                        waiter_pid,
                        response_context.receiver.tid,
                        response_context.request_id.raw()
                    );
                    continue;
                },
                Err(error) => return Err(error),
            }

            self.blocked.swap_remove(index);
            self.reap(parent, child);
            self.resolve_ineligible_waiters(parent)?;
            break;
        }

        Ok(())
    }

    /// Resolves blocked waits that can no longer match any child after another waiter reaped one.
    fn resolve_ineligible_waiters(&mut self, parent: ProcessIdentifier) -> Result<(), Error> {
        let mut index: usize = 0;
        while index < self.blocked.len() {
            let has_eligible_child: bool = {
                let waiter: &BlockedWaiter = &self.blocked[index];
                waiter.waiter != parent
                    || self.processes.get(&parent).is_some_and(|record| {
                        record
                            .children
                            .iter()
                            .any(|child| waiter.selector.matches(*child))
                    })
            };
            if has_eligible_child {
                index += 1;
                continue;
            }

            let waiter: BlockedWaiter = self.blocked.swap_remove(index);
            let reply: Message = message::wait_response(
                waiter.waiter,
                ProcessIdentifier::from(0),
                0,
                ErrorCode::NoChildProcess.get(),
            )?;
            // The waiter has already been dequeued, so an unreachable requesting thread must not
            // abort the resolution of the remaining waiters.
            if let Err(error) = waiter.response_context.send(reply) {
                ::syslog::warn!(
                    "failed to answer ineligible wait request (waiter={:?}, tid={:?}, \
                     request_id={}, error={:?})",
                    waiter.waiter,
                    waiter.response_context.receiver.tid,
                    waiter.response_context.request_id.raw(),
                    error
                );
            }
        }
        Ok(())
    }

    /// Posts `SIGCHLD` to `parent` to notify it of a child's state change (here, termination), as
    /// required by POSIX. `SIGCHLD` defaults to being ignored, so this is a no-op for a parent that
    /// has not installed a handler; it complements, rather than replaces, the `waitpid()` flow. A
    /// failure to post is logged but never propagated: the child's termination must still be
    /// finalized (the zombie retained and any blocked waiter woken) even when the notification
    /// cannot be delivered.
    fn notify_parent_sigchld(&self, parent: ProcessIdentifier) {
        if let Err(e) = ::sys::kcall::pm::__kcall_kill(parent, SIGCHLD as i32) {
            ::syslog::warn!(
                "notify_parent_sigchld(): failed to post SIGCHLD (parent={:?}, error={:?})",
                parent,
                e
            );
        }
    }

    /// Reaps a zombie `child` of `parent`, removing it from the registry and from the parent's list
    /// of children.
    fn reap(&mut self, parent: ProcessIdentifier, child: ProcessIdentifier) {
        if let Some(record) = self.processes.get_mut(&parent) {
            record.children.retain(|c| *c != child);
        }
        self.processes.remove(&child);
        self.clear_foreground_pgrp_if_empty();
    }

    /// Returns `true` if `name` belongs to a guest system daemon that should not trigger shutdown.
    fn is_daemon(name: &str) -> bool {
        ::config::daemons::is_system_daemon(name)
    }

    ///
    /// # Description
    ///
    /// Handles a kill request: posts `signum` to `target` on behalf of `caller`.
    ///
    /// The daemon is the privileged gateway for cross-process signalling: it holds
    /// [`Capability::ProcessManagement`], so it authorizes the request before forwarding it to the
    /// in-kernel posting primitive. The current standalone identity model treats processes as root;
    /// if richer credentials are recorded later, the usual same-user or root policy is enforced.
    ///
    /// # Parameters
    ///
    /// - `caller`: Process identifier of the requesting process.
    /// - `message`: The kill request.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the kill response to send back to the caller. Upon failure, an
    /// error is returned instead.
    ///
    fn handle_kill(
        &mut self,
        caller: ProcessIdentifier,
        message: KillMessage,
    ) -> Result<Message, Error> {
        // Copy the fields out of the packed message before use to avoid unaligned references.
        let target: ProcessIdentifier = message.target;
        let signum: i32 = message.signum;

        ::syslog::info!(
            "handle_kill(): caller={:?}, target={:?}, signum={}",
            caller,
            target,
            signum
        );

        // POSIX `kill()` overloads the sign of the target pid to select a process group: a positive
        // pid names a single process, while `0`, `-1`, and `< -1` name the caller's group, every
        // process, and a specific group respectively. The single-target path is unchanged; the
        // group selectors fan the signal out across every matching member.
        let raw_target: i32 = target.into();
        let error: i32 = if raw_target > 0 {
            self.kill_one(caller, target, signum)
        } else {
            self.kill_group(caller, raw_target, signum)
        };

        message::kill_response(caller, error)
    }

    /// Posts `signum` to a single `target` process on behalf of `caller`, authorizing the request
    /// first. Returns `0` on success or the error code otherwise.
    fn kill_one(&self, caller: ProcessIdentifier, target: ProcessIdentifier, signum: i32) -> i32 {
        match self.authorize_kill(caller, target) {
            Ok(()) => match ::sys::kcall::pm::__kcall_kill(target, signum) {
                Ok(()) => 0,
                Err(e) => e.code.get(),
            },
            Err(e) => e.code.get(),
        }
    }

    /// Posts `signum` to a process group selected by the sign of `raw_target`, on behalf of
    /// `caller`. Mirrors POSIX `kill()` group semantics: `0` selects the caller's process group,
    /// `-1` broadcasts to every signalable process, and `< -1` selects the process group whose
    /// identifier is `-raw_target`. Returns `0` when at least one signal was posted, `EPERM` when
    /// members matched but none could be signalled, or `ESRCH` when no member matched.
    fn kill_group(&self, caller: ProcessIdentifier, raw_target: i32, signum: i32) -> i32 {
        // Resolve the set of target processes from the selector.
        let targets: Vec<ProcessIdentifier> = if raw_target == -1 {
            // Broadcast: every process the caller may signal, excluding the kernel placeholder and
            // the system daemons (they own well-known low identifiers below the init process).
            self.processes
                .keys()
                .copied()
                .filter(|pid| i32::from(*pid) >= ProcessIdentifier::INIT_RAW)
                .collect()
        } else {
            // `0` selects the caller's own group; `< -1` selects the group `-raw_target`.
            let pgid: ProcessIdentifier = if raw_target == 0 {
                match self.processes.get(&caller) {
                    Some(record) => record.pgid,
                    None => return ErrorCode::NoSuchProcess.get(),
                }
            } else {
                // Negating `i32::MIN` would overflow; such a process group can never exist, so it is
                // reported as having no member rather than panicking.
                match raw_target.checked_neg() {
                    Some(pgid_raw) => ProcessIdentifier::from(pgid_raw),
                    None => return ErrorCode::NoSuchProcess.get(),
                }
            };
            self.group_members(pgid)
        };

        if targets.is_empty() {
            return ErrorCode::NoSuchProcess.get();
        }

        let mut any_sent: bool = false;
        for target in targets {
            if self.authorize_kill(caller, target).is_err() {
                continue;
            }
            if ::sys::kcall::pm::__kcall_kill(target, signum).is_ok() {
                any_sent = true;
            }
        }

        if any_sent {
            0
        } else {
            // Members matched, but none could be signalled (permission denied or every post
            // failed): report that the operation was not permitted.
            ErrorCode::OperationNotPermitted.get()
        }
    }

    /// Returns the process identifiers of every live member of process group `pgid`.
    fn group_members(&self, pgid: ProcessIdentifier) -> Vec<ProcessIdentifier> {
        self.processes
            .iter()
            .filter(|(_, record)| record.pgid == pgid && record.zombie.is_none())
            .map(|(pid, _)| *pid)
            .collect()
    }

    /// Clears the terminal foreground process group when it no longer has any live member.
    fn clear_foreground_pgrp_if_empty(&mut self) {
        if let Some(pgrp) = self.foreground_pgrp {
            if self.group_members(pgrp).is_empty() {
                self.foreground_pgrp = None;
            }
        }
    }

    /// Posts `signum` to every member of process group `pgid`, ignoring individual failures. Used by
    /// the terminal paths, where the daemon is the privileged sender and no per-target permission
    /// check applies (the signal originates from the controlling terminal, not from a process).
    fn post_group_signal(&self, pgid: ProcessIdentifier, signum: i32) {
        for pid in self.group_members(pgid) {
            if let Err(e) = ::sys::kcall::pm::__kcall_kill(pid, signum) {
                ::syslog::warn!(
                    "post_group_signal(): failed (pid={:?}, signum={}, error={:?})",
                    pid,
                    signum,
                    e
                );
            }
        }
    }

    ///
    /// # Description
    ///
    /// Handles a job-control request: manipulates or queries the session, process-group, and
    /// foreground-group state the daemon owns.
    ///
    /// # Parameters
    ///
    /// - `caller`: Process identifier of the requesting process.
    /// - `request`: The job-control request.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the job-control response to send back to the caller. Upon
    /// failure, an error is returned instead.
    ///
    fn handle_job_control(
        &mut self,
        caller: ProcessIdentifier,
        request: JobControlRequest,
    ) -> Result<Message, Error> {
        // Copy the fields out of the packed request before use to avoid unaligned references.
        let pid: ProcessIdentifier = request.pid;
        let pgid: ProcessIdentifier = request.pgid;

        let op: JobControlOp = match request.op() {
            Ok(op) => op,
            Err(_) => {
                return message::job_control_response(
                    caller,
                    ErrorCode::InvalidArgument.get(),
                    ProcessIdentifier::from(0),
                );
            },
        };

        ::syslog::info!(
            "handle_job_control(): caller={:?}, op={:?}, pid={:?}, pgid={:?}",
            caller,
            op,
            pid,
            pgid
        );

        let outcome: Result<ProcessIdentifier, Error> = match op {
            JobControlOp::SetSid => self.job_control_setsid(caller),
            JobControlOp::SetPgid => self.job_control_setpgid(caller, pid, pgid).map(|()| pid),
            JobControlOp::GetPgid => self.job_control_getpgid(caller, pid),
            JobControlOp::GetSid => self.job_control_getsid(caller, pid),
            JobControlOp::TcSetPgrp => self.job_control_tcsetpgrp(caller, pgid),
            JobControlOp::TcGetPgrp => self.job_control_tcgetpgrp(caller),
        };

        let (error, result): (i32, ProcessIdentifier) = match outcome {
            Ok(result) => (0, result),
            Err(e) => (e.code.get(), ProcessIdentifier::from(0)),
        };

        message::job_control_response(caller, error, result)
    }

    /// Resolves a job-control target pid, where `0` selects the caller.
    fn resolve_target(caller: ProcessIdentifier, pid: ProcessIdentifier) -> ProcessIdentifier {
        if pid == ProcessIdentifier::from(0) {
            caller
        } else {
            pid
        }
    }

    /// Implements `setsid()`: the caller becomes the leader of a new session and a new process
    /// group. Fails with `EPERM` when the caller is already a process-group leader, as POSIX
    /// requires (a group leader cannot move itself into a brand-new session).
    fn job_control_setsid(
        &mut self,
        caller: ProcessIdentifier,
    ) -> Result<ProcessIdentifier, Error> {
        let record: &mut ProcessRecord = self
            .processes
            .get_mut(&caller)
            .ok_or_else(|| Error::new(ErrorCode::NoSuchProcess, "caller not found"))?;

        if record.pgid == caller {
            return Err(Error::new(
                ErrorCode::OperationNotPermitted,
                "caller is already a process-group leader",
            ));
        }

        record.sid = caller;
        record.pgid = caller;
        Ok(caller)
    }

    /// Implements `setpgid()`: moves `pid` (or the caller when `pid` is `0`) into process group
    /// `pgid` (or a new group led by the target when `pgid` is `0`). Enforces the POSIX constraints
    /// that the target be the caller or a child of the caller, that it not be a session leader, and
    /// that both the target and the destination group live in the caller's session.
    fn job_control_setpgid(
        &mut self,
        caller: ProcessIdentifier,
        pid: ProcessIdentifier,
        pgid: ProcessIdentifier,
    ) -> Result<(), Error> {
        let caller_sid: ProcessIdentifier = self
            .processes
            .get(&caller)
            .map(|record| record.sid)
            .ok_or_else(|| Error::new(ErrorCode::NoSuchProcess, "caller not found"))?;

        let target: ProcessIdentifier = Self::resolve_target(caller, pid);

        let (target_sid, target_parent): (ProcessIdentifier, Option<ProcessIdentifier>) = {
            let record: &ProcessRecord = self
                .processes
                .get(&target)
                .ok_or_else(|| Error::new(ErrorCode::NoSuchProcess, "target not found"))?;
            (record.sid, record.parent)
        };

        // The target must be the caller or a child of the caller.
        if target != caller && target_parent != Some(caller) {
            return Err(Error::new(
                ErrorCode::NoSuchProcess,
                "target is neither the caller nor a child of the caller",
            ));
        }

        // The target must be in the caller's session.
        if target_sid != caller_sid {
            return Err(Error::new(
                ErrorCode::OperationNotPermitted,
                "target is in a different session",
            ));
        }

        // A session leader cannot change its process group.
        if target_sid == target {
            return Err(Error::new(ErrorCode::OperationNotPermitted, "target is a session leader"));
        }

        let new_pgid: ProcessIdentifier = if pgid == ProcessIdentifier::from(0) {
            target
        } else {
            pgid
        };

        // A new group may be created only when its identifier is the target itself; joining an
        // existing group requires that group to already live in the caller's session.
        if new_pgid != target {
            let exists_in_session: bool = self
                .processes
                .values()
                .any(|record| record.pgid == new_pgid && record.sid == caller_sid);
            if !exists_in_session {
                return Err(Error::new(
                    ErrorCode::OperationNotPermitted,
                    "destination process group is not in the caller's session",
                ));
            }
        }

        if let Some(record) = self.processes.get_mut(&target) {
            record.pgid = new_pgid;
        }
        self.clear_foreground_pgrp_if_empty();
        Ok(())
    }

    /// Implements `getpgid()`: returns the process group of `pid` (or the caller when `pid` is `0`).
    fn job_control_getpgid(
        &self,
        caller: ProcessIdentifier,
        pid: ProcessIdentifier,
    ) -> Result<ProcessIdentifier, Error> {
        let target: ProcessIdentifier = Self::resolve_target(caller, pid);
        self.processes
            .get(&target)
            .map(|record| record.pgid)
            .ok_or_else(|| Error::new(ErrorCode::NoSuchProcess, "process not found"))
    }

    /// Implements `getsid()`: returns the session of `pid` (or the caller when `pid` is `0`).
    fn job_control_getsid(
        &self,
        caller: ProcessIdentifier,
        pid: ProcessIdentifier,
    ) -> Result<ProcessIdentifier, Error> {
        let target: ProcessIdentifier = Self::resolve_target(caller, pid);
        self.processes
            .get(&target)
            .map(|record| record.sid)
            .ok_or_else(|| Error::new(ErrorCode::NoSuchProcess, "process not found"))
    }

    /// Implements `tcsetpgrp()`: makes `pgrp` the foreground process group of the controlling
    /// terminal. The group must already exist in the caller's session.
    fn job_control_tcsetpgrp(
        &mut self,
        caller: ProcessIdentifier,
        pgrp: ProcessIdentifier,
    ) -> Result<ProcessIdentifier, Error> {
        let caller_sid: ProcessIdentifier = self
            .processes
            .get(&caller)
            .map(|record| record.sid)
            .ok_or_else(|| Error::new(ErrorCode::NoSuchProcess, "caller not found"))?;

        // The requested foreground group must be a group in the caller's session.
        let valid: bool = self
            .processes
            .values()
            .any(|record| record.pgid == pgrp && record.sid == caller_sid);
        if !valid {
            return Err(Error::new(
                ErrorCode::OperationNotPermitted,
                "process group is not in the caller's session",
            ));
        }

        self.foreground_pgrp = Some(pgrp);
        Ok(pgrp)
    }

    /// Implements `tcgetpgrp()`: returns the foreground process group of the controlling terminal.
    /// When no foreground group has been established, the caller's own group is reported, matching
    /// the default in which the caller is the foreground process.
    fn job_control_tcgetpgrp(&self, caller: ProcessIdentifier) -> Result<ProcessIdentifier, Error> {
        if let Some(pgrp) = self.foreground_pgrp {
            return Ok(pgrp);
        }
        self.processes
            .get(&caller)
            .map(|record| record.pgid)
            .ok_or_else(|| Error::new(ErrorCode::NoSuchProcess, "caller not found"))
    }

    /// Handles a terminal-signal notification from the console owner: delivers `signum` to the
    /// controlling terminal's foreground process group. A signal posted while no foreground group is
    /// established has nowhere to go and is dropped.
    fn handle_terminal_signal(&mut self, signum: i32) {
        match self.foreground_pgrp {
            Some(pgrp) => {
                ::syslog::info!(
                    "handle_terminal_signal(): delivering signum={} to foreground group {:?}",
                    signum,
                    pgrp
                );
                self.post_group_signal(pgrp, signum);
            },
            None => {
                ::syslog::info!(
                    "handle_terminal_signal(): no foreground group; dropping signum={}",
                    signum
                );
            },
        }
    }

    /// Handles a terminal-access notification from the console owner: when `pid` accessed the console
    /// from a *background* process group (one that is not the terminal's foreground group), the
    /// access raises `SIGTTOU` (write) or `SIGTTIN` (read) on that group, as POSIX requires. An
    /// access from the foreground group, or while no foreground group is established, is allowed
    /// silently.
    fn handle_terminal_access(&mut self, pid: ProcessIdentifier, write: bool) {
        let foreground: ProcessIdentifier = match self.foreground_pgrp {
            Some(pgrp) => pgrp,
            None => return,
        };

        let pgid: ProcessIdentifier = match self.processes.get(&pid) {
            Some(record) => record.pgid,
            None => return,
        };

        if pgid == foreground {
            return;
        }

        let signum: i32 = if write {
            SIGTTOU as i32
        } else {
            SIGTTIN as i32
        };
        ::syslog::info!(
            "handle_terminal_access(): background {} by {:?} (group {:?}); raising signum={}",
            if write { "write" } else { "read" },
            pid,
            pgid,
            signum
        );
        self.post_group_signal(pgid, signum);
    }

    /// Resolves the subject of a terminal-access notification from `reporter`.
    ///
    /// The filesystem daemon may report console reads on behalf of any process because it owns the
    /// shared input path. A process may report only its own console writes; the kernel-stamped
    /// message source prevents it from forging another process's terminal access.
    fn terminal_access_subject(
        reporter: ProcessIdentifier,
        notification: &TerminalAccessMessage,
    ) -> Option<ProcessIdentifier> {
        let pid: ProcessIdentifier = notification.pid;
        if reporter == ProcessIdentifier::VFSD || pid == reporter {
            Some(pid)
        } else {
            None
        }
    }

    /// Authorizes a kill request before the daemon spends its process-management capability.
    fn authorize_kill(
        &self,
        caller: ProcessIdentifier,
        target: ProcessIdentifier,
    ) -> Result<(), Error> {
        let caller_identity: &ProcessIdentity = match self
            .processes
            .get(&caller)
            .and_then(|record| record.identity.as_ref())
        {
            Some(identity) => identity,
            None => {
                let reason: &str = "caller identity not found";
                ::syslog::error!(
                    "authorize_kill(): {reason} (caller={:?}, target={:?})",
                    caller,
                    target
                );
                return Err(Error::new(ErrorCode::NoSuchProcess, reason));
            },
        };
        let target_identity: &ProcessIdentity = match self
            .processes
            .get(&target)
            .and_then(|record| record.identity.as_ref())
        {
            Some(identity) => identity,
            None => {
                let reason: &str = "target identity not found";
                ::syslog::error!(
                    "authorize_kill(): {reason} (caller={:?}, target={:?})",
                    caller,
                    target
                );
                return Err(Error::new(ErrorCode::NoSuchProcess, reason));
            },
        };

        if caller_identity.can_signal(target_identity) {
            Ok(())
        } else {
            let reason: &str = "signal permission denied";
            ::syslog::error!(
                "authorize_kill(): {reason} (caller={:?}, target={:?})",
                caller,
                target
            );
            Err(Error::new(ErrorCode::PermissionDenied, reason))
        }
    }

    fn handle_ipc_message(&mut self, message: Message) -> Result<(), Error> {
        // The kernel stamps the authoritative originating process and thread into `message.source`.
        // Retain both, plus the request identifier, before parsing shadows the raw message.
        let sender: MessageSender = { message.source };
        let caller: ProcessIdentifier = sender.pid;
        let request_id: RequestIdentifier = RequestIdentifier::read_from(&message);
        let response_context: ResponseContext = ResponseContext::new(sender, request_id);
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
                    let message: Message = self.handle_signup(caller, message)?;
                    response_context.send(message)?;
                },
                ProcessManagementMessageHeader::Lookup => {
                    let message: LookupMessage = LookupMessage::from_bytes(message.payload);
                    let message: Message = self.handle_lookup(caller, message)?;
                    response_context.send(message)?;
                },
                ProcessManagementMessageHeader::ForkSync => {
                    let message: ForkSyncMessage = ForkSyncMessage::from_bytes(message.payload);
                    self.handle_fork_sync(caller, message.child, response_context);
                },
                ProcessManagementMessageHeader::Exec => {
                    // A freshly `exec`'d process announces that it has replaced its image. The
                    // source is attributed by the kernel, so the subject is the source itself: a
                    // process can only ever request the barrier for itself, never for another.
                    self.handle_exec_sync(caller, response_context);
                },
                ProcessManagementMessageHeader::ExecAck => {
                    // The filesystem daemon confirms whether close-on-exec was applied. Only it may
                    // drive this acknowledgement; one from any other source is a forgery that could
                    // release a process before its descriptors were dropped, so it is dropped
                    // without effect. The daemon's outcome (`status`) is forwarded unchanged to the
                    // held process so that a best-effort failure can be signalled rather than masked
                    // as success.
                    if caller == ProcessIdentifier::VFSD {
                        let ack: ExecAckMessage = ExecAckMessage::from_bytes(message.payload);
                        self.handle_exec_ack(ack.pid, ack.status, request_id);
                    } else {
                        ::syslog::warn!("dropping forged exec-ack (source={:?})", caller);
                    }
                },
                ProcessManagementMessageHeader::ForkCloneAck => {
                    // The filesystem daemon confirms whether it has duplicated the parent's
                    // filesystem state onto the freshly forked child. Only it may drive this
                    // acknowledgement; one from any other source is a forgery that could release a
                    // fork-sync waiter before the snapshot was taken, so it is dropped without
                    // effect.
                    if caller == ProcessIdentifier::VFSD {
                        let ack: ForkCloneAckMessage =
                            ForkCloneAckMessage::from_bytes(message.payload);
                        self.handle_fork_clone_ack(ack.child, ack.status, request_id);
                    } else {
                        ::syslog::warn!("dropping forged fork-clone-ack (source={:?})", caller);
                    }
                },
                ProcessManagementMessageHeader::Wait => {
                    let message: WaitMessage = WaitMessage::from_bytes(message.payload);
                    // The reply is deferred (no send here) when the waiter blocks; it is produced
                    // later by the process-termination handler.
                    if let Some(reply) = self.handle_wait(caller, response_context, message)? {
                        response_context.send(reply)?;
                    }
                },
                ProcessManagementMessageHeader::WaitCancel => {
                    let request: WaitCancelMessage = WaitCancelMessage::from_bytes(message.payload);
                    let cancelled: bool =
                        self.handle_wait_cancel(caller, sender.tid, request.request_id());
                    let reply: Message = message::wait_cancel_response(caller, cancelled)?;
                    response_context.send(reply)?;
                },
                ProcessManagementMessageHeader::ThreadExit => {
                    self.handle_thread_exit(caller, sender.tid);
                },
                ProcessManagementMessageHeader::Kill => {
                    let message: KillMessage = KillMessage::from_bytes(message.payload);
                    let reply: Message = self.handle_kill(caller, message)?;
                    response_context.send(reply)?;
                },
                ProcessManagementMessageHeader::JobControl => {
                    let request: JobControlRequest = JobControlRequest::from_bytes(message.payload);
                    let reply: Message = self.handle_job_control(caller, request)?;
                    response_context.send(reply)?;
                },
                ProcessManagementMessageHeader::TerminalSignal => {
                    // The console line-discipline owner is the only legitimate source of a
                    // terminal-generated signal. A notification from any other process is a forgery
                    // that could signal the foreground group at will, so it is dropped.
                    if caller == ProcessIdentifier::VFSD {
                        let notification: TerminalSignalMessage =
                            TerminalSignalMessage::from_bytes(message.payload);
                        self.handle_terminal_signal(notification.signum);
                    } else {
                        ::syslog::warn!("dropping forged terminal-signal (source={:?})", caller);
                    }
                },
                ProcessManagementMessageHeader::TerminalAccess => {
                    // The console input owner may report reads on behalf of clients. Console writes
                    // bypass vfsd, so a process may report only its own write access; the kernel
                    // stamps the source pid, making cross-process reports detectable.
                    let notification: TerminalAccessMessage =
                        TerminalAccessMessage::from_bytes(message.payload);
                    match Self::terminal_access_subject(caller, &notification) {
                        Some(pid) => self.handle_terminal_access(pid, notification.is_write()),
                        None => {
                            ::syslog::warn!(
                                "dropping forged terminal-access (source={:?})",
                                caller
                            );
                        },
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
                    // Preserve any existing lineage (parent/children) that may have been recorded
                    // by a process-creation event before this signup. Only update the process name:
                    // the init process and daemons are identified authoritatively by the role the
                    // kernel carries in their scheduling events, so signup never decides identity.
                    match self.processes.get_mut(&pid) {
                        Some(record) => {
                            record.name = s;
                        },
                        None => {
                            self.processes.insert(pid, ProcessRecord::new(pid, s, None));
                        },
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
            .or_insert_with(|| ProcessRecord::new(parent, String::new(), None));

        // A forked child inherits the name of its parent.
        let child_name: String = self
            .processes
            .get(&parent)
            .map(|record| record.name.clone())
            .unwrap_or_default();

        // A forked child inherits its parent's session and process group; a process born directly
        // from the kernel (init and the daemons) has no inheritable parent and stays the leader of
        // its own session and group, which is exactly what `ProcessRecord::new` seeds.
        let inherited: Option<(ProcessIdentifier, ProcessIdentifier)> =
            if parent == ProcessIdentifier::KERNEL {
                None
            } else {
                self.processes
                    .get(&parent)
                    .map(|record| (record.sid, record.pgid))
            };

        // Insert or update the child record.
        match self.processes.get_mut(&child) {
            Some(record) => {
                record.parent = Some(parent);
            },
            None => {
                let mut record: ProcessRecord = ProcessRecord::new(child, child_name, Some(parent));
                if let Some((sid, pgid)) = inherited {
                    record.sid = sid;
                    record.pgid = pgid;
                }
                self.processes.insert(child, record);
            },
        }

        // Append the child to the parent's list of children.
        if let Some(record) = self.processes.get_mut(&parent) {
            if !record.children.contains(&child) {
                record.children.push(child);
            }
        }
    }

    /// Notifies the filesystem daemon that the filesystem resources of `parent` must be cloned onto
    /// the freshly forked `child`. The notification is sent asynchronously: the process manager
    /// daemon does not block waiting for the acknowledgement, so that it never risks swallowing an
    /// unrelated message (such as a process-termination event) on its receive path.
    ///
    /// Returns `true` if the fork-clone request was successfully handed to the kernel for delivery
    /// to the filesystem daemon, and `false` if it could not be built or sent. The caller uses a
    /// `false` return to mark the child's fork-clone as failed (a successful dispatch is instead
    /// confirmed later by the filesystem daemon's acknowledgement): a fork-sync request must not
    /// be acknowledged on the basis of a clone that was never delivered.
    fn notify_fork_clone(
        &mut self,
        parent: ProcessIdentifier,
        child: ProcessIdentifier,
    ) -> Option<RequestIdentifier> {
        let request_id: RequestIdentifier = self.allocate_request_id();
        match message::fork_clone_request(parent, child) {
            Ok(mut request) => {
                request_id.write_to(&mut request);
                match ::sys::kcall::ipc::__kcall_send(&request) {
                    Ok(()) => Some(request_id),
                    Err(e) => {
                        ::syslog::warn!(
                            "notify_fork_clone: failed to notify vfsd to clone resources \
                             (parent={:?}, child={:?}, error={:?})",
                            parent,
                            child,
                            e
                        );
                        None
                    },
                }
            },
            Err(e) => {
                ::syslog::warn!(
                    "notify_fork_clone: failed to build fork-clone request (parent={:?}, \
                     child={:?}, error={:?})",
                    parent,
                    child,
                    e
                );
                None
            },
        }
    }

    /// Handles a fork-sync request from a freshly forked `parent` awaiting confirmation that the
    /// filesystem state of `child` has been duplicated.
    ///
    /// If the child's fork-clone has already been acknowledged by the filesystem daemon, the parent
    /// and child are released immediately with a success acknowledgement. If the fork-clone failed
    /// (it could not be dispatched, or the filesystem daemon reported it could not take the
    /// snapshot), they are released with a failure acknowledgement so that `fork()` aborts instead
    /// of hanging. Otherwise the request is recorded and resolved once the filesystem daemon
    /// acknowledges the clone (see [`handle_fork_clone_ack`]). Because the release is ordered after
    /// the filesystem daemon has actually taken the snapshot, neither process can issue a filesystem
    /// operation that races ahead of -- and is therefore dropped by -- the clone.
    ///
    /// [`handle_fork_clone_ack`]: Self::handle_fork_clone_ack
    fn handle_fork_sync(
        &mut self,
        parent: ProcessIdentifier,
        child: ProcessIdentifier,
        response_context: ResponseContext,
    ) {
        ::syslog::info!("fork-sync request (parent={:?}, child={:?})", parent, child);

        let parent_is_live: bool = self
            .processes
            .get(&parent)
            .is_some_and(|record| record.zombie.is_none());
        if !parent_is_live {
            ::syslog::warn!(
                "rejecting fork-sync from terminated parent (parent={:?}, child={:?})",
                parent,
                child
            );
            self.fail_fork_sync(parent, child, response_context);
            return;
        }

        // The request's source (`parent`) is attributed by the kernel and therefore trustworthy,
        // but the named `child` is taken from the untrusted request payload. When the child's
        // lineage is already known, reject the request unless the requester is the recorded parent
        // of `child`. Honoring a request whose `child` is not actually the requester's child would
        // let a malicious process inject a spurious fork-sync acknowledgement into an arbitrary
        // victim's mailbox. Requests that arrive before the child's process-creation event (lineage
        // not yet known) are validated later, when that event is processed.
        let child_is_verified: bool = if let Some(record) = self.processes.get(&child) {
            if record.parent != Some(parent) {
                ::syslog::warn!(
                    "rejecting forged fork-sync (requester={:?}, child={:?}, real_parent={:?})",
                    parent,
                    child,
                    record.parent
                );
                self.send_fork_sync_ack_to_parent(
                    parent,
                    ErrorCode::InvalidArgument.get(),
                    response_context,
                );
                return;
            }
            true
        } else {
            false
        };

        let (cloned, failed): (bool, bool) = self
            .processes
            .get(&child)
            .map(|record| (record.fork_clone_done, record.fork_clone_failed))
            .unwrap_or((false, false));
        if cloned {
            self.release_fork_sync(parent, child, response_context);
        } else if failed {
            // The fork-clone could not be dispatched: abort the fork rather than block forever.
            ::syslog::warn!(
                "fork-clone not dispatched, failing fork-sync request (parent={:?}, child={:?})",
                parent,
                child
            );
            self.fail_fork_sync(parent, child, response_context);
        } else {
            if let Some(pending) = self
                .pending_fork_syncs
                .iter()
                .find(|pending| pending.child == child)
            {
                ::syslog::warn!(
                    "rejecting duplicate fork-sync (requester={:?}, child={:?}, waiter={:?})",
                    parent,
                    child,
                    pending.parent
                );
                // The child may still be waiting on the original context. Reply only to this
                // request's sender so a per-thread request-id collision cannot release the child.
                self.send_fork_sync_ack_to_parent(
                    parent,
                    ErrorCode::ResourceBusy.get(),
                    response_context,
                );
                return;
            }
            if self.pending_fork_syncs.len() >= MAX_PENDING_FORK_SYNCS {
                ::syslog::warn!(
                    "fork-sync queue is full, failing requester (parent={:?}, child={:?})",
                    parent,
                    child
                );
                if child_is_verified {
                    self.send_fork_sync_ack(
                        parent,
                        child,
                        ErrorCode::NoBufferSpace.get(),
                        response_context,
                    );
                } else {
                    // Until the process-creation event establishes lineage, `child` is untrusted.
                    // The parent tears down its child after receiving this failure.
                    self.send_fork_sync_ack_to_parent(
                        parent,
                        ErrorCode::NoBufferSpace.get(),
                        response_context,
                    );
                }
                return;
            }
            self.pending_fork_syncs.push(PendingForkSync {
                child,
                parent,
                response_context,
            });
        }
    }

    /// Releases a parent and its freshly forked child that are blocked awaiting fork
    /// synchronization, by acknowledging both with success. The filesystem daemon has already
    /// acknowledged that it took the fork-clone snapshot, so these acknowledgements are necessarily
    /// ordered after it.
    fn release_fork_sync(
        &self,
        parent: ProcessIdentifier,
        child: ProcessIdentifier,
        response_context: ResponseContext,
    ) {
        self.send_fork_sync_ack(
            parent,
            child,
            ForkSyncAckMessage::STATUS_SUCCESS,
            response_context,
        );
    }

    /// Releases a parent and its freshly forked child that are blocked awaiting fork
    /// synchronization, by acknowledging both with a failure status. Used when the fork-clone
    /// failed -- either it could not be dispatched to the filesystem daemon, or the filesystem
    /// daemon acknowledged that it could not take the snapshot -- so that `fork()` aborts in both
    /// processes instead of deadlocking on a snapshot that will never be taken.
    fn fail_fork_sync(
        &self,
        parent: ProcessIdentifier,
        child: ProcessIdentifier,
        response_context: ResponseContext,
    ) {
        self.send_fork_sync_ack(parent, child, ErrorCode::TryAgain.get(), response_context);
    }

    /// Sends a fork-sync acknowledgement carrying `status` to both the `parent` and the `child`.
    fn send_fork_sync_ack(
        &self,
        parent: ProcessIdentifier,
        child: ProcessIdentifier,
        status: i32,
        response_context: ResponseContext,
    ) {
        for (pid, response_context) in [
            (parent, response_context),
            (child, Self::child_response_context(child, response_context)),
        ] {
            match message::fork_sync_ack(pid, status) {
                Ok(ack) => {
                    if let Err(e) = response_context.send(ack) {
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

    /// Sends a fork-sync acknowledgement only to the child process mailbox.
    fn send_fork_sync_ack_to_child(
        &self,
        child: ProcessIdentifier,
        status: i32,
        response_context: ResponseContext,
    ) {
        let child_context: ResponseContext = Self::child_response_context(child, response_context);
        match message::fork_sync_ack(child, status) {
            Ok(ack) => {
                if let Err(error) = child_context.send(ack) {
                    ::syslog::warn!(
                        "failed to acknowledge orphaned fork child (child={:?}, error={:?})",
                        child,
                        error
                    );
                }
            },
            Err(error) => ::syslog::warn!(
                "failed to build orphaned child acknowledgement (child={:?}, error={:?})",
                child,
                error
            ),
        }
    }

    /// Sends a fork-sync acknowledgement only to the parent process mailbox.
    fn send_fork_sync_ack_to_parent(
        &self,
        parent: ProcessIdentifier,
        status: i32,
        response_context: ResponseContext,
    ) {
        match message::fork_sync_ack(parent, status) {
            Ok(ack) => {
                if let Err(error) = response_context.send(ack) {
                    ::syslog::warn!(
                        "failed to acknowledge parent of terminated fork child (parent={:?}, \
                         error={:?})",
                        parent,
                        error
                    );
                }
            },
            Err(error) => ::syslog::warn!(
                "failed to build parent acknowledgement for terminated fork child (parent={:?}, \
                 error={:?})",
                parent,
                error
            ),
        }
    }

    fn child_response_context(
        child: ProcessIdentifier,
        response_context: ResponseContext,
    ) -> ResponseContext {
        ResponseContext {
            receiver: MessageReceiver::new(child, ThreadIdentifier::NONE),
            request_id: response_context.request_id,
        }
    }

    /// Handles a fork-clone acknowledgement from the filesystem daemon, recording the outcome for
    /// `child` and releasing a parent and child blocked at the fork-synchronization barrier.
    ///
    /// The acknowledgement arrives once the filesystem daemon has actually duplicated (or failed to
    /// duplicate) the parent's filesystem state onto `child`. Recording the outcome lets a fork-sync
    /// request that arrives after this point be answered immediately (see [`handle_fork_sync`]),
    /// while any waiter already blocked is released here: with success when the snapshot is in
    /// place, or with a failure status so that `fork()` aborts rather than proceeding with a child
    /// whose descriptor table was never cloned. The blocked waiter's parent must match `child`'s
    /// recorded real parent; a mismatch is a forged entry and is dropped without acknowledging.
    ///
    /// [`handle_fork_sync`]: Self::handle_fork_sync
    fn handle_fork_clone_ack(
        &mut self,
        child: ProcessIdentifier,
        status: i32,
        request_id: RequestIdentifier,
    ) {
        let expected_request_id: Option<RequestIdentifier> = self
            .processes
            .get(&child)
            .and_then(|record| record.fork_clone_request_id);
        if expected_request_id != Some(request_id) {
            ::syslog::warn!(
                "dropping stale fork-clone-ack (child={:?}, expected_request_id={:?}, \
                 request_id={})",
                child,
                expected_request_id.map(RequestIdentifier::raw),
                request_id.raw()
            );
            return;
        }

        let success: bool = status == ForkCloneAckMessage::STATUS_SUCCESS;

        // Record the outcome so a fork-sync request that races ahead of this acknowledgement is
        // answered on its fast path.
        if let Some(record) = self.processes.get_mut(&child) {
            record.fork_clone_request_id = None;
            if success {
                record.fork_clone_done = true;
            } else {
                record.fork_clone_failed = true;
            }
        }

        // Release any waiter already blocked on this child.
        if let Some(pos) = self
            .pending_fork_syncs
            .iter()
            .position(|pending| pending.child == child)
        {
            let pending: PendingForkSync = self.pending_fork_syncs[pos];
            let real_parent: Option<ProcessIdentifier> =
                self.processes.get(&child).and_then(|record| record.parent);
            self.pending_fork_syncs.swap_remove(pos);
            if real_parent != Some(pending.parent) {
                ::syslog::warn!(
                    "dropping forged fork-sync on clone ack (waiter={:?}, child={:?}, \
                     real_parent={:?})",
                    pending.parent,
                    child,
                    real_parent
                );
            } else if success {
                self.release_fork_sync(pending.parent, child, pending.response_context);
            } else {
                ::syslog::warn!(
                    "fork-clone failed, failing fork-sync waiter (parent={:?}, child={:?})",
                    pending.parent,
                    child
                );
                self.fail_fork_sync(pending.parent, child, pending.response_context);
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

    /// Handles an exec-sync request from a freshly `exec`'d `process` whose new image must be held
    /// until the filesystem daemon has applied close-on-exec to its inherited descriptor table.
    ///
    /// Relays the close-on-exec request to the filesystem daemon and records the process as
    /// awaiting that daemon's acknowledgement, which `handle_exec_ack()` resolves. If the relay
    /// could not be dispatched, the process is released immediately with a failure acknowledgement:
    /// its image has already been replaced and the `exec` cannot be undone, so it proceeds on a
    /// best-effort basis rather than blocking forever on an acknowledgement that will never come.
    fn handle_exec_sync(&mut self, process: ProcessIdentifier, response_context: ResponseContext) {
        ::syslog::info!("exec-sync request (process={:?})", process);
        if self
            .pending_execs
            .iter()
            .any(|pending| pending.process == process)
        {
            ::syslog::warn!("rejecting duplicate exec-sync (process={:?})", process);
            self.release_exec_sync(process, ErrorCode::ResourceBusy.get(), response_context);
            return;
        }
        // The old image and all of its thread identifiers are gone, so none of its blocked wait
        // requests can receive a response. The replacement image may issue new waits after this
        // barrier completes.
        self.blocked.retain(|waiter| waiter.waiter != process);
        if let Some(request_id) = self.notify_exec(process) {
            self.pending_execs.push(PendingExec {
                process,
                request_id,
                response_context,
            });
        } else {
            ::syslog::warn!(
                "exec close-on-exec not dispatched, failing exec-sync (process={:?})",
                process
            );
            self.release_exec_sync(process, ErrorCode::TryAgain.get(), response_context);
        }
    }

    /// Notifies the filesystem daemon to apply close-on-exec to `process`'s descriptor table,
    /// mirroring how `notify_fork_clone` dispatches a fork-clone.
    ///
    /// Returns `true` if the request was handed to the kernel for delivery, and `false` if it could
    /// not be built or sent. The daemon does not block on an acknowledgement here: the filesystem
    /// daemon's acknowledgement is delivered asynchronously and resolved in `handle_exec_ack()`,
    /// so that the receive path never risks swallowing an unrelated message.
    fn notify_exec(&mut self, process: ProcessIdentifier) -> Option<RequestIdentifier> {
        let request_id: RequestIdentifier = self.allocate_request_id();
        match message::exec_request(ProcessIdentifier::PROCD, ProcessIdentifier::VFSD, process) {
            Ok(mut request) => {
                request_id.write_to(&mut request);
                match ::sys::kcall::ipc::__kcall_send(&request) {
                    Ok(()) => Some(request_id),
                    Err(e) => {
                        ::syslog::warn!(
                            "notify_exec: failed to notify vfsd to apply close-on-exec \
                             (process={:?}, error={:?})",
                            process,
                            e
                        );
                        None
                    },
                }
            },
            Err(e) => {
                ::syslog::warn!(
                    "notify_exec: failed to build exec request (process={:?}, error={:?})",
                    process,
                    e
                );
                None
            },
        }
    }

    /// Handles an exec acknowledgement from the filesystem daemon, releasing the held `process`
    /// whose close-on-exec has now been resolved with outcome `status`.
    ///
    /// Only a process recorded as awaiting acknowledgement is released, so a stale or duplicated
    /// acknowledgement — or one naming a process that never requested the barrier — cannot release
    /// a process spuriously. The filesystem daemon's `status` is forwarded unchanged to the
    /// released process: `0` reports that close-on-exec was applied, while a non-zero code lets the
    /// process proceed on a best-effort basis after the daemon could not complete it.
    fn handle_exec_ack(
        &mut self,
        process: ProcessIdentifier,
        status: i32,
        request_id: RequestIdentifier,
    ) {
        if let Some(pos) = self
            .pending_execs
            .iter()
            .position(|pending| pending.process == process && pending.request_id == request_id)
        {
            let pending: PendingExec = self.pending_execs.swap_remove(pos);
            self.release_exec_sync(process, status, pending.response_context);
        } else {
            ::syslog::warn!(
                "ignoring stale exec-ack (process={:?}, request_id={})",
                process,
                request_id.raw()
            );
        }
    }

    /// Releases a `process` held at the exec barrier by acknowledging it with `status`. A `0`
    /// status releases it after close-on-exec has been applied; a non-zero status releases it on a
    /// best-effort basis after the barrier could not be completed.
    fn release_exec_sync(
        &self,
        process: ProcessIdentifier,
        status: i32,
        response_context: ResponseContext,
    ) {
        match message::exec_ack(ProcessIdentifier::PROCD, process, process, status) {
            Ok(ack) => {
                if let Err(e) = response_context.send(ack) {
                    ::syslog::warn!(
                        "release_exec_sync: failed to acknowledge (process={:?}, status={:?}, \
                         error={:?})",
                        process,
                        status,
                        e
                    );
                }
            },
            Err(e) => {
                ::syslog::warn!(
                    "release_exec_sync: failed to build acknowledgement (process={:?}, \
                     status={:?}, error={:?})",
                    process,
                    status,
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
                        let source_pid: ProcessIdentifier = { message.source }.pid;
                        if source_pid != ProcessIdentifier::KERNEL {
                            ::syslog::warn!(
                                "dropping forged process termination event during shutdown \
                                 (source={:?})",
                                source_pid
                            );
                            continue;
                        }
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

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::mem::ManuallyDrop;

    fn process_identity(uid: usize) -> ProcessIdentity {
        ProcessIdentity::new(UserIdentifier::from(uid), GroupIdentifier::ROOT)
    }

    fn process_record(pid: ProcessIdentifier, identity: Option<ProcessIdentity>) -> ProcessRecord {
        ProcessRecord {
            name: String::new(),
            identity,
            parent: None,
            children: Vec::new(),
            fork_clone_done: false,
            fork_clone_failed: false,
            fork_clone_request_id: None,
            zombie: None,
            sid: pid,
            pgid: pid,
        }
    }

    fn process_daemon(
        records: &[(ProcessIdentifier, Option<ProcessIdentity>)],
    ) -> ManuallyDrop<ProcessDaemon> {
        let mut processes: BTreeMap<ProcessIdentifier, ProcessRecord> = BTreeMap::new();
        for (pid, identity) in records.iter() {
            processes.insert(*pid, process_record(*pid, identity.clone()));
        }

        ManuallyDrop::new(ProcessDaemon {
            processes,
            init_proc: None,
            pending_fork_syncs: Vec::new(),
            pending_execs: Vec::new(),
            next_request_id: 1,
            blocked: Vec::new(),
            foreground_pgrp: None,
        })
    }

    fn response_context(process: ProcessIdentifier) -> ResponseContext {
        ResponseContext::new(
            MessageSender::new(process, ThreadIdentifier::from(20)),
            RequestIdentifier::from_raw(99),
        )
    }

    #[test]
    fn relay_request_identifier_wraps_and_skips_active_identifiers() {
        let process: ProcessIdentifier = ProcessIdentifier::from(10);
        let mut daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(process, Some(process_identity(1000)))]);
        let fork_request_id: RequestIdentifier = RequestIdentifier::from_raw(u32::MAX);
        let exec_request_id: RequestIdentifier = RequestIdentifier::from_raw(1);
        daemon
            .processes
            .get_mut(&process)
            .expect("process should exist")
            .fork_clone_request_id = Some(fork_request_id);
        daemon.pending_execs.push(PendingExec {
            process,
            request_id: exec_request_id,
            response_context: response_context(process),
        });
        daemon.next_request_id = u32::MAX;

        assert_eq!(daemon.allocate_request_id(), RequestIdentifier::from_raw(2));
        assert_eq!(daemon.next_request_id, 3);
    }

    #[test]
    fn stale_exec_ack_keeps_matching_request_pending() {
        let process: ProcessIdentifier = ProcessIdentifier::from(10);
        let expected: RequestIdentifier = RequestIdentifier::from_raw(7);
        let mut daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(process, Some(process_identity(1000)))]);
        daemon.pending_execs.push(PendingExec {
            process,
            request_id: expected,
            response_context: response_context(process),
        });

        daemon.handle_exec_ack(process, 0, RequestIdentifier::from_raw(8));

        assert_eq!(daemon.pending_execs.len(), 1);
        assert_eq!(daemon.pending_execs[0].request_id, expected);
    }

    #[test]
    fn stale_fork_clone_ack_keeps_matching_request_pending() {
        let child: ProcessIdentifier = ProcessIdentifier::from(11);
        let expected: RequestIdentifier = RequestIdentifier::from_raw(7);
        let mut daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(child, Some(process_identity(1000)))]);
        daemon
            .processes
            .get_mut(&child)
            .expect("child should exist")
            .fork_clone_request_id = Some(expected);

        daemon.handle_fork_clone_ack(child, 0, RequestIdentifier::from_raw(8));

        let record: &ProcessRecord = daemon.processes.get(&child).expect("child should exist");
        assert_eq!(record.fork_clone_request_id, Some(expected));
        assert!(!record.fork_clone_done);
        assert!(!record.fork_clone_failed);
    }

    #[test]
    fn immediate_wait_response_stamps_request_id_and_exact_thread() {
        let process: ProcessIdentifier = ProcessIdentifier::from(10);
        let thread: ThreadIdentifier = ThreadIdentifier::from(20);
        let request_id: RequestIdentifier = RequestIdentifier::from_raw(0x12345678);
        let response_context: ResponseContext =
            ResponseContext::new(MessageSender::new(process, thread), request_id);
        let mut daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(process, Some(process_identity(1000)))]);
        let mut response: Message = daemon
            .handle_wait(process, response_context, WaitMessage::new(WaitTarget::Any, 0))
            .expect("wait request should be handled")
            .expect("caller without children should receive an immediate response");

        response_context.prepare_response(&mut response);

        assert_eq!(
            { response.destination },
            MessageReceiver::new(process, thread),
            "response should target the requesting thread"
        );
        assert_eq!(
            RequestIdentifier::read_from(&response),
            request_id,
            "response should echo the request identifier"
        );
    }

    #[test]
    fn blocked_waiter_retains_response_context() {
        let parent: ProcessIdentifier = ProcessIdentifier::from(10);
        let child: ProcessIdentifier = ProcessIdentifier::from(11);
        let thread: ThreadIdentifier = ThreadIdentifier::from(20);
        let request_id: RequestIdentifier = RequestIdentifier::from_raw(0x87654321);
        let response_context: ResponseContext =
            ResponseContext::new(MessageSender::new(parent, thread), request_id);
        let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
            (parent, Some(process_identity(1000))),
            (child, Some(process_identity(1000))),
        ]);
        daemon
            .processes
            .get_mut(&parent)
            .expect("parent record should exist")
            .children
            .push(child);

        let reply: Option<Message> = daemon
            .handle_wait(parent, response_context, WaitMessage::new(WaitTarget::Pid(child), 0))
            .expect("blocking wait should be accepted");

        assert!(reply.is_none(), "live child should block the waiter");
        assert_eq!(daemon.blocked.len(), 1, "waiter should be retained");
        let retained_context: ResponseContext = daemon.blocked[0].response_context;
        assert_eq!(
            retained_context, response_context,
            "waiter should retain the original response context"
        );

        let mut response: Message = message::wait_response(parent, child, 7, 0)
            .expect("deferred wait response should be constructed");
        retained_context.prepare_response(&mut response);
        assert_eq!(
            { response.destination },
            MessageReceiver::new(parent, thread),
            "deferred response should target the requesting thread"
        );
        assert_eq!(
            RequestIdentifier::read_from(&response),
            request_id,
            "deferred response should echo the request identifier"
        );
    }

    #[test]
    fn concurrent_blocked_waiters_retain_distinct_response_contexts() {
        let parent: ProcessIdentifier = ProcessIdentifier::from(10);
        let child: ProcessIdentifier = ProcessIdentifier::from(11);
        let first_context: ResponseContext = ResponseContext::new(
            MessageSender::new(parent, ThreadIdentifier::from(20)),
            RequestIdentifier::from_raw(100),
        );
        let second_context: ResponseContext = ResponseContext::new(
            MessageSender::new(parent, ThreadIdentifier::from(21)),
            RequestIdentifier::from_raw(101),
        );
        let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
            (parent, Some(process_identity(1000))),
            (child, Some(process_identity(1000))),
        ]);
        daemon
            .processes
            .get_mut(&parent)
            .expect("parent record should exist")
            .children
            .push(child);

        for context in [first_context, second_context] {
            let reply: Option<Message> = daemon
                .handle_wait(parent, context, WaitMessage::new(WaitTarget::Pid(child), 0))
                .expect("blocking wait should be accepted");
            assert!(reply.is_none(), "live child should block the waiter");
        }

        assert_eq!(daemon.blocked.len(), 2, "both waits should remain queued");
        assert_eq!(daemon.blocked[0].response_context, first_context);
        assert_eq!(daemon.blocked[1].response_context, second_context);
    }

    #[test]
    fn wait_cancel_removes_only_exact_request() {
        let parent: ProcessIdentifier = ProcessIdentifier::from(10);
        let first_tid: ThreadIdentifier = ThreadIdentifier::from(20);
        let second_tid: ThreadIdentifier = ThreadIdentifier::from(21);
        let first_id: RequestIdentifier = RequestIdentifier::from_raw(100);
        let second_id: RequestIdentifier = RequestIdentifier::from_raw(101);
        let mut daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(parent, Some(process_identity(1000)))]);
        daemon.blocked.push(BlockedWaiter {
            waiter: parent,
            selector: WaitSelector::Any,
            response_context: ResponseContext::new(MessageSender::new(parent, first_tid), first_id),
        });
        daemon.blocked.push(BlockedWaiter {
            waiter: parent,
            selector: WaitSelector::Any,
            response_context: ResponseContext::new(
                MessageSender::new(parent, second_tid),
                second_id,
            ),
        });

        assert!(daemon.handle_wait_cancel(parent, first_tid, first_id));
        assert_eq!(daemon.blocked.len(), 1);
        assert_eq!(daemon.blocked[0].response_context.request_id, second_id);
    }

    #[test]
    fn thread_exit_removes_only_exiting_threads_waiters() {
        let parent: ProcessIdentifier = ProcessIdentifier::from(10);
        let exiting_tid: ThreadIdentifier = ThreadIdentifier::from(20);
        let surviving_tid: ThreadIdentifier = ThreadIdentifier::from(21);
        let mut daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(parent, Some(process_identity(1000)))]);
        for (tid, request_id) in [
            (exiting_tid, RequestIdentifier::from_raw(100)),
            (surviving_tid, RequestIdentifier::from_raw(101)),
        ] {
            daemon.blocked.push(BlockedWaiter {
                waiter: parent,
                selector: WaitSelector::Any,
                response_context: ResponseContext::new(MessageSender::new(parent, tid), request_id),
            });
        }

        daemon.handle_thread_exit(parent, exiting_tid);

        assert_eq!(daemon.blocked.len(), 1);
        assert_eq!(daemon.blocked[0].response_context.receiver.tid, surviving_tid);
    }

    #[test]
    fn authorize_kill_allows_root_caller() {
        let caller: ProcessIdentifier = ProcessIdentifier::from(10);
        let target: ProcessIdentifier = ProcessIdentifier::from(11);
        let daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
            (caller, Some(process_identity(0))),
            (target, Some(process_identity(1000))),
        ]);

        assert!(daemon.authorize_kill(caller, target).is_ok());
    }

    #[test]
    fn authorize_kill_allows_same_user() {
        let caller: ProcessIdentifier = ProcessIdentifier::from(10);
        let target: ProcessIdentifier = ProcessIdentifier::from(11);
        let daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
            (caller, Some(process_identity(1000))),
            (target, Some(process_identity(1000))),
        ]);

        assert!(daemon.authorize_kill(caller, target).is_ok());
    }

    #[test]
    fn authorize_kill_rejects_different_user() {
        let caller: ProcessIdentifier = ProcessIdentifier::from(10);
        let target: ProcessIdentifier = ProcessIdentifier::from(11);
        let daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
            (caller, Some(process_identity(1000))),
            (target, Some(process_identity(1001))),
        ]);
        let error: Error = daemon
            .authorize_kill(caller, target)
            .expect_err("different users should not be able to signal each other");

        assert_eq!(error.code, ErrorCode::PermissionDenied);
    }

    #[test]
    fn authorize_kill_rejects_unknown_caller() {
        let caller: ProcessIdentifier = ProcessIdentifier::from(10);
        let target: ProcessIdentifier = ProcessIdentifier::from(11);
        let daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(target, Some(process_identity(1000)))]);
        let error: Error = daemon
            .authorize_kill(caller, target)
            .expect_err("unknown caller should not be authorized");

        assert_eq!(error.code, ErrorCode::NoSuchProcess);
    }

    #[test]
    fn authorize_kill_rejects_unknown_target() {
        let caller: ProcessIdentifier = ProcessIdentifier::from(10);
        let target: ProcessIdentifier = ProcessIdentifier::from(11);
        let daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(caller, Some(process_identity(1000)))]);
        let error: Error = daemon
            .authorize_kill(caller, target)
            .expect_err("unknown target should not be authorized");

        assert_eq!(error.code, ErrorCode::NoSuchProcess);
    }

    #[test]
    fn authorize_kill_rejects_missing_caller_identity() {
        let caller: ProcessIdentifier = ProcessIdentifier::from(10);
        let target: ProcessIdentifier = ProcessIdentifier::from(11);
        let daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(caller, None), (target, Some(process_identity(1000)))]);
        let error: Error = daemon
            .authorize_kill(caller, target)
            .expect_err("caller without identity should not be authorized");

        assert_eq!(error.code, ErrorCode::NoSuchProcess);
    }

    #[test]
    fn authorize_kill_rejects_missing_target_identity() {
        let caller: ProcessIdentifier = ProcessIdentifier::from(10);
        let target: ProcessIdentifier = ProcessIdentifier::from(11);
        let daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(caller, Some(process_identity(1000))), (target, None)]);
        let error: Error = daemon
            .authorize_kill(caller, target)
            .expect_err("target without identity should not be authorized");

        assert_eq!(error.code, ErrorCode::NoSuchProcess);
    }

    #[test]
    fn terminal_access_allows_vfsd_to_report_any_process() {
        let reader: ProcessIdentifier = ProcessIdentifier::from(10);
        let notification: TerminalAccessMessage = TerminalAccessMessage::new(reader, false);

        assert_eq!(
            ProcessDaemon::terminal_access_subject(ProcessIdentifier::VFSD, &notification),
            Some(reader)
        );
    }

    #[test]
    fn terminal_access_allows_self_report() {
        let writer: ProcessIdentifier = ProcessIdentifier::from(10);
        let notification: TerminalAccessMessage = TerminalAccessMessage::new(writer, true);

        assert_eq!(ProcessDaemon::terminal_access_subject(writer, &notification), Some(writer));
    }

    #[test]
    fn terminal_access_rejects_cross_process_report() {
        let reporter: ProcessIdentifier = ProcessIdentifier::from(10);
        let target: ProcessIdentifier = ProcessIdentifier::from(11);
        let notification: TerminalAccessMessage = TerminalAccessMessage::new(target, true);

        assert_eq!(ProcessDaemon::terminal_access_subject(reporter, &notification), None);
    }

    #[test]
    fn foreground_pgrp_clears_when_last_member_is_removed() {
        let leader: ProcessIdentifier = ProcessIdentifier::from(10);
        let mut daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(leader, Some(process_identity(0)))]);
        daemon.foreground_pgrp = Some(leader);

        daemon.processes.remove(&leader);
        daemon.clear_foreground_pgrp_if_empty();

        assert_eq!(daemon.foreground_pgrp, None);
    }

    #[test]
    fn foreground_pgrp_ignores_zombie_members() {
        let leader: ProcessIdentifier = ProcessIdentifier::from(10);
        let member: ProcessIdentifier = ProcessIdentifier::from(11);
        let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
            (leader, Some(process_identity(0))),
            (member, Some(process_identity(0))),
        ]);
        daemon.foreground_pgrp = Some(leader);
        daemon
            .processes
            .get_mut(&member)
            .expect("member record")
            .pgid = leader;

        daemon
            .processes
            .get_mut(&leader)
            .expect("leader record")
            .zombie = Some(0);
        daemon
            .processes
            .get_mut(&member)
            .expect("member record")
            .zombie = Some(0);
        daemon.clear_foreground_pgrp_if_empty();

        assert_eq!(daemon.foreground_pgrp, None);
    }

    #[test]
    fn foreground_pgrp_stays_when_group_has_live_member() {
        let leader: ProcessIdentifier = ProcessIdentifier::from(10);
        let member: ProcessIdentifier = ProcessIdentifier::from(11);
        let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
            (leader, Some(process_identity(0))),
            (member, Some(process_identity(0))),
        ]);
        daemon.foreground_pgrp = Some(leader);
        daemon
            .processes
            .get_mut(&member)
            .expect("member record")
            .pgid = leader;

        daemon
            .processes
            .get_mut(&leader)
            .expect("leader record")
            .zombie = Some(0);
        daemon.clear_foreground_pgrp_if_empty();

        assert_eq!(daemon.foreground_pgrp, Some(leader));
    }
}
