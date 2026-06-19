// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    identity::ProcessIdentity,
    message,
    ExecAckMessage,
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
        ProcessRole,
        ProcessTerminationInfo,
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
    /// Process identifier of the init process, recorded from the role the kernel carries in the
    /// process-creation event. Used only to re-parent orphaned children; the shutdown decision is
    /// made authoritatively from the role carried in the termination event, not from this field.
    init_proc: Option<ProcessIdentifier>,
    /// Fork-sync requests awaiting the fork-clone dispatch, stored as `(child, parent)` pairs that
    /// map a child to the blocked parent. Populated when a fork-sync request arrives before the
    /// child's process-creation event; drained when that event dispatches the clone. A `Vec` is
    /// used rather than a map because only a handful of fork operations are ever pending
    /// concurrently, so a linear scan is cheaper than the overhead of an ordered map.
    pending_fork_syncs: Vec<(ProcessIdentifier, ProcessIdentifier)>,
    /// Processes held at the exec synchronization barrier, awaiting the filesystem daemon's
    /// acknowledgement that close-on-exec has been applied to their inherited descriptor table.
    /// Populated when a freshly `exec`'d process requests the barrier and the close-on-exec
    /// notification is dispatched to the filesystem daemon; drained when that daemon acknowledges,
    /// at which point the process is released. A `Vec` is used rather than a map because only a
    /// handful of exec operations are ever pending concurrently, so a linear scan is cheaper than
    /// the overhead of an ordered map.
    pending_execs: Vec<ProcessIdentifier>,
    /// Parents currently blocked in a `Wait` operation. A blocking `waitpid()` is parked here and
    /// answered later, when a `ProcessTermination` event for a matching child arrives.
    blocked: Vec<BlockedWaiter>,
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
            pending_execs: Vec::new(),
            blocked: Vec::new(),
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
                self.init_proc = None;
                Ok(Some(status))
            },

            // A daemon terminated — deregister it. A non-zero status means the daemon crashed,
            // which triggers a system-wide shutdown.
            ProcessRole::Daemon => {
                ::syslog::info!("deregistering daemon (pid={:?}, status={:?})", pid, status);
                self.cleanup_terminated(pid);
                self.processes.remove(&pid);
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
        // Drop any stale fork-sync bookkeeping for the terminating process.
        self.pending_fork_syncs.retain(|(child, _)| *child != pid);
        // Drop any exec-barrier bookkeeping owned by the terminating process. A process that died
        // while held at the exec barrier can never be released, so leaving its entry behind would
        // strand it and leak the slot across pid reuse.
        self.pending_execs.retain(|process| *process != pid);
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
            },
            // No live process can ever reap it: drop it rather than leak an unreapable zombie.
            None => {
                self.processes.remove(&pid);
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

    /// Returns `true` if `name` belongs to a guest system daemon that should not trigger shutdown.
    fn is_daemon(name: &str) -> bool {
        ::config::daemons::is_system_daemon(name)
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
                ProcessManagementMessageHeader::Exec => {
                    // A freshly `exec`'d process announces that it has replaced its image. The
                    // source is attributed by the kernel, so the subject is the source itself: a
                    // process can only ever request the barrier for itself, never for another.
                    self.handle_exec_sync(destination);
                },
                ProcessManagementMessageHeader::ExecAck => {
                    // The filesystem daemon confirms whether close-on-exec was applied. Only it may
                    // drive this acknowledgement; one from any other source is a forgery that could
                    // release a process before its descriptors were dropped, so it is dropped
                    // without effect. The daemon's outcome (`status`) is forwarded unchanged to the
                    // held process so that a best-effort failure can be signalled rather than masked
                    // as success.
                    if destination == ProcessIdentifier::VFSD {
                        let ack: ExecAckMessage = ExecAckMessage::from_bytes(message.payload);
                        self.handle_exec_ack(ack.pid, ack.status);
                    } else {
                        ::syslog::warn!("dropping forged exec-ack (source={:?})", destination);
                    }
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
                    // Preserve any existing lineage (parent/children) that may have been recorded
                    // by a process-creation event before this signup. Only update the process name:
                    // the init process and daemons are identified authoritatively by the role the
                    // kernel carries in their scheduling events, so signup never decides identity.
                    match self.processes.get_mut(&pid) {
                        Some(record) => {
                            record.name = s;
                        },
                        None => {
                            self.processes.insert(pid, ProcessRecord::new(s, None));
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

    /// Handles an exec-sync request from a freshly `exec`'d `process` whose new image must be held
    /// until the filesystem daemon has applied close-on-exec to its inherited descriptor table.
    ///
    /// Relays the close-on-exec request to the filesystem daemon and records the process as
    /// awaiting that daemon's acknowledgement, which `handle_exec_ack()` resolves. If the relay
    /// could not be dispatched, the process is released immediately with a failure acknowledgement:
    /// its image has already been replaced and the `exec` cannot be undone, so it proceeds on a
    /// best-effort basis rather than blocking forever on an acknowledgement that will never come.
    fn handle_exec_sync(&mut self, process: ProcessIdentifier) {
        ::syslog::info!("exec-sync request (process={:?})", process);
        if self.notify_exec(process) {
            // Replace any stale entry for this process before recording the new one, preserving the
            // at-most-one-pending-exec-per-process invariant across pid reuse.
            self.pending_execs.retain(|p| *p != process);
            self.pending_execs.push(process);
        } else {
            ::syslog::warn!(
                "exec close-on-exec not dispatched, failing exec-sync (process={:?})",
                process
            );
            self.release_exec_sync(process, ErrorCode::TryAgain.get());
        }
    }

    /// Notifies the filesystem daemon to apply close-on-exec to `process`'s descriptor table,
    /// mirroring how `notify_fork_clone` dispatches a fork-clone.
    ///
    /// Returns `true` if the request was handed to the kernel for delivery, and `false` if it could
    /// not be built or sent. The daemon does not block on an acknowledgement here: the filesystem
    /// daemon's acknowledgement is delivered asynchronously and resolved in `handle_exec_ack()`,
    /// so that the receive path never risks swallowing an unrelated message.
    fn notify_exec(&self, process: ProcessIdentifier) -> bool {
        match message::exec_request(ProcessIdentifier::PROCD, ProcessIdentifier::VFSD, process) {
            Ok(request) => match ::sys::kcall::ipc::__kcall_send(&request) {
                Ok(()) => true,
                Err(e) => {
                    ::syslog::warn!(
                        "notify_exec: failed to notify vfsd to apply close-on-exec (process={:?}, \
                         error={:?})",
                        process,
                        e
                    );
                    false
                },
            },
            Err(e) => {
                ::syslog::warn!(
                    "notify_exec: failed to build exec request (process={:?}, error={:?})",
                    process,
                    e
                );
                false
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
    fn handle_exec_ack(&mut self, process: ProcessIdentifier, status: i32) {
        if let Some(pos) = self.pending_execs.iter().position(|p| *p == process) {
            self.pending_execs.swap_remove(pos);
            self.release_exec_sync(process, status);
        } else {
            ::syslog::warn!(
                "ignoring exec-ack for process not awaiting the barrier (process={:?})",
                process
            );
        }
    }

    /// Releases a `process` held at the exec barrier by acknowledging it with `status`. A `0`
    /// status releases it after close-on-exec has been applied; a non-zero status releases it on a
    /// best-effort basis after the barrier could not be completed.
    fn release_exec_sync(&self, process: ProcessIdentifier, status: i32) {
        match message::exec_ack(ProcessIdentifier::PROCD, process, process, status) {
            Ok(ack) => {
                if let Err(e) = ::sys::kcall::ipc::__kcall_send(&ack) {
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
