// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    identity::ProcessIdentity,
    message,
    ForkSyncMessage,
    LookupMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    SignupMessage,
    WaitMessage,
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

/// `WNOHANG` wait option (mirrors `::sysapi::sys_wait::WNOHANG`). A `waitpid()` carrying this flag
/// polls without blocking.
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
}

impl ProcessDaemon {
    /// Initializes the process manager daemon.
    pub fn init() -> Result<Self, Error> {
        ::syslog::info!("running process manager daemon...");
        let mypid: ProcessIdentifier = ::sys::kcall::pm::__kcall_getpid()?;
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
            if clone_dispatched {
                if let Some(record) = self.processes.get_mut(&child) {
                    record.fork_clone_done = true;
                }
            }
        }

        // Release a parent (and its child) that is already blocked awaiting fork synchronization.
        // Two conditions must hold before the waiter is acknowledged:
        //
        // 1. The waiter must match the kernel-attributed real parent of this child. A pending entry
        //    whose waiter differs was injected by a process that named a `child` that is not
        //    actually its own (the `child` field of a fork-sync request is untrusted): drop it
        //    without acknowledging, so it cannot inject a spurious acknowledgement into a victim's
        //    mailbox or displace the genuine waiter.
        // 2. The fork-clone must have actually been dispatched to the filesystem daemon, tracked by
        //    `fork_clone_done`. If the notification failed to send, the flag stays `false` and the
        //    waiter is left pending rather than acknowledged: releasing it would let parent and
        //    child proceed past a filesystem snapshot that was never taken, mirroring the
        //    `fork_clone_done` gating on the fork-sync fast path in `handle_fork_sync()`.
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
                // send). Leave it pending rather than acknowledge a snapshot that was never taken;
                // it is reclaimed when the child terminates.
                ::syslog::warn!(
                    "fork-clone not dispatched, leaving fork-sync waiter blocked (parent={:?}, \
                     child={:?})",
                    waiting_parent,
                    child
                );
            }
        }

        Ok(())
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

        // Drop any stale fork-sync bookkeeping and notify the filesystem daemon to reclaim the
        // process's per-process state (open file descriptors and working directory). Unlike the
        // fork-clone notification — which is skipped for kernel-spawned processes that have no
        // parent state to inherit — this is sent for every terminating process: daemons and the
        // init process accumulate their own filesystem state lazily as they open files, and that
        // state must be reclaimed too. The notification is a no-op in the filesystem daemon for a
        // process that never registered any state, so the extra message is harmless.
        self.pending_fork_syncs.retain(|(child, _)| *child != pid);
        self.notify_process_exit(pid);

        // Look up the terminating process in the registry.
        if let Some(record) = self.processes.get(&pid) {
            let name: String = record.name.clone();
            // The boot/init workload is the non-daemon process created directly by the kernel.
            // If it has signed up, its identity is recorded reliably in `init_proc`, so prefer
            // that: only its termination triggers shutdown. Otherwise — most workloads never sign
            // up — fall back to lineage: the process whose recorded parent is the kernel is the
            // init process. Requiring the parent to be the kernel (rather than merely absent)
            // prevents a forked child from spuriously triggering a system-wide shutdown.
            let is_init: bool = match self.init_proc {
                Some(init_proc) => init_proc == pid,
                None => record.parent == Some(ProcessIdentifier::KERNEL),
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

            // A forked child terminated — transition it to a zombie awaiting reap by `waitpid()`.
            // Re-parent its surviving children to the init process (orphan adoption by init), then
            // retain its exit status as a zombie so a current or future `waitpid()` can collect it.
            let parent: Option<ProcessIdentifier> = record.parent;

            self.reparent_children(pid);

            // Mark the process as a zombie, clearing its (now re-parented) children list.
            if let Some(record) = self.processes.get_mut(&pid) {
                record.children.clear();
                record.zombie = Some(status);
            }

            // Wake a parent blocked in `waitpid()` (if any), which reaps the zombie immediately. If
            // no parent is currently blocked, the zombie is retained until a future `waitpid()`.
            if let Some(parent) = parent {
                self.wake_waiter(parent, pid, status)?;
            }

            return Ok(None);
        }

        // The terminating process is unknown to the registry: its process-creation event was never
        // observed. This is always a transient forked child, never the init process, so it is
        // reaped without triggering shutdown. The init process is spawned directly by the kernel
        // before any user workload runs, and procd subscribes to scheduling events at startup, so
        // init's creation event (recorded with parent == KERNEL) is always observed long before
        // init can terminate. Init therefore always terminates as a *known* process, handled by the
        // lineage check above. An unknown process, by contrast, can only be a child that forked and
        // exited before its creation event was processed; with no `waitpid()` to reap it, the event
        // is simply ignored. Treating it as init here would spuriously shut the whole system down.
        ::syslog::info!(
            "unregistered process terminated (pid={:?}, status={:?}) — ignoring",
            pid,
            status
        );
        Ok(None)
    }

    /// Re-parents the surviving children of `pid` to the init process.
    fn reparent_children(&mut self, pid: ProcessIdentifier) {
        let init_proc: ProcessIdentifier = match self.init_proc {
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

    /// Handles a `Wait` request from `caller` selecting `(pid, options)`. Returns
    /// `Ok(Some(reply))` for an immediate reply, or `Ok(None)` when the waiter blocks (the reply is
    /// then produced later by the process-termination handler).
    fn handle_wait(
        &mut self,
        caller: ProcessIdentifier,
        message: WaitMessage,
    ) -> Result<Option<Message>, Error> {
        let pid: ProcessIdentifier = message.pid;
        let options: i32 = message.options;

        // Resolve the selector. A positive `pid` selects that specific child; every other value
        // selects any child of the caller. The POSIX process-group selectors (`pid == 0` and
        // `pid < -1`) are treated as "any child" because Nanvix has no process groups yet (see the
        // `waitpid()` limitations). Since only parent/child lineage is tracked, every eligible
        // child is already a child of the caller, so this is a harmless superset.
        let selector: WaitSelector = if i32::from(pid) > 0 {
            WaitSelector::Pid(pid)
        } else {
            WaitSelector::Any
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
            let waiter: BlockedWaiter = self.blocked.remove(index);

            // Reap the zombie before sending the deferred response.
            self.reap(parent, child);

            let reply: Message = message::wait_response(waiter.waiter, child, status, 0)?;
            ::sys::kcall::ipc::__kcall_send(&reply)?;
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
    /// and child are released immediately. Otherwise the request is recorded and they are released
    /// once the child's process-creation event dispatches the clone. Either way, the release is
    /// ordered after the fork-clone on the filesystem daemon's receive path, so neither process can
    /// race ahead of the snapshot.
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

        let cloned: bool = self
            .processes
            .get(&child)
            .map(|record| record.fork_clone_done)
            .unwrap_or(false);
        if cloned {
            self.release_fork_sync(parent, child);
        } else {
            // Replace any stale entry for this child before recording the new waiter, preserving
            // the at-most-one-waiter-per-child invariant that the map previously guaranteed.
            self.pending_fork_syncs.retain(|(c, _)| *c != child);
            self.pending_fork_syncs.push((child, parent));
        }
    }

    /// Releases a parent and its freshly forked child that are blocked awaiting fork
    /// synchronization, by acknowledging both. The fork-clone has already been dispatched to the
    /// filesystem daemon, so these acknowledgements are necessarily ordered after it.
    fn release_fork_sync(&self, parent: ProcessIdentifier, child: ProcessIdentifier) {
        for pid in [parent, child] {
            match message::fork_sync_ack(pid) {
                Ok(ack) => {
                    if let Err(e) = ::sys::kcall::ipc::__kcall_send(&ack) {
                        ::syslog::warn!(
                            "release_fork_sync: failed to acknowledge (pid={:?}, error={:?})",
                            pid,
                            e
                        );
                    }
                },
                Err(e) => {
                    ::syslog::warn!(
                        "release_fork_sync: failed to build acknowledgement (pid={:?}, error={:?})",
                        pid,
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

        // Search the registry for a process whose name matches the requested name.
        for (pid, record) in self.processes.iter() {
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
