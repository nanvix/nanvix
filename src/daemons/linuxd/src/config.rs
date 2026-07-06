// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::time::Duration;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Timeout for connecting to control-plane.
///
pub const CONTROL_PLANE_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

///
/// # Description
///
/// Timeout for joining the reader task when closing a user VM connection.
///
pub const READER_TASK_JOIN_TIMEOUT: Duration = Duration::from_secs(1);

///
/// # Description
///
/// Timeout for accepting a connection on the gateway socket. Prevents leaked priming tasks when the
/// peer (nanvixd probe or benchmark client) fails to connect.
///
pub const GATEWAY_ACCEPT_TIMEOUT: Duration = Duration::from_secs(60);

///
/// # Description
///
/// Timeout for each step of the worker thread shutdown sequence in `close_connection()`. This
/// bounds both the `send(Shutdown)` enqueue and the subsequent thread join.
///
pub const WORKER_THREAD_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);
