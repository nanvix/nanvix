// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use crate::config::TestCaseConfig;

#[test]
fn signal_number_converts_known_signals() -> Result<()> {
    for signal_name in TestCaseConfig::KNOWN_SIGNALS {
        signal_number(signal_name)?;
    }
    assert_eq!(signal_number("SIGTERM")?, ::libc::SIGTERM);
    Ok(())
}

#[test]
fn signal_number_rejects_unknown_signals() {
    assert!(signal_number("SIGKILL").is_err(), "SIGKILL cannot be handled by nanvixd");
    assert!(signal_number("sigterm").is_err(), "signal names are case-sensitive");
}
