// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::sync::{
    Arc,
    atomic::{
        AtomicUsize,
        Ordering,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Shared counters for tracking message flow across threads.
///
#[derive(Clone)]
pub struct MessageCounters {
    /// Tracks the number of messages sent by the I/O handler.
    io_handler_num_messages_sent: Arc<AtomicUsize>,
    /// Tracks the number of messages received by the memory thread.
    mem_thread_num_messages_received: Arc<AtomicUsize>,
    /// Tracks the number of messages received by the VMM thread.
    vmm_thread_num_messages_received: Arc<AtomicUsize>,
    /// Tracks the number of times the input function has been called.
    vmm_thread_num_input_calls: Arc<AtomicUsize>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Default for MessageCounters {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageCounters {
    ///
    /// # Description
    ///
    /// Creates a new set of message counters with all values initialized to zero.
    ///
    pub fn new() -> Self {
        Self {
            io_handler_num_messages_sent: Arc::new(AtomicUsize::new(0)),
            mem_thread_num_messages_received: Arc::new(AtomicUsize::new(0)),
            vmm_thread_num_messages_received: Arc::new(AtomicUsize::new(0)),
            vmm_thread_num_input_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    ///
    /// # Description
    ///
    /// Increments the counter for messages sent by the I/O handler.
    ///
    pub fn increment_io_handler_messages_sent(&self) {
        self.io_handler_num_messages_sent
            .fetch_add(1, Ordering::SeqCst);
    }

    ///
    /// # Description
    ///
    /// Returns the current count of messages sent by the I/O handler.
    ///
    pub fn get_io_handler_messages_sent(&self) -> usize {
        self.io_handler_num_messages_sent.load(Ordering::SeqCst)
    }

    ///
    /// # Description
    ///
    /// Increments the counter for messages received by the memory thread.
    ///
    pub fn increment_mem_thread_messages_received(&self) {
        self.mem_thread_num_messages_received
            .fetch_add(1, Ordering::SeqCst);
    }

    ///
    /// # Description
    ///
    /// Returns the current count of messages received by the memory thread.
    ///
    pub fn get_mem_thread_messages_received(&self) -> usize {
        self.mem_thread_num_messages_received.load(Ordering::SeqCst)
    }

    ///
    /// # Description
    ///
    /// Increments the counter for messages received by the VMM thread.
    ///
    pub fn increment_vmm_thread_messages_received(&self) {
        self.vmm_thread_num_messages_received
            .fetch_add(1, Ordering::SeqCst);
    }

    ///
    /// # Description
    ///
    /// Returns the current count of messages received by the VMM thread.
    ///
    pub fn get_vmm_thread_messages_received(&self) -> usize {
        self.vmm_thread_num_messages_received.load(Ordering::SeqCst)
    }

    ///
    /// # Description
    ///
    /// Increments the counter for input function calls in the VMM thread.
    ///
    pub fn increment_vmm_thread_input_calls(&self) {
        self.vmm_thread_num_input_calls
            .fetch_add(1, Ordering::SeqCst);
    }

    ///
    /// # Description
    ///
    /// Returns the current count of input function calls in the VMM thread.
    ///
    pub fn get_vmm_thread_input_calls(&self) -> usize {
        self.vmm_thread_num_input_calls.load(Ordering::SeqCst)
    }
}
