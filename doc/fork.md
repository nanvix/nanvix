# Design: `fork()` System Call

## Status

- **State:** Proposed
- **Scope:** Standalone deployment mode only
- **Tracking issue:** [nanvix/nanvix#321](https://github.com/nanvix/nanvix/issues/321)
- **Related issue:** [nanvix/nanvix#336](https://github.com/nanvix/nanvix/issues/336) (`waitpid()`)

---

## 1. Overview

This document describes how to add support for the POSIX `fork()` system call to Nanvix.

`fork()` creates a new process (the *child*) that is a near-exact duplicate of the calling
process (the *parent*). The new functionality MUST:

1. Be available **only** in **standalone deployment mode**. In every other deployment mode
   (multi-proces and single-process container) `fork()` MUST fail cleanly with `errno`
   set to `ENOSYS`.
2. Be implemented on top of the existing **`duplicate`** kernel call
   (`KcallNumber::Duplicate`, `NR_DUPLICATE_SYSCALL = 37`).
3. Honor the **exact semantics** specified by the Open Group Base Specifications for
   [`fork()`](https://pubs.opengroup.org/onlinepubs/9799919799/functions/fork.html), within the
   limits noted in [§9 Limitations](#9-limitations).

The work spans three layers:

| Layer        | Component                                            | Change                                        |
| ------------ | --------------------------------------------------- | --------------------------------------------- |
| User C ABI   | `src/libs/syscall/src/unistd/bindings/fork.rs`      | Replace the stub with a real implementation.  |
| User runtime | `src/libs/syscall` (new `unistd/fork` trampoline)   | Context save/restore trampoline + child boot. |
| Daemon       | `src/daemons/procd`, `src/libs/proc`                | Parent/child registry + `waitpid()` support.  |

No kernel changes are required for the core duplication mechanism — the `duplicate` kernel call
already provides everything the kernel must provide. The remaining work is user-space plumbing.

---

## 2. Background

### 2.1 Current state

- `fork()` is currently a stub. `src/libs/syscall/src/unistd/bindings/fork.rs` logs
  `"fork(): not implemented"`, sets `errno` to `InvalidSysCall`, and returns `-1`.
- `waitpid()` is likewise a stub (`src/libs/syscall/src/unistd/bindings/waitpid.rs`).
- The kernel already exposes a **`duplicate`** kernel call.

### 2.2 The `duplicate` kernel call

`duplicate` is the kernel primitive this design builds upon. Its handler lives in
`src/kernel/src/pm/kcall/duplicate.rs` and the heavy lifting is in
`ProcessManager::duplicate_process` (`src/kernel/src/pm/process/manager/mod.rs`). The user-space
binding is `::sys::kcall::pm::__kcall_duplicate` (`src/libs/sys/src/sys/kcall/pm.rs`).

Behavior of `duplicate(args: &ThreadCreateArgs) -> Result<ProcessIdentifier, Error>`:

1. Reserves a fresh PID/TID for the child.
2. Clones the caller's address space into a new `Vmem` and links **all user-space pages
   copy-on-write** (`mm.link_user_pages`). Both address spaces transparently share physical
   frames until either side writes; the page-fault handler then makes a private copy for the
   faulting side.
3. Forges a kernel context for the child's main thread that, on first dispatch, enters **user
   mode at `args.user_fn`** on the **user-supplied stack** `args.user_stack_base`, with
   `args.user_fn_arg0`/`arg1` as arguments and `args.user_tda` as the thread data area.
4. Returns the **child PID to the parent**. The child never "returns" from `duplicate`; it
   *starts* at `user_fn`.

It **refuses** the request (`OperationNotPermitted`) when the caller owns any *special resource*:
memory-mapped I/O regions, port-mapped I/O ports, event ownerships, or buffered in-flight
mailbox messages. It also enforces the system-wide live-process cap
(`config::kernel::MAX_PROCESSES`, → `OutOfMemory`) and validates that `user_fn`/`user_stack`/
`user_tda` lie inside the user address space (→ `InvalidArgument`).

### 2.3 The semantic gap between `duplicate` and `fork`

`duplicate` and `fork` differ in one crucial way:

| Aspect                 | `duplicate`                                    | POSIX `fork`                                          |
| ---------------------- | ---------------------------------------------- | ----------------------------------------------------- |
| Where the child starts | At a **new entry point** (`user_fn`)           | At the **same instruction** following the `fork` call |
| Child's stack          | A **fresh** caller-provided stack              | A **copy of the parent's stack** at the call site     |
| Child's return value   | N/A (it does not "return")                     | `0`                                                   |
| Parent's return value  | Child PID                                      | Child PID                                              |

The entire user-space design below exists to **bridge this gap**: it makes the child, which the
kernel starts at a synthetic entry point, *appear* to return `0` from `fork()` at the original
call site, running on its own (copy-on-write) copy of the parent's stack.

### 2.4 The `procd` process daemon

In standalone mode the guest application runs alongside the `procd`, `memd`, and `vfsd` daemons
inside a single VM (multibinary image). `procd` (`src/daemons/procd`, library `src/libs/proc`)
already:

- Holds the `Capability::ProcessManagement` capability.
- Maintains a registry of live processes (`BTreeMap<ProcessIdentifier, (name, identity)>`) via
  `signup`/`lookup` IPC messages.
- Subscribes to the `SchedulingEvent::ProcessTermination` event and is notified, with exit
  status, whenever any process terminates.
- Today treats the termination of any **non-daemon** process as a shutdown trigger (because
  interactive standalone mode runs a single application).

`procd` is the natural home for the parent/child bookkeeping that POSIX requires but the kernel
does not track (the kernel's `duplicate_process` creates an *independent* process with no
recorded parent).

---

## 3. POSIX semantics to honor

The Open Group specification requires, among other things:

1. The child is a duplicate of the parent. It gets its **own copy of the parent's address
   space** (writes are private). — *Provided by `duplicate`'s copy-on-write clone.*
2. `fork()` returns the **child PID to the parent** and **`0` to the child**. — *Provided by the
   trampoline in [§4](#4-user-space-design).*
3. On failure, no child is created, `-1` is returned, and `errno` is set to **`EAGAIN`** or
   **`ENOMEM`**. — *See [§7](#7-error-handling).*
4. The child's **parent process ID** equals the parent's PID; `getppid()` in the child returns
   it. — *Tracked by `procd` ([§5](#5-process-relationship-tracking)).*
5. The child inherits the parent's **environment, open file descriptors** (sharing the same open
   file descriptions / offsets), current working directory, root directory, file-mode creation
   mask, signal handlers/dispositions, and resource limits.
6. The child has a **single thread** — the one that called `fork()` — even if the parent is
   multithreaded.
7. The child's set of **pending signals is empty** and its **process times** (`tms_utime`, …)
   are reset to zero.
8. File locks held by the parent are **not** inherited by the child.

The mapping of each requirement onto Nanvix is given in [§6](#6-posix-conformance-matrix).

---

## 4. User-space design

### 4.1 Strategy: a context-save/restore trampoline

Because the kernel starts the child at a synthetic entry point on a fresh stack, the user-space
`fork()` wrapper must reconstruct "resume at the call site" semantics itself. This is a
`setjmp`/`longjmp`-style maneuver, performed entirely in the parent's address space *before*
calling `duplicate` (so that the copy-on-write clone captures it):

```
pid_t fork(void) {
    // 0. Gate: only available in standalone deployment mode. Reuses the procd
    //    RegisterChild handshake as the probe (see §8); fails ENOSYS otherwise.
    if (!deployment_is_standalone()) { errno = ENOSYS; return -1; }

    // `boot_stack` must survive the save/restore: keep it volatile (or otherwise
    // force it to memory) so the CHILD reads the allocated pointer -- not a stale
    // register value -- when it resumes at step 5.
    void *volatile boot_stack = NULL;

    // 1. Snapshot the calling thread's resumable CPU context into a buffer that
    //    lives in normal (CoW-cloned) memory: callee-saved registers, the return
    //    address, and the *current* stack pointer (ESP/EBP) at the fork call site.
    fork_context_t ctx;
    if (fork_save_context(&ctx) == CHILD_RESUME) {
        // 5. Reached only in the CHILD, after fork_restore_context() below
        //    switches back to this stack. The bootstrap stack is dead now, so
        //    free it eagerly, then return 0 to the child caller.
        free_bootstrap_stack(boot_stack);
        return 0;
    }

    // 2. Allocate a small bootstrap stack for the child's main thread. The kernel
    //    requires a valid user stack for duplicate(); the child only uses it for the
    //    few instructions of the trampoline before it switches back to `ctx`'s stack.
    boot_stack = alloc_bootstrap_stack();

    // 3. Ask the kernel to duplicate us. The child will start at fork_trampoline.
    ThreadCreateArgs args = {
        .user_fn         = (vaddr)fork_trampoline,
        .user_fn_arg0    = (usize)&ctx,   // valid address in the CoW-cloned child
        .user_stack_base = (vaddr)boot_stack,
        .user_stack_size = BOOTSTRAP_STACK_SIZE,
        .user_tda        = current_thread_data_area(),  // preserve TLS
    };
    pid_t child = __kcall_duplicate(&args);   // returns child PID in the PARENT

    // 4. PARENT path. Register the child with procd and return the child PID. The
    //    parent does NOT free boot_stack: the child owns it and frees it eagerly
    //    once it resumes (step 5).
    if (child < 0) { errno = map_error(child); return -1; }
    procd_register_child(child, getpid());
    return child;
}
```

```
// Runs in the CHILD only, on `boot_stack`, started by the kernel.
noreturn void fork_trampoline(fork_context_t *ctx, usize _unused) {
    // Switch ESP/EBP back to the parent's stack pointer (now a private CoW copy in
    // the child) and restore callee-saved registers + return address, causing
    // fork_save_context() to "return" CHILD_RESUME on the child's own stack.
    fork_restore_context(ctx);   // does not return here
}
```

### 4.2 Why switching back to the saved stack is correct

`duplicate` copy-on-write clones the **entire** user address space, including the parent's call
stack. Therefore, in the child:

- The stack page(s) holding the `fork()` call frame and `ctx` exist at the **same virtual
  addresses** as in the parent and contain identical data.
- `fork_restore_context(ctx)` switches `ESP`/`EBP` to those addresses and resumes; the moment the
  child writes to that stack, copy-on-write gives it a private copy. The parent's stack is
  untouched.

The `boot_stack` is needed only to satisfy `duplicate`'s requirement for a valid, distinct user
stack and to give the trampoline a place to execute its first few instructions. After
`fork_restore_context`, the child no longer needs it: the child **frees the bootstrap stack
eagerly** in the `CHILD_RESUME` branch of `fork()` (by then it is running on the parent's
copy-on-write stack, so the bootstrap stack is dead). The `boot_stack` pointer must be forced to
memory (e.g. `volatile`) so the restored child reads the allocated value rather than a stale
register, mirroring the classic `setjmp`/`longjmp` rule. Pooling or reusing bootstrap stacks
across successive `fork()` calls is a possible future optimization but is **not** done in V1:
eager free is deterministic and leak-free.

### 4.3 Context primitives

Nanvix currently ships **no** `setjmp`/`getcontext`. Two tiny, architecture-specific assembly
routines must be added (x86, 32-bit — the only guest target):

- `fork_save_context(ctx) -> i32`: stores callee-saved registers (`EBX`, `ESI`, `EDI`, `EBP`),
  `ESP`, and the return address into `ctx`; returns a sentinel distinguishing the *direct* return
  (parent) from the *restored* return (child). Semantically equivalent to `setjmp`.
- `fork_restore_context(ctx) -> !`: loads `ESP`/`EBP`/callee-saved registers from `ctx` and jumps
  to the saved return address, forcing `fork_save_context` to return the "child" sentinel.
  Semantically equivalent to `longjmp`.

These live next to the trampoline (e.g. `src/libs/syscall/src/unistd/fork/`), are `no_std`, and
follow the i386 SysV ABI already documented in `nvx-crt0`'s `_do_start` stub. They must preserve
the i386 16-byte stack-alignment invariant.

### 4.4 TLS / thread data area

The calling thread's TDA pointer is passed through `ThreadCreateArgs.user_tda` so the child's
single thread keeps a consistent thread-local storage view. Because the address space is cloned,
the TLS block itself is duplicated copy-on-write at the same address; only the pointer needs to be
carried across.

### 4.5 Important constraint: do **not** re-run `crt0`

The child must resume at the `fork()` call site — it MUST NOT re-enter `_start`/`_do_start`
(`nvx-crt0`). The trampoline jumps straight back into application code via the saved context, so
`argv`, `environ`, the heap, and runtime initialization are inherited verbatim and never
re-initialized. This is the key reason `fork` cannot be implemented as "spawn a fresh process".

---

## 5. Process relationship tracking

POSIX requires the child to know its parent (`getppid()`) and the parent to reap the child
(`waitpid()`). The kernel's `duplicate_process` does **not** record a parent/child relationship,
so this is maintained in `procd`.

### 5.1 `procd` data-model extension

Extend the registry value to carry lineage and exit state:

```
processes: BTreeMap<ProcessIdentifier, ProcessRecord>

struct ProcessRecord {
    name: String,
    identity: Option<ProcessIdentity>,
    parent: Option<ProcessIdentifier>,   // None for the root application + daemons
    children: Vec<ProcessIdentifier>,
    zombie: Option<i32>,                 // Some(status) once terminated, awaiting reap
}
```

### 5.2 New `procd` IPC operations

Add to the `ProcessManagementMessage` protocol (`src/libs/proc/src/message/`):

| Operation        | Direction        | Purpose                                                       |
| ---------------- | ---------------- | ------------------------------------------------------------- |
| `RegisterChild`  | parent → procd   | Record `(child_pid, parent_pid)` after a successful `fork()`. |
| `GetParent`      | child → procd    | Implements `getppid()`.                                       |
| `Wait`           | parent → procd   | Implements `waitpid()`: block/poll for a child's exit status. |

`procd` already receives `ProcessTermination` events with the exit status. On such an event for a
process that has a recorded parent, `procd`:

1. Stores the status in `zombie` instead of immediately discarding the record (so a later
   `waitpid()` can retrieve it).
2. If the parent is currently blocked in a matching `Wait`, replies immediately with the PID +
   status and removes the zombie record.
3. Re-parents any surviving children of the terminated process to the root application process
   (POSIX "orphan adoption by init").

### 5.3 `waitpid()` and zombie reaping

`waitpid()` (`src/libs/syscall/src/unistd/bindings/waitpid.rs`) becomes a thin client of the new
`Wait` operation:

- `pid > 0`: wait for the specific child.
- `pid == -1`: wait for any child.
- `WNOHANG` in `options`: non-blocking poll (`procd` replies with `0` if no zombie is ready).
- On success, writes the encoded status through the `status` out-pointer and returns the child
  PID.
- Errors map to `ECHILD` (no such child / not a child of the caller) and `EINTR`.

This closes the loop for the POSIX requirement that a child's resources persist as a *zombie*
until the parent reaps it.

### 5.4 Shutdown-trigger correction

`procd` currently triggers VM shutdown when **any** non-daemon process terminates. With `fork`
there can be many non-daemon processes simultaneously. The termination handler
(`handle_process_termination_event`) must be changed so that **only the termination of the root
application process** (the first non-daemon process, i.e. the one with `parent == None`) triggers
shutdown and propagates the exit status. Termination of a forked child instead transitions that
child to the zombie/reaped path described above.

---

## 6. POSIX conformance matrix

| POSIX requirement                                  | Mechanism in Nanvix                                                                 | Status         |
| -------------------------------------------------- | ---------------------------------------------------------------------------------- | -------------- |
| Child has own copy of address space (private writes)| `duplicate` copy-on-write page linking                                             | Supported      |
| `fork` returns child PID to parent, `0` to child   | `duplicate` return value (parent) + restore trampoline (child)                     | Supported      |
| Child PID is unique                                | Kernel reserves a fresh PID in `duplicate_process`                                  | Supported      |
| `getppid()` returns parent PID                     | `procd` lineage record + `GetParent`                                                | Supported      |
| Inherit environment / argv                         | Cloned address space; child does not re-run `crt0`                                  | Supported      |
| Inherit open file descriptors / offsets            | FD table lives in `vfsd`; child inherits it via the cloned descriptor state         | See §9         |
| Inherit CWD, root, umask                           | Held in cloned user state / queried from `vfsd`                                     | See §9         |
| Single thread in child                             | `duplicate` creates exactly one main thread for the child                           | Supported      |
| Child pending-signal set is empty                  | Reset during child trampoline bring-up                                              | Supported      |
| Process times reset to zero in child               | Reset during child trampoline bring-up                                              | Supported      |
| File locks not inherited                           | Locks are tracked outside the address space (`vfsd`); not copied                    | Supported      |
| Failure → `-1`, `errno` ∈ {`EAGAIN`, `ENOMEM`}     | Error mapping in [§7](#7-error-handling)                                            | Supported      |

---

## 7. Error handling

`__kcall_duplicate` failures map to `errno` as follows:

| Kernel `ErrorCode`        | Cause                                                          | `errno`     |
| ------------------------- | ------------------------------------------------------------- | ----------- |
| `OutOfMemory`             | Live-process cap reached / clone allocation failed            | `EAGAIN`*   |
| `OperationNotPermitted`   | Caller owns special resources (mmio/pmio/events/mailbox)      | `ENOSYS`/`EAGAIN` (see §9) |
| `InvalidArgument`         | Internal argument validation failure                          | `ENOMEM`    |
| (deployment not standalone)| `fork()` invoked outside standalone mode                     | `ENOSYS`    |

\* **V1 decision:** the kernel returns `OutOfMemory` for both
the live-process cap (a resource limit) and genuine clone-allocation failures, and user space
cannot distinguish them without a kernel change. For V1 **both collapse to `EAGAIN`** — POSIX lists
it first for `fork` and it matches the dominant case (the process cap). No `duplicate` kernel-call
change is made; threading distinct codes out of `duplicate_process` to split `EAGAIN` (cap) from
`ENOMEM` (allocation) is deferred as an optional future refinement.

On any error, no child exists and the parent continues at the instruction after `fork()`.

---

## 8. Restricting to standalone deployment mode

`fork()` is compiled into the guest binary, which does not statically know its deployment mode, so
the restriction is enforced at **runtime**:

- In **standalone** mode the full process tree (application + children + daemons) shares one VM,
  `procd` is present, and the `duplicate` clone is meaningful. `fork()` is allowed.
- In **HTTP / multi-process** and **single-process container** modes each application runs in its
  own UserVM/sandbox; an in-VM `duplicate` does not fit that model. `fork()` MUST return `-1` with
  `errno = ENOSYS`.

**Chosen mechanism: `procd` capability probe.** `fork()` gates
on a `procd` `RegisterChild` handshake — it reuses the registration that a successful `fork()` must
perform anyway. If `procd` is absent or rejects the handshake (i.e. the process is not running in
standalone mode), `fork()` fails with `ENOSYS` *before* any `duplicate` is attempted. This adds no
new transport, environment variable, or kernel call.

Considered and rejected for V1:

- **Environment variable** (`NANVIX_DEPLOYMENT=standalone` injected at spawn): simple, but adds a
  separate out-of-band signal that can drift from the actual runtime topology.
- **Dedicated `procd` query / kernel-call / `capctl` probe:** redundant given that the
  `RegisterChild` handshake already establishes `procd`'s presence.

The gate MUST run **before** `duplicate`, so no orphaned child is ever created in an unsupported
mode.

---

## 9. Limitations

These are accepted deviations from full POSIX semantics for the initial implementation; each
should be documented in the `fork()` doc-comment and tracked for follow-up.

1. **Special-resource processes cannot fork.** `duplicate` refuses callers that own MMIO/PMIO
   regions, event ownerships, or in-flight mailbox messages. A process holding such resources
   gets an error rather than a successful fork. Daemons (which hold these) therefore cannot fork;
   ordinary applications normally can.
2. **Multithreaded parents.** POSIX permits forking a multithreaded process (child keeps only the
   calling thread). The first implementation targets single-threaded callers; forking while other
   threads run is unsupported until thread-quiescing is added, because the cloned address space
   could capture another thread mid-mutation. The `fork()` wrapper detects >1 live threads and
   fails with `EAGAIN` rather than corrupt state. Thread-quiescing and `pthread_atfork` support are
   **deferred to a follow-up tracking issue**.
3. **File-descriptor / CWD / umask inheritance** depends on `vfsd` exposing the descriptor table,
   working directory, and creation mask in a way the child inherits. If any of that state is held
   only in `vfsd` keyed by PID, `vfsd` must learn about the new child (via a `procd`→`vfsd`
   notification on `RegisterChild`) so the child inherits a *copy* of the parent's descriptor
   state with shared open file descriptions. This is a prerequisite for full conformance of the
   rows marked "See §9" in [§6](#6-posix-conformance-matrix).
4. **`pthread_atfork` handlers** are not invoked (no pthread fork-handler registry yet).

---

## 10. Affected components

| Path                                                        | Change                                                          |
| ----------------------------------------------------------- | -------------------------------------------------------------- |
| `src/libs/syscall/src/unistd/bindings/fork.rs`              | Real `fork()`: gate, snapshot, duplicate, register, return.    |
| `src/libs/syscall/src/unistd/fork/` (new)                   | `fork_save_context` / `fork_restore_context` / `fork_trampoline` (x86 asm + Rust glue). |
| `src/libs/syscall/src/unistd/bindings/waitpid.rs`           | Real `waitpid()` over the new `procd` `Wait` op.               |
| `src/libs/syscall/src/unistd/bindings/` (getppid, new)      | `getppid()` over the new `procd` `GetParent` op.               |
| `src/libs/proc/src/message/`                                | New `RegisterChild` / `GetParent` / `Wait` messages.           |
| `src/libs/proc/src/syscall/`                                | Client helpers for the new operations.                         |
| `src/daemons/procd/src/main.rs`                             | Lineage tracking, zombie/reap logic, shutdown-trigger fix.     |
| `src/daemons/procd` (gate)                                  | `RegisterChild` handshake doubles as the standalone-mode gate; no host-side hint injection needed (see §8). |
| `src/libs/sys` (deferred)                                   | Finer-grained `duplicate` error codes for `EAGAIN` vs `ENOMEM` — not needed for V1 (see §7). |

No change is required to the `duplicate` kernel call itself.

---

## 11. Testing strategy

Add tests following the existing patterns (`test-kernel`, integration tests, system tests). See
the `test-development` skill for harness details.

1. **Trampoline unit behavior.** Verify `fork()` returns `0` in the child and a positive PID in
   the parent; verify both continue at the call site (not at `_start`).
2. **Copy-on-write isolation.** Mirror `src/tests/test-kernel/src/duplicate.rs`: a value written
   by the parent after `fork()` is invisible to the child and vice-versa, while pre-`fork()` data
   is shared.
3. **Lineage.** Child `getppid()` equals parent `getpid()`; orphan re-parents to the root process.
4. **`waitpid()`.** Parent reaps a child's exit status; `WNOHANG` polls correctly; reaping a
   non-child yields `ECHILD`; zombie persists until reaped.
5. **Deployment gate.** In a non-standalone build/mode, `fork()` returns `-1` / `ENOSYS` and
   creates no child.
6. **Error paths.** Forking past `MAX_PROCESSES` yields `EAGAIN`; forking a special-resource owner
   fails without creating a child; multithreaded caller fails with `EAGAIN`.
7. **Shutdown correctness.** Terminating a forked child does **not** shut the VM down; terminating
   the root application process does, propagating its exit status.
