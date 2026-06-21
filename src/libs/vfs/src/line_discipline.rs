// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! In-guest terminal line discipline for the vfsd console backend.
//!
//! This is the cooked side of the console device. It owns the terminal attributes
//! ([`Termios`]/[`Winsize`]) introduced by the TTY-probing plan and promotes them into a real line
//! discipline: it turns the raw byte stream arriving from the host into the bytes a guest `read`
//! observes, honoring the `termios` flags.
//!
//! # Model
//!
//! Raw input bytes are fed in with [`LineDiscipline::push_input`], which returns the bytes that must
//! be echoed back to the terminal. A guest read consumes cooked bytes with [`LineDiscipline::read`].
//! The two operations are decoupled by an internal queue of [`Segment`]s so that the daemon can
//! buffer input that arrives before a reader is waiting and serve a reader from already-buffered
//! input.
//!
//! vfsd owns one `LineDiscipline` per console device behind a shared lock and drives it: it calls
//! [`LineDiscipline::push_input`] as host bytes arrive and serves guest console reads through
//! [`LineDiscipline::read`], parking the reader or returning `EAGAIN` on
//! [`ConsoleReadOutcome::WouldBlock`]. The discipline itself performs no I/O and never blocks;
//! routing host and guest bytes to these calls is the daemon's responsibility.
//!
//! ## Canonical mode (`ICANON`)
//!
//! Input is assembled into a *line*: ordinary bytes accumulate in a pending-line buffer, `VERASE`
//! deletes the last byte, `VKILL` deletes the whole line, and a newline (or a carriage return when
//! `ICRNL` is set) commits the line — including its terminating newline — as one readable unit. A
//! read returns at most one line and never crosses a line boundary, so a complete line becomes
//! available exactly when the user presses Enter. `VEOF` (`^D`) commits the pending bytes
//! immediately without a newline; on an empty line it instead yields an end-of-file marker so the
//! reader observes a zero-length read.
//!
//! ## Raw mode (`ICANON` cleared)
//!
//! There is no line editing: every byte becomes readable as soon as it arrives, so a reader is
//! served immediately. This is the mode used by `readline`, editors, and the REPL.
//!
//! ## Echo (`ECHO`)
//!
//! When `ECHO` is set, ordinary input is echoed verbatim, an erased byte is echoed as a
//! backspace-space-backspace sequence, and a committed line is echoed as a carriage-return/newline
//! pair so the host cursor advances correctly while the host terminal is in raw mode. `VEOF` is
//! never echoed. When `ECHO` is cleared, no bytes are echoed.
//!
//! ## Limitations
//!
//! The discipline implements the cooked-input behavior the console needs and deliberately omits the
//! rest of the `termios` machinery:
//!
//! - **Signals.** `ISIG` is not acted upon: `VINTR` (`^C`), `VQUIT`, and `VSUSP` (`^Z`) are treated
//!   as ordinary input rather than raising signals.
//! - **Flow control.** `IXON`/`IXOFF` are ignored: `VSTART` (`^Q`) and `VSTOP` (`^S`) do not gate
//!   output.
//! - **Input translation.** Among the input-mode flags only `ICRNL` is honored; `IGNCR`, `INLCR`,
//!   and `ISTRIP` are not.
//! - **Output processing.** Output post-processing (`OPOST`/`ONLCR`) is not modeled here; the echo
//!   path emits a `CR`-`LF` pair directly, and the console write path is handled separately.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::{
    collections::VecDeque,
    vec::Vec,
};
use ::sysapi::{
    sys_ioctl::Winsize,
    termios::{
        self,
        Termios,
    },
};

//==================================================================================================
// Outcomes
//==================================================================================================

/// Outcome of a non-blocking console read attempt.
///
/// The discipline itself never blocks: it reports whether data was available, and the daemon
/// decides whether to park the reader (blocking) or return `EAGAIN` (`O_NONBLOCK`) on
/// [`ConsoleReadOutcome::WouldBlock`].
#[derive(Debug, PartialEq, Eq)]
pub enum ConsoleReadOutcome {
    /// Copied `N` bytes out of the cooked queue. `N` is `0` only for a zero-length request.
    Read(usize),
    /// No cooked input is available — the caller must block (or receive `EAGAIN`).
    WouldBlock,
    /// A `VEOF` (`^D`) on an empty line — the caller must receive end-of-file (`0`).
    Eof,
}

//==================================================================================================
// Segments
//==================================================================================================

/// One committed, readable unit in the cooked queue.
///
/// In canonical mode every committed line is one [`Segment::Data`]; in raw mode incoming bytes are
/// coalesced into a single trailing [`Segment::Data`]. A `VEOF` on an empty line produces a
/// [`Segment::Eof`] so the end-of-file is delivered in order with the surrounding input.
enum Segment {
    /// Readable bytes belonging to one canonical line (or a coalesced run of raw bytes).
    Data(VecDeque<u8>),
    /// A soft end-of-file produced by `VEOF` (`^D`) on an empty line.
    Eof,
}

//==================================================================================================
// Line Discipline
//==================================================================================================

/// The cooked state of the console device: terminal attributes plus the line-discipline buffers.
///
/// Terminal attributes belong to the console *device*, not to an individual descriptor: the three
/// standard streams (and every descriptor duplicated or forked from them) observe one consistent
/// `termios`/`winsize`. This object is therefore held behind a shared lock by every console handle,
/// so a `tcsetattr` through one console descriptor is visible through another and is inherited
/// across `fork`/`dup`, and the input buffered here is shared by every reader of the console.
pub struct LineDiscipline {
    /// Terminal attributes (`tcgetattr`/`tcsetattr`, `TCGETS`/`TCSETS`).
    pub termios: Termios,
    /// Terminal window size (`TIOCGWINSZ`/`TIOCSWINSZ`).
    pub winsize: Winsize,
    /// Bytes of the canonical line currently being edited (no terminator yet). Always empty in raw
    /// mode.
    pending: Vec<u8>,
    /// Committed, readable segments in FIFO order.
    ready: VecDeque<Segment>,
    /// Whether the trailing `ready` segment is an open raw-mode run that new raw bytes may extend.
    ///
    /// Raw bytes coalesce only onto another raw run, never onto a committed canonical line or an
    /// end-of-file marker. This keeps a canonical line one self-contained segment, so a canonical
    /// read never crosses a line boundary even if `ICANON` is cleared and later restored while that
    /// line is still queued unread.
    raw_run_open: bool,
}

impl Default for LineDiscipline {
    fn default() -> Self {
        Self {
            termios: Termios::console_default(),
            winsize: Winsize::console_default(),
            pending: Vec::new(),
            ready: VecDeque::new(),
            raw_run_open: false,
        }
    }
}

impl LineDiscipline {
    /// Replaces the terminal attributes and applies line-discipline state transitions.
    pub fn set_termios(&mut self, termios: Termios) {
        let was_canonical: bool = self.canonical();
        self.termios = termios;

        if was_canonical && !self.canonical() && !self.pending.is_empty() {
            self.commit_pending_line();
        }
    }

    /// Returns `true` when canonical-mode line editing is enabled (`ICANON`).
    fn canonical(&self) -> bool {
        self.termios.c_lflag & termios::ICANON != 0
    }

    /// Returns `true` when input echo is enabled (`ECHO`).
    fn echo_enabled(&self) -> bool {
        self.termios.c_lflag & termios::ECHO != 0
    }

    /// Returns `true` when carriage returns are mapped to newlines on input (`ICRNL`).
    fn map_cr_to_nl(&self) -> bool {
        self.termios.c_iflag & termios::ICRNL != 0
    }

    /// Returns the control character at `index`, or `None` when it is disabled (`0`).
    ///
    /// POSIX disables a control character by setting its `c_cc` slot to `_POSIX_VDISABLE` (`0`); a
    /// disabled character must never match an input byte, so it is reported as absent here.
    fn control_char(&self, index: usize) -> Option<u8> {
        match self.termios.c_cc[index] {
            0 => None,
            value => Some(value),
        }
    }

    /// Feeds raw input bytes through the discipline and returns the bytes to echo.
    ///
    /// The returned vector is empty when `ECHO` is cleared. Cooked bytes produced here become
    /// available to [`LineDiscipline::read`].
    pub fn push_input(&mut self, input: &[u8]) -> Vec<u8> {
        let mut echo: Vec<u8> = Vec::new();
        let canonical: bool = self.canonical();
        let echo_on: bool = self.echo_enabled();
        let map_cr_to_nl: bool = self.map_cr_to_nl();
        let erase: Option<u8> = self.control_char(termios::VERASE);
        let kill: Option<u8> = self.control_char(termios::VKILL);
        let eof: Option<u8> = self.control_char(termios::VEOF);

        for &raw in input {
            // Input translation: map carriage return to newline when ICRNL is set. The host
            // terminal is in raw mode, so Enter arrives as a carriage return.
            let byte: u8 = if map_cr_to_nl && raw == b'\r' {
                b'\n'
            } else {
                raw
            };

            if !canonical {
                // Raw mode: the byte becomes readable immediately and is echoed verbatim.
                self.push_raw_byte(byte);
                if echo_on {
                    echo.push(byte);
                }
                continue;
            }

            // Canonical mode: assemble and edit the pending line.
            if byte == b'\n' {
                self.pending.push(b'\n');
                self.commit_pending_line();
                if echo_on {
                    echo.extend_from_slice(b"\r\n");
                }
            } else if Some(byte) == erase {
                if self.pending.pop().is_some() && echo_on {
                    echo.extend_from_slice(b"\x08 \x08");
                }
            } else if Some(byte) == kill {
                let erased: usize = self.pending.len();
                self.pending.clear();
                if echo_on {
                    for _ in 0..erased {
                        echo.extend_from_slice(b"\x08 \x08");
                    }
                }
            } else if Some(byte) == eof {
                // VEOF delivers the pending bytes immediately without a newline; on an empty line it
                // is an end-of-file. It is never echoed.
                if self.pending.is_empty() {
                    self.ready.push_back(Segment::Eof);
                    self.raw_run_open = false;
                } else {
                    self.commit_pending_line();
                }
            } else {
                self.pending.push(byte);
                if echo_on {
                    echo.push(byte);
                }
            }
        }

        echo
    }

    /// Signals end-of-input from the raw console stream.
    pub fn push_eof(&mut self) {
        if !self.pending.is_empty() {
            self.commit_pending_line();
        }
        self.ready.push_back(Segment::Eof);
        self.raw_run_open = false;
    }

    /// Reads cooked bytes into `buf`, returning the outcome of the attempt.
    ///
    /// In canonical mode a read returns at most one line and never crosses a line boundary; a line
    /// shorter than `buf` is returned whole, while a line longer than `buf` is returned in pieces
    /// across successive reads. In raw mode as many buffered bytes as fit are returned. A read of a
    /// zero-length buffer transfers nothing and never blocks.
    pub fn read(&mut self, buf: &mut [u8]) -> ConsoleReadOutcome {
        // A zero-length read transfers nothing and never blocks, even on empty input.
        if buf.is_empty() {
            return ConsoleReadOutcome::Read(0);
        }

        loop {
            match self.ready.front_mut() {
                // No cooked input is available.
                None => return ConsoleReadOutcome::WouldBlock,
                // An end-of-file marker: consume it and report EOF.
                Some(Segment::Eof) => {
                    self.ready.pop_front();
                    return ConsoleReadOutcome::Eof;
                },
                Some(Segment::Data(data)) => {
                    // A committed segment always holds at least one byte, so this guards an
                    // invariant rather than a reachable state: were an empty segment ever queued,
                    // skipping it prevents returning `Read(0)`, which a blocking reader would
                    // misinterpret as end-of-file.
                    if data.is_empty() {
                        self.ready.pop_front();
                        continue;
                    }
                    let n: usize = buf.len().min(data.len());
                    // `n <= data.len()`, so the drain yields exactly `n` bytes in order.
                    for (dst, src) in buf.iter_mut().zip(data.drain(..n)) {
                        *dst = src;
                    }
                    if data.is_empty() {
                        self.ready.pop_front();
                    }
                    return ConsoleReadOutcome::Read(n);
                },
            }
        }
    }

    /// Commits the pending line to the cooked queue as a single readable segment.
    fn commit_pending_line(&mut self) {
        let line: VecDeque<u8> = ::core::mem::take(&mut self.pending).into();
        self.ready.push_back(Segment::Data(line));
        // A canonical line is a closed unit: a following raw byte must start its own segment rather
        // than extend this line, or a canonical read could later cross the line boundary.
        self.raw_run_open = false;
    }

    /// Appends a raw-mode byte, coalescing it into an open raw run when one is at the tail.
    ///
    /// Coalescing groups a burst of raw bytes into one segment so a single read can drain them
    /// together; it never reorders bytes and never merges onto a committed canonical line or an
    /// end-of-file marker (tracked by `raw_run_open`). A canonical line therefore stays one
    /// self-contained segment, so a canonical read never crosses a line boundary even when `ICANON`
    /// is cleared and later restored while that line is still queued unread.
    fn push_raw_byte(&mut self, byte: u8) {
        if self.raw_run_open {
            if let Some(Segment::Data(data)) = self.ready.back_mut() {
                data.push_back(byte);
                return;
            }
        }
        let mut data: VecDeque<u8> = VecDeque::new();
        data.push_back(byte);
        self.ready.push_back(Segment::Data(data));
        self.raw_run_open = true;
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::sysapi::termios::{
        ECHO,
        ICANON,
    };

    /// Reads all immediately-available bytes into a freshly allocated vector of length `max`.
    fn read_bytes(ld: &mut LineDiscipline, max: usize) -> Vec<u8> {
        let mut buf: Vec<u8> = ::alloc::vec![0u8; max];
        match ld.read(&mut buf) {
            ConsoleReadOutcome::Read(n) => {
                buf.truncate(n);
                buf
            },
            _ => Vec::new(),
        }
    }

    /// Clears the given local-mode flags on the discipline's terminal attributes.
    fn clear_lflag(ld: &mut LineDiscipline, flags: u32) {
        ld.termios.c_lflag &= !flags;
    }

    /// Tests that a canonical line becomes readable, whole, only once Enter is pressed.
    #[test]
    fn canonical_line_assembly_on_enter() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        // Without a newline the line is incomplete and a read would block.
        let echo: Vec<u8> = ld.push_input(b"hello");
        assert_eq!(echo, b"hello", "ordinary bytes are echoed verbatim");
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
        // The newline commits the line, which is then read back in full including the newline.
        let echo: Vec<u8> = ld.push_input(b"\n");
        assert_eq!(echo, b"\r\n", "a committed line echoes CR-LF");
        assert_eq!(read_bytes(&mut ld, 16), b"hello\n");
        // After the line is consumed there is nothing left to read.
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
    }

    /// Tests that echo is suppressed when the `ECHO` flag is cleared.
    #[test]
    fn echo_off_when_flag_cleared() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        clear_lflag(&mut ld, ECHO);
        let echo: Vec<u8> = ld.push_input(b"abc\n");
        assert!(echo.is_empty(), "no bytes are echoed when ECHO is cleared");
        assert_eq!(read_bytes(&mut ld, 16), b"abc\n", "the line is still assembled and readable");
    }

    /// Tests that raw mode delivers bytes immediately and, by default here, without echo.
    #[test]
    fn raw_mode_delivers_immediately() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        clear_lflag(&mut ld, ICANON | ECHO);
        // No newline is needed: the bytes are readable as soon as they arrive.
        let echo: Vec<u8> = ld.push_input(b"ab");
        assert!(echo.is_empty(), "ECHO is cleared, so nothing is echoed");
        assert_eq!(read_bytes(&mut ld, 16), b"ab");
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
    }

    /// Tests that raw mode echoes bytes verbatim when `ECHO` remains set.
    #[test]
    fn raw_mode_echoes_when_enabled() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        clear_lflag(&mut ld, ICANON);
        let echo: Vec<u8> = ld.push_input(b"xy");
        assert_eq!(echo, b"xy", "raw bytes are echoed verbatim when ECHO is set");
        assert_eq!(read_bytes(&mut ld, 16), b"xy");
    }

    /// Tests that `^D` on an empty line yields a zero-length read (EOF).
    #[test]
    fn ctrl_d_on_empty_line_is_eof() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        let echo: Vec<u8> = ld.push_input(b"\x04");
        assert!(echo.is_empty(), "VEOF is never echoed");
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::Eof);
        // The EOF marker is consumed: a subsequent read blocks rather than repeating EOF.
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
    }

    /// Tests that `^D` with pending bytes delivers them as a partial line without a newline.
    #[test]
    fn ctrl_d_with_data_delivers_partial_line() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        ld.push_input(b"ab\x04");
        assert_eq!(
            read_bytes(&mut ld, 16),
            b"ab",
            "VEOF delivers the pending bytes without a newline"
        );
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
        // A following lone ^D is then an end-of-file.
        ld.push_input(b"\x04");
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::Eof);
    }

    /// Tests that `VERASE` deletes the last pending byte before the line is committed.
    #[test]
    fn verase_deletes_last_byte() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        // DEL (0x7f) is the default VERASE.
        let echo: Vec<u8> = ld.push_input(b"ab\x7fc\n");
        assert_eq!(read_bytes(&mut ld, 16), b"ac\n", "the erased byte is dropped from the line");
        // The erase echoes a destructive backspace; the surrounding bytes echo verbatim.
        assert_eq!(echo, b"ab\x08 \x08c\r\n");
    }

    /// Tests that `VKILL` discards the entire pending line.
    #[test]
    fn vkill_discards_pending_line() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        // ^U (0x15) is the default VKILL.
        ld.push_input(b"abc\x15xy\n");
        assert_eq!(read_bytes(&mut ld, 16), b"xy\n", "everything before VKILL is discarded");
    }

    /// Tests that a carriage return terminates a canonical line when `ICRNL` is set.
    #[test]
    fn icrnl_maps_cr_to_nl() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        // The default attributes set ICRNL, so Enter (CR) terminates the line as a newline.
        ld.push_input(b"hi\r");
        assert_eq!(read_bytes(&mut ld, 16), b"hi\n", "CR is translated to NL and commits the line");
    }

    /// Tests that a zero-length read transfers nothing and never blocks.
    #[test]
    fn zero_length_read_returns_zero() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        assert_eq!(ld.read(&mut []), ConsoleReadOutcome::Read(0));
    }

    /// Tests that an empty console blocks (the `EAGAIN`/park decision point for the daemon).
    #[test]
    fn empty_console_would_block() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        assert_eq!(ld.read(&mut [0u8; 8]), ConsoleReadOutcome::WouldBlock);
    }

    /// Tests that canonical reads return exactly one line at a time.
    #[test]
    fn canonical_returns_one_line_per_read() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        ld.push_input(b"a\nbc\n");
        assert_eq!(
            read_bytes(&mut ld, 16),
            b"a\n",
            "the first read stops at the first line boundary"
        );
        assert_eq!(read_bytes(&mut ld, 16), b"bc\n", "the second read returns the next line");
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
    }

    /// Tests that a line longer than the read buffer is returned across successive reads.
    #[test]
    fn canonical_line_split_across_reads() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        ld.push_input(b"abcde\n");
        assert_eq!(read_bytes(&mut ld, 3), b"abc", "only as much as fits is returned");
        assert_eq!(read_bytes(&mut ld, 3), b"de\n", "the remainder of the same line follows");
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
    }

    /// Tests that clearing `ICANON` switches subsequent input to immediate raw delivery.
    #[test]
    fn switching_to_raw_delivers_subsequent_bytes() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        clear_lflag(&mut ld, ICANON);
        ld.push_input(b"z");
        assert_eq!(read_bytes(&mut ld, 16), b"z", "raw input is readable without a newline");
    }

    /// Tests that pending canonical bytes are not stranded when `ICANON` is cleared.
    #[test]
    fn switching_to_raw_releases_pending_canonical_bytes() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        ld.push_input(b"abc");
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);

        let mut termios: Termios = ld.termios;
        termios.c_lflag &= !ICANON;
        ld.set_termios(termios);

        assert_eq!(read_bytes(&mut ld, 16), b"abc");
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
    }

    /// Tests that raw stream EOF delivers pending canonical bytes before reporting EOF.
    #[test]
    fn raw_stream_eof_releases_pending_canonical_bytes() {
        let mut ld: LineDiscipline = LineDiscipline::default();
        ld.push_input(b"abc");
        ld.push_eof();

        assert_eq!(read_bytes(&mut ld, 16), b"abc");
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::Eof);
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
    }

    /// Tests that raw bytes do not merge into an unread, already-committed canonical line.
    ///
    /// A committed line stays one self-contained segment, so after `ICANON` is cleared and later
    /// restored a canonical read still returns the original line whole, never crossing its boundary
    /// into the raw bytes that arrived in between.
    #[test]
    fn raw_bytes_do_not_extend_a_committed_canonical_line() {
        let mut ld: LineDiscipline = LineDiscipline::default();

        // A full canonical line is committed but left unread.
        ld.push_input(b"abc\n");

        // ICANON is cleared and raw bytes arrive while the line is still queued.
        let mut termios: Termios = ld.termios;
        termios.c_lflag &= !ICANON;
        ld.set_termios(termios);
        ld.push_input(b"xy");

        // ICANON is restored before the queued line is drained.
        let mut termios: Termios = ld.termios;
        termios.c_lflag |= ICANON;
        ld.set_termios(termios);

        // The canonical read returns the original line whole, not "abc\nxy".
        assert_eq!(
            read_bytes(&mut ld, 16),
            b"abc\n",
            "a canonical read stops at the committed line boundary"
        );
        // The raw bytes that arrived in between remain their own readable segment.
        assert_eq!(read_bytes(&mut ld, 16), b"xy", "the raw bytes are a separate segment");
        assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
    }
}
