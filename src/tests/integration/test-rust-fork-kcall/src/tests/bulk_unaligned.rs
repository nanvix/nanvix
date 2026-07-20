// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Un-aligned Push/Pull Buffer Regression Tests
//!
//! Exercises the rendezvous `push()`/`pull()` IPC kernel calls with deliberately **un-aligned**
//! user buffers that straddle a page boundary, covering the two transfer paths that behave
//! differently for such buffers:
//!
//! 1. **User-to-user rendezvous** (`do_push`/`do_pull` → `Vmem::copy_user_to_user`). This path is
//!    expected to transfer the payload byte-for-byte even when the source and destination buffers
//!    are un-aligned and cross page boundaries at *different* intra-page offsets, because the copy
//!    is split at the nearest page boundary on each side and resolved through physical frames.
//!    [`test_rendezvous_unaligned_roundtrip`] forks a child, hands the two halves of a rendezvous
//!    to the parent and child, and verifies the received bytes against a known pattern.
//!
//! 2. **Kernel-peer bulk transfer** (`push()`/`pull()` with [`ProcessIdentifier::KERNEL`]). This
//!    path hands the buffer to the VMM as a scatter/gather list of guest-physical segments.
//!    Because a user buffer is only *virtually* contiguous, a buffer that crosses a page boundary
//!    spans multiple physical frames, so the kernel describes each physically-contiguous run as
//!    its own segment. [`test_kernel_bulk_push_page_crossing`] pushes a page-crossing buffer to
//!    the kernel and confirms the call succeeds — it was previously rejected with an
//!    `InvalidArgument` error because only the first page was translated.
//!
//! `push()` does not wait for a reply, so the kernel-peer test is deterministic even though no
//! Linux daemon completes the transfer.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    ptr,
    slice,
    sync::atomic::{
        AtomicU32,
        Ordering,
    },
};
use ::sys::{
    error::Error,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    kcall::{
        fork,
        ipc,
        pm,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Ordering used for all atomic operations.
const ORDER: Ordering = Ordering::SeqCst;

/// Size of a memory page on the supported guest architectures (x86 and x86-64 both use 4 KiB
/// pages). The buffers below are laid out relative to this value to straddle a page boundary.
const PAGE_SIZE: usize = 4096;

/// Size of each page-aligned scratch arena: two pages, large enough to host a buffer that starts in
/// the first page and ends in the second.
const ARENA_SIZE: usize = 2 * PAGE_SIZE;

/// Number of payload bytes transferred by the rendezvous round-trip. Larger than the slack on
/// either side of the page boundary so the transfer is guaranteed to cross it.
const PAYLOAD_LEN: usize = 300;

/// Intra-arena offset of the source buffer. Chosen so the buffer crosses the page boundary 100
/// bytes from its start.
const SRC_OFFSET: usize = PAGE_SIZE - 100;

/// Intra-arena offset of the destination buffer. Deliberately different from [`SRC_OFFSET`] so the
/// source and destination cross the page boundary at *different* points (37 bytes from the start),
/// exercising the page-splitting logic in `copy_user_to_user`.
const DST_OFFSET: usize = PAGE_SIZE - 37;

/// Exit status reported by the child when its half of the rendezvous succeeded.
const CHILD_EXIT_OK: i32 = 0;

/// Exit status reported by the child when its half of the rendezvous failed.
const CHILD_EXIT_FAIL: i32 = 1;

//==================================================================================================
// Global State
//==================================================================================================

/// Page-aligned arena backing the rendezvous **source** buffer (written and pushed by the child).
#[repr(C, align(4096))]
struct Arena([u8; ARENA_SIZE]);

/// Source arena. Inherited copy-on-write by the child, which fills its straddling region and pushes
/// it to the parent.
static mut SRC_ARENA: Arena = Arena([0u8; ARENA_SIZE]);

/// Destination arena. Owned by the parent, which pulls the child's payload into its straddling
/// region.
static mut DST_ARENA: Arena = Arena([0u8; ARENA_SIZE]);

/// Parent process identifier, recorded before `fork()` so the child can recover it from
/// copy-on-write memory.
static PARENT_PID_RAW: AtomicU32 = AtomicU32::new(0);

/// Parent main-thread identifier, recorded before `fork()` so the child can address its `push()` at
/// the parent's pulling thread.
static PARENT_TID_RAW: AtomicU32 = AtomicU32::new(0);

//==================================================================================================
// Buffer Helpers
//==================================================================================================

/// Computes the deterministic payload byte for index `i`.
///
/// The sequence has period 256, which is irrelevant here because verification is performed
/// index-by-index; the multiply/add merely makes a stuck-at or off-by-one corruption visible.
fn pattern_byte(i: usize) -> u8 {
    let low: u8 = u8::try_from(i & 0xFF).unwrap_or(0);
    low.wrapping_mul(167).wrapping_add(13)
}

/// Returns a mutable view of the [`PAYLOAD_LEN`]-byte region at `offset` within the arena rooted at
/// `arena`.
///
/// # Safety
///
/// The guest is single-threaded at every call site below, so the returned slice is the only live
/// reference to the region for its lifetime. The pointer is derived through [`ptr::addr_of_mut!`]
/// (never by taking a reference to the `static mut`), which keeps the access sound.
unsafe fn arena_region(arena: *mut Arena, offset: usize) -> &'static mut [u8] {
    // SAFETY: `arena` points at a live `Arena`; `offset + PAYLOAD_LEN <= ARENA_SIZE`; the byte
    // pointer stays within the arena allocation.
    unsafe {
        let base: *mut u8 = ptr::addr_of_mut!((*arena).0).cast::<u8>();
        // The arena is declared `#[repr(align(4096))]`; confirm the linker honored it so the
        // intra-arena offset doubles as the intra-page offset used by `assert_straddles_page`.
        assert!(base.align_offset(PAGE_SIZE) == 0, "arena must be page-aligned");
        // Bound the region to the arena so the slice can never run past the allocation, even if the
        // offset/length constants are adjusted later.
        assert!(
            offset + PAYLOAD_LEN <= ARENA_SIZE,
            "region must lie within the arena (offset={offset}, len={PAYLOAD_LEN}, \
             arena={ARENA_SIZE})"
        );
        let region: *mut u8 = base.add(offset);
        slice::from_raw_parts_mut(region, PAYLOAD_LEN)
    }
}

/// Asserts that a region placed at `offset` within a page-aligned arena is un-aligned and crosses
/// exactly one page boundary, documenting the precondition the rest of the test relies on. Working
/// from the offset avoids converting a pointer to an integer.
fn assert_straddles_page(offset: usize, len: usize) {
    let page_offset: usize = offset & (PAGE_SIZE - 1);
    assert!(page_offset != 0, "buffer must be page-unaligned (page_offset={page_offset})");
    assert!(
        page_offset + len > PAGE_SIZE,
        "buffer must cross a page boundary (page_offset={page_offset}, len={len})"
    );
    assert!(
        page_offset + len <= 2 * PAGE_SIZE,
        "buffer must cross exactly one page boundary (page_offset={page_offset}, len={len})"
    );
}

//==================================================================================================
// Kernel-Peer Bulk Transfer
//==================================================================================================

/// Verifies that a page-crossing buffer is accepted on the kernel-peer bulk transfer path.
///
/// A `push()` whose peer is [`ProcessIdentifier::KERNEL`] is serviced via the vmbus scatter/gather
/// data chunk transfer, which hands the VMM a list of guest-physical segments. A buffer that
/// crosses a page boundary is only *virtually* contiguous, so the kernel describes it as one
/// segment per physically-contiguous run. On the unfixed kernel the buffer was rejected with an
/// `InvalidArgument` error because only its first page was translated, so this call is the
/// regression trigger: it fails on the unfixed kernel and succeeds once the buffer is described
/// as scatter/gather segments.
///
/// `pull()` is intentionally not exercised here: it would block waiting for a completion that no
/// Linux daemon supplies. `push()` does not wait, so it stays deterministic.
fn test_kernel_bulk_push_page_crossing() -> Result<(), Error> {
    // SAFETY: single-threaded; the slice is the only live reference to the source region.
    let buffer: &mut [u8] = unsafe { arena_region(ptr::addr_of_mut!(SRC_ARENA), SRC_OFFSET) };

    // Fill the page-straddling buffer with a recognizable pattern and confirm it really does cross
    // a page boundary — the exact shape the kernel-peer bulk path maps to scatter/gather segments.
    for (i, byte) in buffer.iter_mut().enumerate() {
        *byte = pattern_byte(i);
    }
    assert_straddles_page(SRC_OFFSET, PAYLOAD_LEN);

    // push(): writing a page-crossing buffer to the kernel must succeed. On the unfixed kernel this
    // returns InvalidArgument (the VMM could translate only the first page); with scatter/gather
    // segment staging in place it must be accepted and complete without blocking.
    let push_result: Result<(), Error> =
        ipc::__kcall_push(ProcessIdentifier::KERNEL, ThreadIdentifier::KERNEL, buffer);
    assert!(
        push_result.is_ok(),
        "push() to the kernel with a page-crossing buffer must succeed once it is described as \
         scatter/gather segments (got {push_result:?})"
    );

    Ok(())
}

//==================================================================================================
// Rendezvous Round-Trip
//==================================================================================================

/// Child half of the rendezvous round-trip: fills the un-aligned source buffer with the known
/// pattern and pushes it to the parent's pulling thread.
///
/// The child first reports its own thread identifier to the parent over a regular IPC message (sent
/// under its real identity so the kernel's source-spoofing check accepts it), then issues the
/// `push()`. The parent's matching `pull()` completes the cross-process copy.
fn run_child_push() -> Result<(), Error> {
    let child_pid: ProcessIdentifier = pm::getpid_uncached()?;
    let child_tid: ThreadIdentifier = pm::__kcall_gettid()?;

    // Recover the parent identifiers stashed before fork() through copy-on-write memory.
    let parent_pid: ProcessIdentifier = ProcessIdentifier::try_from(PARENT_PID_RAW.load(ORDER))?;
    let parent_tid: ThreadIdentifier = ThreadIdentifier::try_from(PARENT_TID_RAW.load(ORDER))?;

    // Report the child's thread identifier so the parent can target its pull() at this thread.
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0..4].copy_from_slice(&u32::try_from(child_tid)?.to_le_bytes());
    let announce: Message = Message::new(
        MessageSender::new(child_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    ipc::__kcall_send(&announce)?;

    // Fill the un-aligned, page-crossing source region with the known pattern. Writing every byte
    // also takes a private copy-on-write copy of both backing pages.
    // SAFETY: single-threaded; the slice is the only live reference to the source region.
    let source: &mut [u8] = unsafe { arena_region(ptr::addr_of_mut!(SRC_ARENA), SRC_OFFSET) };
    for (i, byte) in source.iter_mut().enumerate() {
        *byte = pattern_byte(i);
    }

    // Push the payload to the parent's pulling thread. The rendezvous blocks until the parent's
    // matching pull() arrives, at which point the kernel performs the cross-process copy.
    ipc::__kcall_push(parent_pid, parent_tid, source)?;

    Ok(())
}

/// Verifies that an un-aligned, page-crossing payload survives a user-to-user rendezvous
/// `push()`/`pull()` byte-for-byte.
///
/// The parent records its identifiers, forks, and pulls the child's payload into its own un-aligned
/// destination buffer — whose page boundary falls at a different intra-page offset than the source.
/// A correct `copy_user_to_user` splits the transfer at the nearest boundary on each side; any
/// regression that mishandles the misaligned split corrupts or truncates the payload and trips the
/// assertions below.
fn test_rendezvous_unaligned_roundtrip() -> Result<(), Error> {
    let parent_pid: ProcessIdentifier = pm::getpid_uncached()?;
    let parent_tid: ThreadIdentifier = pm::__kcall_gettid()?;
    PARENT_PID_RAW.store(u32::try_from(parent_pid)?, ORDER);
    PARENT_TID_RAW.store(u32::try_from(parent_tid)?, ORDER);

    // Pre-touch the destination region so both backing pages are present before the kernel resolves
    // them during the pull(); a never-touched page would not yet be demand-paged.
    // SAFETY: single-threaded; the slice is the only live reference to the destination region.
    let destination: &mut [u8] = unsafe { arena_region(ptr::addr_of_mut!(DST_ARENA), DST_OFFSET) };
    destination.fill(0);
    assert_straddles_page(DST_OFFSET, PAYLOAD_LEN);

    let child_pid: ProcessIdentifier = fork::__kcall_fork()?;
    if child_pid == ProcessIdentifier::from(0) {
        // Child path: never returns to the shared flow.
        let status: i32 = match run_child_push() {
            Ok(()) => CHILD_EXIT_OK,
            Err(_) => CHILD_EXIT_FAIL,
        };
        pm::__kcall_exit(status)?;
    }

    assert!(child_pid != parent_pid, "child PID must differ from parent PID");

    // Receive the child's announced thread identifier so the pull() can be matched to the child's
    // pushing thread.
    let announce: Message = ipc::__kcall_recv()?;
    assert!(announce.message_type == MessageType::Ipc, "expected IPC announcement from child");
    let child_tid_raw: u32 = u32::from_le_bytes([
        announce.payload[0],
        announce.payload[1],
        announce.payload[2],
        announce.payload[3],
    ]);
    let child_tid: ThreadIdentifier = ThreadIdentifier::try_from(child_tid_raw)?;

    // Pull the child's payload into the un-aligned destination buffer.
    let received: usize = ipc::__kcall_pull(child_pid, child_tid, destination)?;
    assert!(
        received == PAYLOAD_LEN,
        "pull() returned {received} bytes; expected {PAYLOAD_LEN} (truncated rendezvous transfer)"
    );

    // The received bytes must match the pattern the child wrote, proving the misaligned,
    // page-crossing cross-process copy preserved every byte.
    for (i, byte) in destination.iter().enumerate() {
        let expected: u8 = pattern_byte(i);
        assert!(
            *byte == expected,
            "byte {i} mismatch after un-aligned rendezvous: got {byte:#x}, expected {expected:#x}"
        );
    }

    Ok(())
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs all un-aligned push/pull buffer regression tests.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-fork-kcall: starting un-aligned push/pull buffer tests");
    test_kernel_bulk_push_page_crossing()?;
    ::syslog::info!("test-fork-kcall: PASS - kernel_bulk_push_page_crossing");
    test_rendezvous_unaligned_roundtrip()?;
    ::syslog::info!("test-fork-kcall: PASS - rendezvous_unaligned_roundtrip");
    Ok(())
}
