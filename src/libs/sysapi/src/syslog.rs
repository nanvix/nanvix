// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// Include the process identifier in each message.
pub const LOG_PID: c_int = 0x01;

/// Write to the system console if logging fails.
pub const LOG_CONS: c_int = 0x02;

/// Delay opening the logging connection until the first message.
pub const LOG_ODELAY: c_int = 0x04;

/// Open the logging connection immediately.
pub const LOG_NDELAY: c_int = 0x08;

/// Do not wait for child processes created while logging.
pub const LOG_NOWAIT: c_int = 0x10;

/// Also write messages to standard error.
pub const LOG_PERROR: c_int = 0x20;

/// Kernel facility.
pub const LOG_KERN: c_int = 0 << 3;

/// User-level facility.
pub const LOG_USER: c_int = 1 << 3;

/// Mail facility.
pub const LOG_MAIL: c_int = 2 << 3;

/// System daemon facility.
pub const LOG_DAEMON: c_int = 3 << 3;

/// Security or authorization facility.
pub const LOG_AUTH: c_int = 4 << 3;

/// Internal system logger facility.
pub const LOG_SYSLOG: c_int = 5 << 3;

/// Line printer facility.
pub const LOG_LPR: c_int = 6 << 3;

/// Network news facility.
pub const LOG_NEWS: c_int = 7 << 3;

/// UUCP facility.
pub const LOG_UUCP: c_int = 8 << 3;

/// Clock daemon facility.
pub const LOG_CRON: c_int = 9 << 3;

/// Private security or authorization facility.
pub const LOG_AUTHPRIV: c_int = 10 << 3;

/// File transfer facility.
pub const LOG_FTP: c_int = 11 << 3;

/// Reserved local facility 0.
pub const LOG_LOCAL0: c_int = 16 << 3;

/// Reserved local facility 1.
pub const LOG_LOCAL1: c_int = 17 << 3;

/// Reserved local facility 2.
pub const LOG_LOCAL2: c_int = 18 << 3;

/// Reserved local facility 3.
pub const LOG_LOCAL3: c_int = 19 << 3;

/// Reserved local facility 4.
pub const LOG_LOCAL4: c_int = 20 << 3;

/// Reserved local facility 5.
pub const LOG_LOCAL5: c_int = 21 << 3;

/// Reserved local facility 6.
pub const LOG_LOCAL6: c_int = 22 << 3;

/// Reserved local facility 7.
pub const LOG_LOCAL7: c_int = 23 << 3;

/// Number of logging facilities.
pub const LOG_NFACILITIES: c_int = 24;

/// Mask selecting the facility bits from a priority value.
pub const LOG_FACMASK: c_int = 0x03f8;

/// System is unusable.
pub const LOG_EMERG: c_int = 0;

/// Immediate action is required.
pub const LOG_ALERT: c_int = 1;

/// Critical condition.
pub const LOG_CRIT: c_int = 2;

/// Error condition.
pub const LOG_ERR: c_int = 3;

/// Warning condition.
pub const LOG_WARNING: c_int = 4;

/// Normal but significant condition.
pub const LOG_NOTICE: c_int = 5;

/// Informational message.
pub const LOG_INFO: c_int = 6;

/// Debug-level message.
pub const LOG_DEBUG: c_int = 7;

/// Mask selecting the priority bits from a priority value.
pub const LOG_PRIMASK: c_int = 0x07;

/// Internal marker for an absent priority name.
pub const INTERNAL_NOPRI: c_int = 0x10;

/// Internal marker facility.
pub const INTERNAL_MARK: c_int = LOG_NFACILITIES << 3;
