// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Per-process descriptor and working-directory state.

//==================================================================================================
// Imports
//==================================================================================================

use crate::descriptor::{
    FdFlags,
    VfsFileHandle,
};
use ::alloc::{
    collections::BTreeMap,
    string::String,
    sync::Arc,
};
use ::core::sync::atomic::{
    AtomicBool,
    AtomicI32,
    Ordering,
};
use ::spin::Mutex;
use ::sys::pm::ProcessIdentifier;
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Working directory assigned to a process that has no recorded directory of its own.
pub(crate) const DEFAULT_CWD: &str = "/";

//==================================================================================================
// Structures
//==================================================================================================

/// An open file description managed by the VFS.
pub(crate) struct VfsEntry {
    /// The file handle from any backend.
    pub(crate) handle: VfsFileHandle,
    /// POSIX-compliant virtual file position.
    pub(crate) virtual_pos: off_t,
    /// Mutable open file status flags.
    pub(crate) status_flags: c_int,
}

// SAFETY: all access to an entry is serialized by its `spin::Mutex`.
unsafe impl Send for VfsEntry {}

/// Shared open file description.
pub(crate) type OpenFile = Arc<Mutex<VfsEntry>>;

/// A single file-descriptor slot.
#[derive(Clone)]
pub(crate) struct Slot {
    /// Shared open file description.
    pub(crate) file: OpenFile,
    /// Per-descriptor flags.
    pub(crate) fd_flags: FdFlags,
}

impl Slot {
    /// Creates a slot with default descriptor flags.
    pub(crate) fn new(file: OpenFile) -> Self {
        Self {
            file,
            fd_flags: FdFlags::default(),
        }
    }
}

/// Per-process descriptor and working-directory state.
pub(crate) struct ProcessState {
    /// File descriptor slots keyed by descriptor number.
    pub(crate) slots: BTreeMap<c_int, Slot>,
    /// Current working directory.
    pub(crate) cwd: String,
    /// Whether this state is active rather than a lazy placeholder.
    pub(crate) initialized: bool,
    /// Descriptor-table coherence generation.
    pub(crate) generation: u64,
}

impl ProcessState {
    /// Creates an uninitialized placeholder state.
    pub(crate) fn new() -> Self {
        Self {
            slots: BTreeMap::new(),
            cwd: String::from(DEFAULT_CWD),
            initialized: false,
            generation: 0,
        }
    }

    /// Creates the state inherited by a freshly forked child.
    pub(crate) fn fork(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            cwd: self.cwd.clone(),
            initialized: true,
            generation: self.generation,
        }
    }

    /// Advances the descriptor-table coherence generation.
    pub(crate) fn bump_generation(&mut self) {
        self.generation = self.generation.saturating_add(1);
    }
}

//==================================================================================================
// Global State
//==================================================================================================

/// Registry of per-process VFS state.
pub(crate) static PROCESSES: Mutex<BTreeMap<ProcessIdentifier, ProcessState>> =
    Mutex::new(BTreeMap::new());

/// Process on whose behalf the VFS is currently operating.
pub(crate) static CURRENT_PID: AtomicI32 = AtomicI32::new(0);

/// Tracks whether the root process's console descriptors have been seeded.
pub(crate) static ROOT_CONSOLE_SEEDED: AtomicBool = AtomicBool::new(false);

//==================================================================================================
// Functions
//==================================================================================================

/// Returns the selected process identifier.
#[inline]
pub(crate) fn current_pid() -> ProcessIdentifier {
    ProcessIdentifier::from(CURRENT_PID.load(Ordering::Relaxed))
}

/// Returns the root/init process identifier.
pub(crate) fn root_process_identifier() -> ProcessIdentifier {
    ProcessIdentifier::INIT
}

/// Selects the process on whose behalf subsequent VFS operations are performed.
pub fn set_current_process(pid: ProcessIdentifier) {
    {
        let mut processes = PROCESSES.lock();
        processes.entry(pid).or_insert_with(ProcessState::new);
    }
    CURRENT_PID.store(i32::from(pid), Ordering::Relaxed);
}

/// Returns the selected process's current working directory.
pub(crate) fn current_cwd() -> String {
    let processes = PROCESSES.lock();
    processes
        .get(&current_pid())
        .map(|state| state.cwd.clone())
        .unwrap_or_else(|| String::from(DEFAULT_CWD))
}

/// Records the selected process's current working directory.
pub(crate) fn set_current_cwd(cwd: String) {
    let mut processes = PROCESSES.lock();
    processes
        .entry(current_pid())
        .or_insert_with(ProcessState::new)
        .cwd = cwd;
}
