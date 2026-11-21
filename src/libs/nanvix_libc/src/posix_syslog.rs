// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::core::{
    ffi::{
        c_char,
        c_int,
    },
    hint,
    ptr,
    sync::atomic::{
        AtomicBool,
        AtomicI32,
        AtomicPtr,
        Ordering,
    },
};
use ::sysapi::syslog::{
    LOG_DEBUG,
    LOG_FACMASK,
    LOG_USER,
};

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
use ::core::{
    ffi::{
        c_void,
        CStr,
        VaList,
    },
    fmt::Write,
};
#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
use ::sysapi::{
    sys_types::{
        c_size_t,
        c_ssize_t,
        pid_t,
        time_t,
    },
    syslog::{
        LOG_ALERT,
        LOG_AUTH,
        LOG_AUTHPRIV,
        LOG_CRIT,
        LOG_CRON,
        LOG_DAEMON,
        LOG_EMERG,
        LOG_ERR,
        LOG_FTP,
        LOG_INFO,
        LOG_LOCAL0,
        LOG_LOCAL1,
        LOG_LOCAL2,
        LOG_LOCAL3,
        LOG_LOCAL4,
        LOG_LOCAL5,
        LOG_LOCAL6,
        LOG_LOCAL7,
        LOG_LPR,
        LOG_MAIL,
        LOG_NEWS,
        LOG_NOTICE,
        LOG_PERROR,
        LOG_PID,
        LOG_PRIMASK,
        LOG_SYSLOG,
        LOG_UUCP,
        LOG_WARNING,
    },
};

static OPENLOG_LOCKED: AtomicBool = AtomicBool::new(false);
static OPENLOG_IDENT: AtomicPtr<c_char> = AtomicPtr::new(ptr::null_mut());
static OPENLOG_OPTIONS: AtomicI32 = AtomicI32::new(0);
static OPENLOG_FACILITY: AtomicI32 = AtomicI32::new(LOG_USER);
static LOG_PRIORITY_MASK: AtomicI32 = AtomicI32::new((1 << (LOG_DEBUG + 1)) - 1);

struct OpenLogGuard;

impl Drop for OpenLogGuard {
    fn drop(&mut self) {
        OPENLOG_LOCKED.store(false, Ordering::Release);
    }
}

fn lock_openlog() -> OpenLogGuard {
    while OPENLOG_LOCKED
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        hint::spin_loop();
    }
    OpenLogGuard
}

/// # Description
///
/// Configures the identifier, options, and default facility for subsequent messages.
///
/// # Parameters
///
/// - `ident`: Optional identifier prepended to each message.
/// - `option`: Bitwise OR of logging options such as `LOG_PID` and `LOG_PERROR`.
/// - `facility`: Default facility used when a message does not specify one.
///
/// # Safety
///
/// `ident` must be null or point to a valid null-terminated string that remains valid while
/// subsequent calls to `syslog` use it.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn openlog(ident: *const c_char, option: c_int, facility: c_int) {
    let _guard: OpenLogGuard = lock_openlog();
    OPENLOG_IDENT.store(ident.cast_mut(), Ordering::Relaxed);
    OPENLOG_OPTIONS.store(option, Ordering::Relaxed);
    OPENLOG_FACILITY.store(facility & LOG_FACMASK, Ordering::Relaxed);
}

/// # Description
///
/// Resets the process logging configuration to its defaults.
///
/// Restores a null identifier, no logging options, and `LOG_USER` as the default facility.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn closelog() {
    let _guard: OpenLogGuard = lock_openlog();
    OPENLOG_IDENT.store(ptr::null_mut(), Ordering::Relaxed);
    OPENLOG_OPTIONS.store(0, Ordering::Relaxed);
    OPENLOG_FACILITY.store(LOG_USER, Ordering::Relaxed);
}

/// # Description
///
/// Sets the process log priority mask and returns the previous mask.
///
/// # Parameters
///
/// - `mask`: Bitmask of enabled severities. Zero leaves the current mask unchanged.
///
/// # Returns
///
/// The mask that was in effect before the call.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn setlogmask(mask: c_int) -> c_int {
    if mask == 0 {
        LOG_PRIORITY_MASK.load(Ordering::Acquire)
    } else {
        LOG_PRIORITY_MASK.swap(mask, Ordering::AcqRel)
    }
}

/// # Description
///
/// Formats and writes a message to the guest console.
///
/// # Parameters
///
/// - `priority`: Severity and optional facility for this record.
/// - `format`: Null-terminated printf-style format string.
/// - `...`: Values corresponding to the format conversions.
///
/// # Safety
///
/// `format` must point to a valid null-terminated format string, and the variadic arguments must
/// match its conversion specifications.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn syslog(priority: c_int, format: *const c_char, args: ...) {
    #[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
    {
        let args: VaList<'_> = args;
        unsafe { syslog_inner(priority, format, args) };
    }
    #[cfg(not(all(feature = "backend-nanvix", feature = "staticlib")))]
    {
        let _ = (priority, format, args);
    }
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
unsafe fn syslog_inner(priority: c_int, format: *const c_char, args: VaList<'_>) {
    let entry_errno: c_int = unsafe { *crate::__errno() };
    let _errno_guard: ErrnoGuard = ErrnoGuard(entry_errno);

    let severity: c_int = priority & LOG_PRIMASK;
    if LOG_PRIORITY_MASK.load(Ordering::Acquire) & (1 << severity) == 0 {
        return;
    }

    let _guard: OpenLogGuard = lock_openlog();
    let ident: *const c_char = OPENLOG_IDENT.load(Ordering::Relaxed);
    let options: c_int = OPENLOG_OPTIONS.load(Ordering::Relaxed);
    let configured_facility: c_int = OPENLOG_FACILITY.load(Ordering::Relaxed);
    let facility: c_int = match priority & LOG_FACMASK {
        0 => configured_facility,
        value => value,
    };

    if format.is_null() {
        return;
    }
    let Some(format_storage) = (unsafe { expand_errno_specifiers(format, entry_errno) }) else {
        ::syslog::error!("syslog(): failed to expand the message format");
        return;
    };

    let Some(message) = (unsafe {
        ::libc_stdio::vsnprintf::format_to_vec(format_storage.as_ptr().cast::<c_char>(), args)
    }) else {
        ::syslog::error!("syslog(): message formatting failed");
        return;
    };

    let ident_bytes: &[u8] = if ident.is_null() {
        b"syslog"
    } else {
        unsafe { CStr::from_ptr(ident) }.to_bytes()
    };
    let timestamp: time_t = unsafe { time(ptr::null_mut()) };
    let pid: pid_t = if options & LOG_PID != 0 {
        unsafe { getpid() }
    } else {
        -1
    };
    let Some(record_capacity) = ident_bytes
        .len()
        .checked_add(message.len())
        .and_then(|length| length.checked_add(128))
    else {
        ::syslog::error!("syslog(): log record length overflow");
        return;
    };
    let Some(mut record) = FallibleBuffer::with_capacity(record_capacity) else {
        ::syslog::error!("syslog(): failed to allocate the log record");
        return;
    };
    if timestamp < 0 {
        if write!(&mut record, "[SYSLOG][unknown][").is_err() {
            ::syslog::error!("syslog(): failed to format the log record");
            return;
        }
    } else {
        if write!(&mut record, "[SYSLOG][{timestamp}][").is_err() {
            ::syslog::error!("syslog(): failed to format the log record");
            return;
        }
    }
    if record.write_bytes(ident_bytes).is_err() {
        ::syslog::error!("syslog(): failed to format the log record");
        return;
    }
    if options & LOG_PID != 0 {
        if pid >= 0 {
            if write!(&mut record, "][{pid}]").is_err() {
                ::syslog::error!("syslog(): failed to format the log record");
                return;
            }
        } else if record.write_bytes(b"][pid=unknown]").is_err() {
            ::syslog::error!("syslog(): failed to format the log record");
            return;
        }
    } else if record.write_bytes(b"]").is_err() {
        ::syslog::error!("syslog(): failed to format the log record");
        return;
    }
    if write!(&mut record, "[{}:{}] ", facility_name(facility), severity_name(severity)).is_err() {
        ::syslog::error!("syslog(): failed to format the log record");
        return;
    }
    if record.write_bytes(&message).is_err() {
        ::syslog::error!("syslog(): failed to format the log record");
        return;
    }
    if record.last_byte() != Some(b'\n') && record.write_bytes(b"\n").is_err() {
        ::syslog::error!("syslog(): failed to format the log record");
        return;
    }

    let record: ::alloc::vec::Vec<u8> = record.into_inner();
    if let Some(console_record) = utf8_lossy(&record) {
        emit_console(&console_record);
    } else {
        ::syslog::error!("syslog(): failed to allocate the console message");
    }
    if options & LOG_PERROR != 0 {
        emit_stderr(&record);
    }
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
struct FallibleBuffer {
    bytes: ::alloc::vec::Vec<u8>,
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
impl FallibleBuffer {
    fn with_capacity(capacity: usize) -> Option<Self> {
        let mut bytes: ::alloc::vec::Vec<u8> = ::alloc::vec::Vec::new();
        bytes.try_reserve_exact(capacity).ok()?;
        Some(Self { bytes })
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), ()> {
        self.bytes.try_reserve(bytes.len()).map_err(|_| ())?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn last_byte(&self) -> Option<u8> {
        self.bytes.last().copied()
    }

    fn into_inner(self) -> ::alloc::vec::Vec<u8> {
        self.bytes
    }
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
impl ::core::fmt::Write for FallibleBuffer {
    fn write_str(&mut self, value: &str) -> ::core::fmt::Result {
        self.bytes
            .try_reserve(value.len())
            .map_err(|_| ::core::fmt::Error)?;
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
fn facility_name(facility: c_int) -> &'static str {
    match facility {
        LOG_KERN => "kern",
        LOG_USER => "user",
        LOG_MAIL => "mail",
        LOG_DAEMON => "daemon",
        LOG_AUTH => "auth",
        LOG_SYSLOG => "syslog",
        LOG_LPR => "lpr",
        LOG_NEWS => "news",
        LOG_UUCP => "uucp",
        LOG_CRON => "cron",
        LOG_AUTHPRIV => "authpriv",
        LOG_FTP => "ftp",
        LOG_LOCAL0 => "local0",
        LOG_LOCAL1 => "local1",
        LOG_LOCAL2 => "local2",
        LOG_LOCAL3 => "local3",
        LOG_LOCAL4 => "local4",
        LOG_LOCAL5 => "local5",
        LOG_LOCAL6 => "local6",
        LOG_LOCAL7 => "local7",
        _ => "unknown",
    }
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
fn severity_name(severity: c_int) -> &'static str {
    match severity {
        LOG_EMERG => "emerg",
        LOG_ALERT => "alert",
        LOG_CRIT => "crit",
        LOG_ERR => "err",
        LOG_WARNING => "warning",
        LOG_NOTICE => "notice",
        LOG_INFO => "info",
        _ => "debug",
    }
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
fn emit_console(mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let chunk_len: usize = bytes.len().min(::sys::kcall::debug::MAX_DEBUG_MESSAGE_SIZE);
        let mut end: usize = chunk_len;
        while ::core::str::from_utf8(&bytes[..end]).is_err() {
            end -= 1;
        }
        if end == 0 {
            return;
        }
        if let Err(error) = ::sys::kcall::debug::__kcall_debug(bytes.as_ptr(), end) {
            ::syslog::error!("syslog(): console write failed (error={error:?})");
            return;
        }
        bytes = &bytes[end..];
    }
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
fn emit_stderr(bytes: &[u8]) {
    extern "C" {
        fn write(fd: c_int, buffer: *const c_void, count: c_size_t) -> c_ssize_t;
    }

    let mut written: usize = 0;
    while written < bytes.len() {
        let result: c_ssize_t = unsafe {
            write(
                2,
                bytes.as_ptr().add(written).cast::<c_void>(),
                (bytes.len() - written) as c_size_t,
            )
        };
        if result <= 0 {
            ::syslog::error!("syslog(): standard-error write failed");
            return;
        }
        written += result as usize;
    }
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
fn utf8_lossy(bytes: &[u8]) -> Option<::alloc::vec::Vec<u8>> {
    let mut output: ::alloc::vec::Vec<u8> = ::alloc::vec::Vec::new();
    let mut offset: usize = 0;
    while offset < bytes.len() {
        let remaining: &[u8] = &bytes[offset..];
        match ::core::str::from_utf8(remaining) {
            Ok(_) => {
                output.try_reserve(remaining.len()).ok()?;
                output.extend_from_slice(remaining);
                return Some(output);
            },
            Err(error) => {
                let valid_len: usize = error.valid_up_to();
                let invalid_len: usize = error.error_len().unwrap_or(remaining.len() - valid_len);
                let next_offset: usize = offset.checked_add(valid_len)?.checked_add(invalid_len)?;
                let required_capacity: usize = valid_len.checked_add(3)?;
                output.try_reserve(required_capacity).ok()?;
                output.extend_from_slice(&remaining[..valid_len]);
                output.extend_from_slice(b"\xEF\xBF\xBD");
                offset = next_offset;
            },
        }
    }
    Some(output)
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
unsafe fn expand_errno_specifiers(
    format: *const c_char,
    error_number: c_int,
) -> Option<::alloc::vec::Vec<u8>> {
    extern "C" {
        fn strerror(errnum: c_int) -> *mut c_char;
    }

    let format_bytes: &[u8] = unsafe { CStr::from_ptr(format) }.to_bytes();
    let error_string: *mut c_char = unsafe { strerror(error_number) };
    let error_bytes: &[u8] = if error_string.is_null() {
        b"Unknown error"
    } else {
        unsafe { CStr::from_ptr(error_string) }.to_bytes()
    };
    let mut expanded: ::alloc::vec::Vec<u8> = ::alloc::vec::Vec::new();
    let initial_capacity: usize = format_bytes.len().checked_add(1)?;
    expanded.try_reserve_exact(initial_capacity).ok()?;

    let mut index: usize = 0;
    while index < format_bytes.len() {
        if format_bytes[index] == b'%' && index + 1 < format_bytes.len() {
            match format_bytes[index + 1] {
                b'%' => {
                    expanded.extend_from_slice(b"%%");
                    index += 2;
                    continue;
                },
                b'm' => {
                    let escaped_len: usize =
                        error_bytes.iter().try_fold(0usize, |length, byte| {
                            length.checked_add(if *byte == b'%' { 2 } else { 1 })
                        })?;
                    let remaining_len: usize = format_bytes.len() - index - 2;
                    let required_capacity: usize =
                        escaped_len.checked_add(remaining_len)?.checked_add(1)?;
                    expanded.try_reserve(required_capacity).ok()?;
                    for byte in error_bytes {
                        if *byte == b'%' {
                            expanded.extend_from_slice(b"%%");
                        } else {
                            expanded.push(*byte);
                        }
                    }
                    index += 2;
                    continue;
                },
                _ => {},
            }
        }
        expanded.push(format_bytes[index]);
        index += 1;
    }
    expanded.push(0);
    Some(expanded)
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
struct ErrnoGuard(c_int);

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
impl Drop for ErrnoGuard {
    fn drop(&mut self) {
        unsafe { *crate::__errno() = self.0 };
    }
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
extern "C" {
    fn getpid() -> pid_t;
    fn time(tloc: *mut time_t) -> time_t;
}

#[cfg(all(feature = "backend-nanvix", feature = "staticlib"))]
const LOG_KERN: c_int = 0;
