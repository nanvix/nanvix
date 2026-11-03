// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::read_console_block;
use ::config::kernel::KLOG_BUFFER_SIZE;
use ::sys::ipc::{
    VmBusMessage,
    VmBusMessageKind,
};

/// A normal block returns exactly the declared bytes read from the simulated guest memory.
#[test]
fn read_console_block_returns_declared_payload() {
    let guest_memory: &[u8] = b"[INFO][kernel] hello, klog";
    let envelope: VmBusMessage =
        VmBusMessage::new(guest_memory.len() as u32, VmBusMessageKind::KlogBlock, 0);

    let bytes: Vec<u8> = read_console_block(&envelope, |addr, buf| {
        let start: usize = addr as usize;
        buf.copy_from_slice(&guest_memory[start..start + buf.len()]);
        Ok(())
    })
    .expect("decoding a valid block must succeed");

    assert_eq!(bytes, guest_memory, "the whole declared payload must be returned");
}

/// A zero-length block is a no-op: it never reads guest memory and yields no bytes.
#[test]
fn read_console_block_empty_does_not_read() {
    let envelope: VmBusMessage = VmBusMessage::new(0, VmBusMessageKind::KlogBlock, 0xdead_beef);

    let bytes: Vec<u8> = read_console_block(&envelope, |_addr, _buf| {
        // A zero-length transfer must not touch guest memory.
        Err(::anyhow::anyhow!("read_payload must not be called for an empty block"))
    })
    .expect("an empty block must succeed without reading");

    assert!(bytes.is_empty(), "an empty block must produce no bytes");
}

/// An oversized declared length is clamped to the cap, bounding the host allocation.
#[test]
fn read_console_block_clamps_oversized_length() {
    let oversized: u32 = (KLOG_BUFFER_SIZE as u32) + 4096;
    let envelope: VmBusMessage = VmBusMessage::new(oversized, VmBusMessageKind::KlogBlock, 0);

    let mut requested: usize = 0;
    let bytes: Vec<u8> = read_console_block(&envelope, |_addr, buf| {
        requested = buf.len();
        for byte in buf.iter_mut() {
            *byte = b'x';
        }
        Ok(())
    })
    .expect("a clamped block must still succeed");

    assert_eq!(requested, KLOG_BUFFER_SIZE, "the read must be clamped to the cap");
    assert_eq!(bytes.len(), KLOG_BUFFER_SIZE, "the output must be clamped to the cap");
}

/// A propagated read error surfaces to the caller rather than being swallowed.
#[test]
fn read_console_block_propagates_read_error() {
    let envelope: VmBusMessage = VmBusMessage::new(8, VmBusMessageKind::KlogBlock, 0x1000);

    let result = read_console_block(&envelope, |_addr, _buf| {
        Err(::anyhow::anyhow!("simulated out-of-bounds guest read"))
    });

    assert!(result.is_err(), "a failing payload read must produce an error");
}
