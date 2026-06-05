# Design: `waitpid()` System Call

## Status

- **State:** Proposed
- **Scope:** Standalone deployment mode only
- **Tracking issue:** [nanvix/nanvix#336](https://github.com/nanvix/nanvix/issues/336)
- **Related issue:** [nanvix/nanvix#321](https://github.com/nanvix/nanvix/issues/321) (`fork()`)
- **Related design:** [`doc/fork.md`](./fork.md)

---

## 1. Overview

This document describes how to add support for the POSIX `waitpid()` system call to Nanvix.

`waitpid()` lets a parent process **synchronize with the state changes (termination) of its
children** and **reap** their exit status. It is the companion of `fork()`: `fork()` creates the
child, `waitpid()` retrieves the child's exit status and releases the kernel/daemon resources that
the terminated child kept alive as a *zombie*.

The new functionality MUST:

1. Be available **only** in **standalone deployment mode**. In every other deployment mode
   (HTTP / multi-process, single-process container) `waitpid()` MUST fail cleanly with `errno`
   set to `ENOSYS`. Standalone mode is the only mode where a parent and its children share a
   single VM with the `procd` daemon that brokers process lineage.
2. Be implemented as an **IPC request/response exchange with `procd`** (the process-management
   daemon), not as a new kernel call. The kernel has no notion of parent/child relationships;
   that bookkeeping lives in `procd`.
3. Honor the **semantics** specified by the Open Group Base Specifications for
   [`wait()`/`waitpid()`](https://pubs.opengroup.org/onlinepubs/9799919799/functions/wait.html),
   within the limits noted in [§11 Limitations](#11-limitations).

The work spans two layers (no kernel changes are required):

| Layer        | Component                                                | Change                                                       |
| ------------ | -------------------------------------------------------- | ------------------------------------------------------------ |
| User C ABI   | `src/libs/syscall/src/unistd/bindings/waitpid.rs`        | Replace the stub with a real, standalone-gated implementation. |
| User runtime | `src/libs/proc` (`message/`, `syscall/`)                 | New `Wait` request/response IPC + a `wait()` client helper.  |
| Daemon       | `src/daemons/procd`, `src/libs/proc/src/daemon`          | Lineage tracking, zombie retention, blocked-waiter queue, reaping. |

`procd` is the natural home for this work: it already holds the
`Capability::ProcessManagement` capability, maintains a registry of live processes, and is
subscribed to the `SchedulingEvent::ProcessTermination` event — it is *already notified, with exit
status, whenever any process terminates*.

---

## 2. Background

### 2.1 Current state

- `waitpid()` is a stub. `src/libs/syscall/src/unistd/bindings/waitpid.rs` logs
  `"waitpid(): not implemented"`, sets `errno` to `InvalidSysCall`, and returns `-1`.
- `fork()` is being implemented in parallel (see [`doc/fork.md`](./fork.md)); it introduces the
  parent/child registration that `waitpid()` consumes. This design assumes the `fork()` design's
  lineage tracking is available, and refines the `Wait` portion that `fork.md` sketches in its §5.
- The kernel's `duplicate` kernel call (the primitive behind `fork()`) creates an **independent**
  process with **no recorded parent**. There is therefore no kernel-level `wait`; the relationship
  and the zombie state must be tracked in user space by `procd`.

### 2.2 The `procd` process daemon

In standalone mode the guest application runs alongside the `procd`, `memd`, and `vfsd` daemons
inside a single VM (multibinary image). The relevant existing facts about `procd`
(`src/daemons/procd`, library `src/libs/proc`) are:

- It runs as PID `proc::PROCD` (`ProcessIdentifier::INITD`).
- It holds `Capability::ProcessManagement`.
- It maintains a registry of live processes:
  `processes: BTreeMap<ProcessIdentifier, (String /*name*/, Option<ProcessIdentity>)>`, populated
  through `signup`/`lookup` IPC (`ProcessManagementMessageHeader::Signup` / `Lookup`).
- It is subscribed to `SchedulingEvent::ProcessTermination`. Its run loop receives
  `MessageType::ProcessTerminationEvent` messages whose payload carries
  `(pid: i32, status: i32)` and dispatches them to `handle_process_termination_event`.
- **Today** it treats the termination of *any* non-daemon process as a shutdown trigger, because
  interactive standalone mode historically ran a single application. `handle_process_termination_event`
  returns `Some(status)` (causing `run()` to return and the VM to shut down) for any non-daemon
  process, and `None` only for cleanly-exiting daemons.

This last point must change for `waitpid()` (and `fork()`) to be meaningful, because with multiple
children there are many non-daemon terminations that must **not** end the VM. See
[§6.4](#64-shutdown-trigger-correction).

### 2.3 The IPC request/response pattern

`procd` clients already use a simple synchronous request/response idiom over the
`ProcessManagementMessage` envelope. For example `proc::signup`
(`src/libs/proc/src/syscall/signup.rs`):

1. Build a `ProcessManagementMessage` with a header (e.g. `Signup`) and a typed payload.
2. Wrap it in a `SystemMessage { header: ProcessManagement, .. }` and an IPC `Message` addressed to
   `proc::PROCD`.
3. `__kcall_send` it, then `__kcall_recv` and decode the matching response header (e.g.
   `SignupResponse`).

`waitpid()` follows the exact same idiom, adding a `Wait` request and a `WaitResponse` reply. The
distinguishing feature is that the *reply may be deferred* by `procd` (a blocking wait) until a
matching child terminates.

---

## 3. POSIX semantics to honor

The Open Group specification for `waitpid(pid_t pid, int *stat_loc, int options)` requires, among
other things:

1. **`pid` selection of which child(ren) to wait for:**
   - `pid > 0`  — wait for the specific child whose PID equals `pid`.
   - `pid == -1` — wait for **any** child.
   - `pid == 0` — wait for any child in the **caller's process group**.
   - `pid < -1` — wait for any child in process group `abs(pid)`.
2. **`options` flags** (bitwise OR):
   - `WNOHANG` — do not block; return `0` immediately if no eligible child has changed state.
   - `WUNTRACED` / `WCONTINUED` — report stopped / continued children (job control).
3. **Return value:** on success, the PID of the child whose status is reported; `0` if `WNOHANG`
   was set and no child was ready; `-1` with `errno` on error.
4. **Status encoding** written through `stat_loc`, inspectable via the macros `WIFEXITED`,
   `WEXITSTATUS`, `WIFSIGNALED`, `WTERMSIG`, `WIFSTOPPED`, `WSTOPSIG`, `WIFCONTINUED`.
5. **Reaping / zombies:** once a parent successfully waits on a terminated child, the child's
   record is released; a terminated-but-unwaited child remains a *zombie* so its status is not
   lost. If a process with `WNOHANG`-pollable children never reaps them, the records persist until
   the parent terminates.
6. **Orphans / re-parenting:** when a parent terminates before its children, surviving children are
   re-parented to the *init*-like process (in Nanvix, the **root application process**), which is
   then responsible for reaping them.
7. **`ECHILD`:** if the caller has **no** children matching `pid` (or the target is not a child of
   the caller), `waitpid()` fails with `ECHILD`.
8. **`EINTR`:** a blocking `waitpid()` interrupted by a signal returns `-1` / `EINTR`.
9. **`EINVAL`:** invalid `options`.

The Nanvix mapping of each requirement is given in [§9](#9-posix-conformance-matrix). The initial
implementation targets process **termination** reporting (items 1, 3, 4-exit, 5, 6, 7);
job-control stop/continue reporting (`WUNTRACED`/`WCONTINUED`) and signal-death encoding are
limited as described in [§11](#11-limitations) because Nanvix has no job control or POSIX signals
yet.

---

## 4. Architecture

```
   guest application                         procd (PID = INITD)
   ----------------                          --------------------
   waitpid(pid, &st, opts)
       |
       | 1. gate: standalone only
       |
       | 2. build Wait request  --------->  recv() in run loop
       |    (caller, pid, opts)             dispatch -> handle_wait()
       |                                         |
       |                                         | 3a. eligible zombie ready?
       |                                         |       -> reply WaitResponse(child, status)
       |                                         |          and drop the zombie record
       |                                         |
       |                                         | 3b. WNOHANG and none ready?
       |                                         |       -> reply WaitResponse(0, 0)
       |                                         |
       |                                         | 3c. has eligible live children?
       |                                         |       -> enqueue blocked waiter, no reply yet
       |                                         |
       |                                         | 3d. no eligible children at all?
       |                                         |       -> reply WaitResponse(-ECHILD)
       |                                         |
       | 4. recv() WaitResponse  <---------  (immediate for 3a/3b/3d;
       |    decode -> *stat_loc, ret             deferred for 3c, sent when the
       v                                         awaited child's termination event
   return child / 0 / -1                         arrives)
```

The key insight: a **blocking** `waitpid()` becomes a request that `procd` parks on a *blocked-waiter
queue* and answers later, when a `ProcessTermination` event for an eligible child arrives. From the
caller's perspective it is a single `send` followed by a (possibly long) `recv` — identical in shape
to the existing `signup` round-trip.

---

## 5. `procd` data model

Extend the registry value to carry lineage and exit state. Replace the current
`(String, Option<ProcessIdentity>)` tuple with a struct:

```rust
struct ProcessRecord {
    name: String,
    identity: Option<ProcessIdentity>,
    parent: Option<ProcessIdentifier>,   // None for the root application + daemons
    children: Vec<ProcessIdentifier>,    // live + zombie children
    zombie: Option<i32>,                 // Some(encoded_status) once terminated, awaiting reap
}

processes: BTreeMap<ProcessIdentifier, ProcessRecord>
```

Plus a queue of parents currently blocked in a `Wait`:

```rust
struct BlockedWaiter {
    waiter: ProcessIdentifier,   // who to reply to
    selector: WaitSelector,      // which child(ren) it is waiting for (see §7.1)
}

blocked: Vec<BlockedWaiter>      // or VecDeque, FIFO per waiter
```

Lineage is populated by the `fork()` path: after a successful `duplicate`, the parent sends a
`RegisterChild(child_pid)` request (see [`doc/fork.md` §5](./fork.md)). `procd` then:

- inserts/updates `child_pid`'s record with `parent = Some(parent_pid)`, and
- appends `child_pid` to `parent_pid`'s `children`.

(If `fork()` lands first, this design only *adds* the `Wait` operation and the zombie/blocked-waiter
logic on top of the lineage tracking that `fork()` introduced.)

---

## 6. `procd` behavior

### 6.1 Handling a `Wait` request

On receiving a `Wait` request from `caller` with `(pid, options)`:

1. **Resolve the selector** from `pid` (see [§7.1](#71-the-pid-selector)).
2. **Enumerate eligible children** of `caller` matching the selector among
   `processes[caller].children`.
   - If there are **no** eligible children (none match, or `caller` has none) →
     reply `WaitResponse` with error `ECHILD`.
3. **Look for a ready zombie** among the eligible children (a child whose `zombie` is `Some`).
   - If found, pick one (lowest PID for determinism), reply `WaitResponse(child_pid, status)`,
     then **reap**: remove the child from `caller.children` and remove its `ProcessRecord` from
     `processes`.
4. **No ready zombie:**
   - If `options & WNOHANG` → reply `WaitResponse(0, 0)` immediately (non-blocking poll, nothing
     ready).
   - Otherwise → **block**: push `BlockedWaiter { waiter: caller, selector }` onto `blocked` and
     send **no** reply yet. The reply is produced later by the termination handler
     ([§6.2](#62-handling-a-process-termination-event)).

### 6.2 Handling a process-termination event

`handle_process_termination_event` already decodes `(pid, status)`. Extend it so that, for a process
with a recorded `parent`:

1. **Mark zombie:** set `processes[pid].zombie = Some(status)` instead of immediately removing the
   record (so a later or pending `waitpid()` can retrieve it).
2. **Re-parent survivors:** for each live child `c` of the terminating `pid`, set
   `processes[c].parent = Some(ROOT_APP)` and move `c` into the root application's `children`
   (POSIX "orphan adoption by init"; the root application process is the first non-daemon process,
   i.e. the one with `parent == None`).
3. **Wake a blocked waiter:** scan `blocked` for a `BlockedWaiter` whose `waiter == parent` and
   whose `selector` matches `pid`. If one exists:
   - send `WaitResponse(pid, status)` to that waiter,
   - remove the waiter from `blocked`, and
   - **reap** `pid` (remove it from the parent's `children` and from `processes`).

If no blocked waiter matched, the record simply stays as a zombie until a future `waitpid()` reaps
it (step 3 of [§6.1](#61-handling-a-wait-request)).

### 6.3 Daemon and root-application termination

- **Daemon termination** keeps today's behavior: a clean daemon exit is ignored; a non-zero daemon
  exit triggers shutdown (it indicates a critical failure).
- **Root-application termination** triggers VM shutdown and propagates the root app's exit status,
  exactly as today — but the trigger condition is tightened (next section).

### 6.4 Shutdown-trigger correction

`procd` currently triggers shutdown when **any** non-daemon process terminates. With `fork()`/
`waitpid()` there can be many non-daemon processes alive simultaneously, so this is changed:

- **Only** the termination of the **root application process** (`parent == None`, non-daemon)
  triggers shutdown and propagates the exit status (`run()` returns `Some(status)`).
- Termination of a **forked child** (`parent == Some(_)`) instead follows the zombie/reap path of
  [§6.2](#62-handling-a-process-termination-event) and returns `None` (keep running).

This is the single most important `procd` change required to make `waitpid()` usable.

---

## 7. User-space `waitpid()` design

`waitpid()` (`src/libs/syscall/src/unistd/bindings/waitpid.rs`) becomes a thin, standalone-gated
client of the new `proc::wait()` helper.

```rust
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn waitpid(pid: pid_t, status: *mut c_int, options: c_int) -> pid_t {
    // 0. Gate: only available in standalone deployment mode.
    #[cfg(not(feature = "standalone"))]
    { *__errno_location() = ErrorCode::FunctionNotImplemented.get(); return -1; }  // ENOSYS

    #[cfg(feature = "standalone")]
    {
        // 1. Validate options.
        if options & !(WNOHANG | WUNTRACED | WCONTINUED) != 0 {
            *__errno_location() = ErrorCode::InvalidArgument.get(); return -1;     // EINVAL
        }

        // 2. Round-trip to procd.
        match ::proc::wait(ProcessIdentifier::from(pid), options) {
            // Child reaped: write encoded status (if caller asked for it) and return its PID.
            Ok(WaitOutcome::Reaped { child, encoded }) => {
                if !status.is_null() { *status = encoded; }
                child.into()
            }
            // WNOHANG, nothing ready.
            Ok(WaitOutcome::NoneReady) => 0,
            // ECHILD / EINTR / ... mapped to errno.
            Err(e) => { *__errno_location() = e.code.get(); -1 }
        }
    }
}
```

### 7.1 The `pid` selector

`proc::wait()` maps the `pid` argument to a `WaitSelector` that travels in the `Wait` request and is
re-evaluated by `procd`:

| `pid`     | Selector                         | V1 support                                    |
| --------- | -------------------------------- | --------------------------------------------- |
| `> 0`     | `Pid(pid)` — that exact child    | Supported                                     |
| `== -1`   | `Any` — any child                | Supported                                     |
| `== 0`    | `Group(caller_pgid)`             | See [§11](#11-limitations) (no process groups)|
| `< -1`    | `Group(abs(pid))`                | See [§11](#11-limitations) (no process groups)|

Until process groups exist in Nanvix, the group selectors are accepted but treated as `Any`
restricted to the caller's children (documented deviation), or rejected with `EINVAL` — chosen in
[§13](#13-open-questions).

### 7.2 Status encoding and inspection macros

`procd` reports the kernel-provided `status: i32` (the value passed to `__kcall_exit`). The C ABI
must expose it through the standard wait macros, so a status-encoding convention is fixed in
`sysapi` (matching the common Linux layout so existing C code works):

- Normal exit: `encoded = (exit_code & 0xff) << 8`. `WIFEXITED(encoded) == true`,
  `WEXITSTATUS(encoded) == exit_code & 0xff`.
- Signal death: low 7 bits hold the terminating signal; `WIFSIGNALED`/`WTERMSIG` decode it.
  *Not produced in V1* (Nanvix has no POSIX signals — see [§11](#11-limitations)).

Add to `sysapi` (e.g. `src/libs/sysapi/src/sys/wait.rs`, exported to the C headers):

```rust
pub const WNOHANG:    c_int = 1;
pub const WUNTRACED:  c_int = 2;
pub const WCONTINUED: c_int = 8;

// Macro-equivalent helpers (also emitted as C macros in the public headers).
pub const fn wifexited(s: c_int) -> bool { (s & 0x7f) == 0 }
pub const fn wexitstatus(s: c_int) -> c_int { (s >> 8) & 0xff }
pub const fn wifsignaled(s: c_int) -> bool { ((((s & 0x7f) + 1) >> 1) as i8) > 0 }
pub const fn wtermsig(s: c_int) -> c_int { s & 0x7f }
```

`wait(int *stat_loc)` is then a trivial wrapper: `waitpid(-1, stat_loc, 0)`.

### 7.3 Blocking semantics

A blocking `waitpid()` is implemented purely by the *deferred reply* from `procd`: the client sends
the `Wait` request and calls `__kcall_recv`, which blocks the calling thread until `procd` sends the
`WaitResponse`. No spinning or polling is needed. `WNOHANG` short-circuits this by having `procd`
reply immediately.

---

## 8. IPC message formats

Add a `Wait` request and a `WaitResponse` reply to the process-management protocol
(`src/libs/proc/src/message/`), mirroring the existing `Signup`/`Lookup` message pairs.

### 8.1 Protocol header additions

Extend `ProcessManagementMessageHeader` (`src/libs/proc/src/message/mod.rs`):

| Variant             | Value | Direction      | Purpose                                            |
| ------------------- | ----- | -------------- | -------------------------------------------------- |
| `Wait`              | 6     | parent → procd | Request to wait/reap a child.                      |
| `WaitResponse`      | 7     | procd → parent | Carries `(child_pid, encoded_status, error)`.      |
| `RegisterChild`     | 8     | parent → procd | (From `fork()` design) record `(child, parent)`.   |
| `RegisterChildResp` | 9     | procd → parent | Ack for `RegisterChild`.                           |

(`RegisterChild*` are listed for completeness; they are owned by the `fork()` design and reused here.
Update the `TryFrom<u8>` / `From<&_> for u8` conversions accordingly.)

### 8.2 `WaitMessage` (request)

```rust
#[repr(C, packed)]
pub struct WaitMessage {
    pub pid: ProcessIdentifier,   // raw selector as passed by the caller
    pub options: i32,             // WNOHANG | WUNTRACED | WCONTINUED
    _padding: [u8; PADDING_SIZE],
}
```

### 8.3 `WaitResponseMessage` (reply)

```rust
#[repr(C, packed)]
pub struct WaitResponseMessage {
    pub child: ProcessIdentifier, // reaped child PID, or 0 for WNOHANG-none, or i32::MAX on error
    pub status: i32,              // encoded exit status (valid only when child > 0)
    pub error: i32,               // 0 on success, else ErrorCode (ECHILD / EINVAL / ...)
    _padding: [u8; PADDING_SIZE],
}
```

Both must satisfy `assert_eq_size!(.., ProcessManagementMessage::PAYLOAD_SIZE)` like every other
message in this module, and provide `from_bytes`/`into_bytes` transmute helpers plus
`wait_request(..)` / `wait_response(..)` builders (cf. `signup_request`/`signup_response`).

### 8.4 Client helper

Add `proc::wait()` (`src/libs/proc/src/syscall/wait.rs`, exported from `lib.rs` behind the
`syscall` feature), structured exactly like `proc::signup`: build the `Wait` request, `__kcall_send`
it, `__kcall_recv` the `WaitResponse`, and translate it into a `Result<WaitOutcome, Error>`.

---

## 9. POSIX conformance matrix

| POSIX requirement                                          | Mechanism in Nanvix                                             | Status                |
| --------------------------------------------------------- | -------------------------------------------------------------- | --------------------- |
| `pid > 0` waits for a specific child                      | `WaitSelector::Pid` filter in `procd`                          | Supported             |
| `pid == -1` waits for any child                           | `WaitSelector::Any` over `caller.children`                     | Supported             |
| `pid == 0` / `pid < -1` (process groups)                  | No process groups yet                                          | See §11               |
| Returns reaped child PID / `0` (WNOHANG) / `-1`           | `WaitResponse` decoded by the `waitpid()` wrapper              | Supported             |
| `WNOHANG` non-blocking poll                               | `procd` replies immediately when nothing ready                | Supported             |
| Blocking until a child changes state                      | Deferred `WaitResponse` from `procd` blocked-waiter queue      | Supported             |
| `WIFEXITED` / `WEXITSTATUS` on a normally-exited child    | Status encoding in §7.2                                        | Supported             |
| `WIFSIGNALED` / `WTERMSIG` (signal death)                 | No POSIX signals yet                                           | See §11               |
| `WUNTRACED` / `WCONTINUED` (job control)                  | No job control / stop-continue yet                             | See §11               |
| Zombie retained until reaped                              | `ProcessRecord.zombie` retention in `procd`                    | Supported             |
| Orphans re-parented to init (root app)                    | Re-parent step in termination handler (§6.2)                   | Supported             |
| No matching child → `ECHILD`                              | `procd` checks `caller.children`                               | Supported             |
| Interrupted blocking wait → `EINTR`                       | Depends on signal delivery to a blocked `recv`                 | See §11               |
| Invalid `options` → `EINVAL`                              | Validated in the `waitpid()` wrapper                           | Supported             |
| Non-standalone mode → `ENOSYS`                            | Compile-time `standalone` feature gate                         | Supported             |

---

## 10. Error handling

| Condition                                                    | `errno`     | Where detected           |
| ------------------------------------------------------------ | ----------- | ------------------------ |
| Caller has no child matching `pid`                           | `ECHILD`    | `procd` (`handle_wait`)  |
| Invalid bits in `options`                                    | `EINVAL`    | `waitpid()` wrapper      |
| Blocking wait interrupted by a signal                        | `EINTR`     | client `recv` (see §11)  |
| Invoked outside standalone deployment mode                   | `ENOSYS`    | compile-time gate        |

`procd` reports failures by setting `WaitResponseMessage.error` to the relevant `ErrorCode`; the
client maps that to `errno`. On any error, `*stat_loc` is left unmodified and `-1` is returned (or
`0` for the `WNOHANG`-nothing-ready case, which is **not** an error).

---

## 11. Restricting to standalone deployment mode

Like the other process-management and IPC-to-daemon syscalls in `src/libs/syscall` (`pipe`, `poll`,
`select`, `mount`, …), `waitpid()` is gated at **compile time** by the `standalone` Cargo feature of
the `syscall` crate:

- `#[cfg(feature = "standalone")]` — full implementation: gate passes, talk to `procd`.
- `#[cfg(not(feature = "standalone"))]` — return `-1` with `errno = ENOSYS`
  (`ErrorCode::FunctionNotImplemented`).

This matches how the runtime already distinguishes modes (the `standalone` feature in `nanvixd`,
`src/utils/nanvixd`) and guarantees that in HTTP / multi-process and single-process container builds
— where each application runs in its own UserVM and there is no in-VM `procd` lineage to query —
`waitpid()` fails cleanly and creates no inconsistency.

---

## 12. Limitations

Accepted deviations from full POSIX semantics for the initial implementation; each should be noted
in the `waitpid()` doc-comment and tracked for follow-up.

1. **No process groups.** `pid == 0` and `pid < -1` cannot select by process group because Nanvix
   has no `setpgid`/process-group concept yet. They are either treated as "any child of the caller"
   or rejected with `EINVAL` (decided in [§13](#13-open-questions)).
2. **No job control.** `WUNTRACED` and `WCONTINUED` have no effect because Nanvix processes cannot be
   stopped/continued. Only *termination* is reported. The flags are accepted (not `EINVAL`) for
   source compatibility.
3. **No signal-death encoding.** Because Nanvix lacks POSIX signals, `WIFSIGNALED`/`WTERMSIG` never
   report true in V1; all reaped children appear as normal exits via `WIFEXITED`/`WEXITSTATUS`.
4. **`EINTR` depends on signals.** Until signal delivery can interrupt a blocked `__kcall_recv`, a
   blocking `waitpid()` cannot return `EINTR`; it blocks until a child terminates.
5. **Single waiter per child wake.** When multiple threads/parents could match a termination, `procd`
   wakes exactly one FIFO waiter (POSIX allows any one to be chosen). Multithreaded `waitpid()`
   correctness tracks the multithreading limitations of `fork()` (see `doc/fork.md` §9).
6. **Resource accounting (`rusage`).** `wait3`/`wait4`-style resource usage is out of scope.

---

## 13. Affected components

| Path                                                       | Change                                                           |
| ---------------------------------------------------------- | --------------------------------------------------------------- |
| `src/libs/syscall/src/unistd/bindings/waitpid.rs`          | Real `waitpid()`: gate, validate, round-trip to `procd`.        |
| `src/libs/syscall/src/unistd/bindings/` (`wait`, new)      | `wait(stat_loc)` as `waitpid(-1, stat_loc, 0)`.                 |
| `src/libs/sysapi/src/sys/wait.rs` (new)                    | `WNOHANG`/`WUNTRACED`/`WCONTINUED` + `W*` status macros.        |
| `src/libs/proc/src/message/mod.rs`                         | `Wait` / `WaitResponse` (+ `RegisterChild*`) header variants.  |
| `src/libs/proc/src/message/wait.rs` (new)                  | `WaitMessage` / `WaitResponseMessage` + builders.              |
| `src/libs/proc/src/syscall/wait.rs` (new)                  | `proc::wait()` client helper.                                  |
| `src/libs/proc/src/lib.rs`                                 | Export the new message types and `wait` helper.                |
| `src/libs/proc/src/daemon/mod.rs`                          | `ProcessRecord`, blocked-waiter queue, `handle_wait`, zombie/reap, re-parent, shutdown-trigger fix. |

No change is required to the kernel.

---

## 14. Testing strategy

Add tests following the existing patterns (`test-kernel`, integration tests, system tests). See the
`test-development` skill for harness details.

1. **Basic reap.** Parent `fork()`s a child that `exit(N)`s; `waitpid(child, &st, 0)` returns
   `child` and `WEXITSTATUS(st) == N`.
2. **Wait-for-any.** `waitpid(-1, &st, 0)` reaps an arbitrary child; repeated calls drain all
   children, then return `ECHILD`.
3. **`WNOHANG` poll.** Before the child exits, `waitpid(child, &st, WNOHANG)` returns `0`; after it
   exits, the same call returns `child`.
4. **Zombie persistence.** A child that exits before the parent waits is retained; a later
   `waitpid()` still retrieves its status (status is not lost).
5. **Blocking.** A parent that `waitpid()`s before the child exits blocks and is woken with the
   correct PID/status when the child terminates.
6. **`ECHILD`.** `waitpid()` with no children, or for a PID that is not the caller's child, returns
   `-1` / `ECHILD`.
7. **`EINVAL`.** Invalid `options` bits return `-1` / `EINVAL`.
8. **Orphan re-parenting.** A grandchild whose parent dies is re-parented to the root application,
   which can then reap it.
9. **Shutdown correctness.** Reaping (or merely terminating) a forked child does **not** shut the VM
   down; terminating the root application process does, propagating its exit status.
10. **Deployment gate.** In a non-standalone build, `waitpid()` returns `-1` / `ENOSYS`.

---

## 15. Open questions

1. Should `pid == 0` / `pid < -1` (process-group waits) be accepted-as-`Any` or rejected with
   `EINVAL` until process groups exist?
2. Where exactly should the status-encoding convention live so both the Rust `W*` helpers and the
   emitted C macros stay in sync (`sysapi` vs. generated headers)?
3. Should `procd` cap the number of retained zombies / blocked waiters per parent to bound memory,
   and if so, what is the policy when the cap is hit?
4. How should a blocking `waitpid()` be unblocked if the *waiting* parent itself is asked to shut
   down (VM teardown) before any child exits — is a synthetic `EINTR`/`ECHILD` reply from `procd`
   during shutdown sufficient?
5. Long-term: once POSIX signals and job control land, extend `WaitResponse` to carry stop/continue
   and signal-death status without breaking the wire format.
