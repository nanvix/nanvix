// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Pipe closure notification.

//==================================================================================================
// Structures
//==================================================================================================

/// Pipe count-to-zero transition surfaced during process resource reclamation.
pub struct PipeClosure {
    /// Stable identity of the affected pipe.
    pub pipe_id: u64,
    /// Whether the released end was the write end (`true`) or the read end (`false`).
    pub was_write: bool,
}
