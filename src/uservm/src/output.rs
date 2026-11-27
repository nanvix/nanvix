// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Budgeted buffering at blocking host-output boundaries.

#[cfg(target_os = "windows")]
pub(crate) mod console;

use ::log::error;
use ::std::{
    collections::VecDeque,
    io,
};
use ::tokio::sync::mpsc;

const OUTPUT_BUDGET_BYTES: usize = 64 * 1024 * 1024;

const OUTPUT_BATCH_BYTES: usize = 64 * 1024;

/// Byte buffer shared by best-effort console and lossless application output.
pub(crate) struct OutputBuffer {
    data: VecDeque<u8>,
    capacity: usize,
    newlines: usize,
}

impl OutputBuffer {
    /// Creates an empty buffer with the given byte budget.
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::new(),
            capacity,
            newlines: 0,
        }
    }

    /// Returns the number of bytes that can be appended without exceeding the budget.
    pub(crate) fn available(&self) -> usize {
        self.capacity - self.data.len()
    }

    /// Returns whether all queued bytes have been drained.
    pub(crate) fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Appends the prefix that fits, returning its length for the caller's overflow policy.
    pub(crate) fn append(&mut self, bytes: &[u8]) -> usize {
        let count: usize = bytes.len().min(self.available());
        self.data.extend(&bytes[..count]);
        self.newlines += bytes[..count].iter().filter(|&&byte| byte == b'\n').count();
        count
    }

    /// Replaces `batch` with the next bounded batch of queued bytes.
    pub(crate) fn drain_into(&mut self, batch: &mut Vec<u8>, line_buffered: bool) {
        batch.clear();
        let mut count: usize = self.data.len().min(OUTPUT_BATCH_BYTES);
        if line_buffered && self.newlines > 0 {
            count = self
                .data
                .iter()
                .take(count)
                .position(|&byte| byte == b'\n')
                .map_or(count, |index| index + 1);
        }
        batch.extend(self.data.drain(..count));
        self.newlines -= batch.iter().filter(|&&byte| byte == b'\n').count();
    }
}

///
/// # Description
///
/// Eagerly drains guest output while a separate host writer may be stalled.
///
/// The queue is lossless up to 64 MiB. The downstream channel must have capacity one to bound
/// additional staging to one 64 KiB batch plus the batch being written.
///
/// # Parameters
///
/// - `upstream`: Guest application output.
/// - `downstream`: Channel consumed by the host writer.
/// - `line_buffered`: Hand off complete lines promptly instead of coalescing them into blocks.
///
/// # Returns
///
/// Returns after all accepted output has been forwarded and upstream EOF has been observed.
/// Returns an error if the byte budget is exhausted or the host writer goes away.
///
pub async fn buffer_output(
    upstream: &mut mpsc::Receiver<Vec<u8>>,
    downstream: mpsc::Sender<Vec<u8>>,
    line_buffered: bool,
) -> io::Result<()> {
    buffer_output_with_budget(upstream, downstream, OUTPUT_BUDGET_BYTES, line_buffered).await
}

async fn buffer_output_with_budget(
    upstream: &mut mpsc::Receiver<Vec<u8>>,
    downstream: mpsc::Sender<Vec<u8>>,
    budget: usize,
    line_buffered: bool,
) -> io::Result<()> {
    let mut buffer: OutputBuffer = OutputBuffer::new(budget);
    let mut eof: bool = false;
    while !eof || !buffer.is_empty() {
        tokio::select! {
            data = upstream.recv(), if !eof => {
                match data {
                    Some(data) => {
                        if data.len() > buffer.available() {
                            error!(
                                "buffer_output(): host output budget exhausted \
                                 (queued_bytes={}, incoming_bytes={}, budget_bytes={budget})",
                                budget - buffer.available(),
                                data.len()
                            );
                            return Err(io::Error::new(
                                io::ErrorKind::OutOfMemory,
                                "host output buffer budget exhausted",
                            ));
                        }
                        buffer.append(&data);
                    },
                    None => eof = true,
                }
            },
            permit = downstream.reserve(), if !buffer.is_empty()
                && (upstream.is_empty() || buffer.data.len() >= OUTPUT_BATCH_BYTES
                    || (line_buffered && buffer.newlines > 0)) => {
                let permit: mpsc::Permit<'_, Vec<u8>> = permit.map_err(|_| {
                    error!("buffer_output(): host output writer closed");
                    io::Error::new(io::ErrorKind::BrokenPipe, "host output writer closed")
                })?;
                let mut batch: Vec<u8> = Vec::new();
                buffer.drain_into(&mut batch, line_buffered);
                permit.send(batch);
            },
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
