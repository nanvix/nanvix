// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::flexi_logger::{
    DeferredNow,
    FileSpec,
    Logger,
};
use ::log::Record;
use ::std::{
    io::Write,
    sync::Once,
};

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Default directory for Rust log files. All components should use this constant to ensure logs
/// are written to a consistent location.
///
pub const DEFAULT_LOG_DIRECTORY: &str = "./logs";

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Formats a log record with relative file paths instead of absolute paths.
///
/// # Parameters
///
/// - `write`: The writer to write the formatted log record to.
/// - `now`: The current timestamp.
/// - `record`: The log record to format.
///
/// # Returns
///
/// A result indicating success or failure.
///
fn format_with_relative_path(
    write: &mut dyn Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), ::std::io::Error> {
    // Get the file path and strip the absolute path prefix to show only the relative path.
    let file_path: &str = if let Some(file) = record.file() {
        // Find the last occurrence of "/nanvix/" and strip everything before it (including "/nanvix/")
        // This will convert paths like:
        // "/opt/github/actions-runner/_work/nanvix/nanvix/src/libs/nanvix-sandbox/src/initialized.rs"
        // to: "src/libs/nanvix-sandbox/src/initialized.rs"
        if let Some(pos) = file.rfind("/nanvix/") {
            &file[pos + 8..] // +8 to skip "/nanvix/"
        } else {
            file
        }
    } else {
        "<unknown>"
    };

    // Format the log record similar to colored_detailed_format but with relative path
    write!(
        write,
        "[{}] {} [{}] {}:{}: {}",
        now.format("%Y-%m-%d %H:%M:%S%.6f %:z"),
        ::flexi_logger::style(record.level()).paint(record.level().to_string()),
        record.module_path().unwrap_or("<unnamed>"),
        file_path,
        record.line().unwrap_or(0),
        record.args()
    )
}

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the logger.
///
/// # Parameters
///
/// - `log_to_file`: Log to file?
/// - `default_level`: Default log level (overridden by RUST_LOG environment variable if set).
/// - `log_dir`: Directory to write log files to (if `log_to_file` is true).
/// - `discriminator`: Optional discriminator to include in log file name for uniqueness.
///
/// # Note
///
/// If the logger cannot be initialized, the function will panic.
///
pub fn init(
    log_to_file: bool,
    default_level: &str,
    log_dir: String,
    discriminator: Option<String>,
) {
    static INIT_LOG: Once = Once::new();
    INIT_LOG.call_once(|| {
        let logger: Logger = Logger::try_with_env_or_str(default_level)
            .expect("malformed RUST_LOG environment variable")
            .format(format_with_relative_path)
            .write_mode(::flexi_logger::WriteMode::Direct);
        if log_to_file {
            let mut file_spec: FileSpec = FileSpec::default().directory(log_dir);
            if let Some(disc) = discriminator {
                file_spec = file_spec.discriminant(disc);
            }
            logger
                .log_to_file(file_spec)
                .start()
                .expect("failed to initialize logger");
        } else {
            logger.start().expect("failed to initialize logger");
        }
    });
}
