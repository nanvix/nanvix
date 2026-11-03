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
//! - **Signals.** When `ISIG` is set, `VINTR` (`^C`), `VQUIT` (`^\`), and `VSUSP` (`^Z`) flush the
//!   input queue and are reported through [`LineDiscipline::take_signals`] for the daemon to deliver
//!   to the foreground process group; the discipline never delivers a signal itself. When `ISIG` is
//!   cleared they are treated as ordinary input.
//! - **Flow control.** `IXON`/`IXOFF` are ignored: `VSTART` (`^Q`) and `VSTOP` (`^S`) do not gate
//!   output.
//! - **Input translation.** Among the input-mode flags only `ICRNL` is honored; `IGNCR`, `INLCR`,
//!   and `ISTRIP` are not.
//! - **Output processing.** Output post-processing (`OPOST`/`ONLCR`) is not modeled here; the echo
//!   path emits a `CR`-`LF` pair directly, and the console write path is handled separately.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    ConsoleReadOutcome,
    TerminalSignal,
};
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
    /// Terminal-generated signals recognized since the last drain. Populated by
    /// [`LineDiscipline::push_input`] when `ISIG` is set and a `VINTR`/`VQUIT`/`VSUSP` byte arrives,
    /// and drained by [`LineDiscipline::take_signals`] so the daemon can forward them to the
    /// foreground process group. The discipline itself never delivers signals.
    signals: Vec<TerminalSignal>,
    /// Whether the underlying host input stream has closed permanently.
    input_closed: bool,
}

impl Default for LineDiscipline {
    fn default() -> Self {
        Self {
            termios: Termios::console_default(),
            winsize: Winsize::console_default(),
            pending: Vec::new(),
            ready: VecDeque::new(),
            raw_run_open: false,
            signals: Vec::new(),
            input_closed: false,
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

    /// Returns `true` when terminal-generated signals are enabled (`ISIG`).
    fn signals_enabled(&self) -> bool {
        self.termios.c_lflag & termios::ISIG != 0
    }

    /// Returns `true` when terminal-generated signals should preserve queued input.
    fn no_flush(&self) -> bool {
        self.termios.c_lflag & termios::NOFLSH != 0
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
    /// available to [`LineDiscipline::read`]. Terminal-generated signals recognized while `ISIG` is
    /// set are queued for [`LineDiscipline::take_signals`] rather than acted upon here.
    pub fn push_input(&mut self, input: &[u8]) -> Vec<u8> {
        let mut echo: Vec<u8> = Vec::new();
        let canonical: bool = self.canonical();
        let echo_on: bool = self.echo_enabled();
        let signals_on: bool = self.signals_enabled();
        let no_flush: bool = self.no_flush();
        let map_cr_to_nl: bool = self.map_cr_to_nl();
        let erase: Option<u8> = self.control_char(termios::VERASE);
        let kill: Option<u8> = self.control_char(termios::VKILL);
        let eof: Option<u8> = self.control_char(termios::VEOF);
        let intr: Option<u8> = self.control_char(termios::VINTR);
        let quit: Option<u8> = self.control_char(termios::VQUIT);
        let susp: Option<u8> = self.control_char(termios::VSUSP);

        for &raw in input {
            // Input translation: map carriage return to newline when ICRNL is set. The host
            // terminal is in raw mode, so Enter arrives as a carriage return.
            let byte: u8 = if map_cr_to_nl && raw == b'\r' {
                b'\n'
            } else {
                raw
            };

            // Terminal-generated signals take precedence over line editing when ISIG is set. A
            // signal character never reaches the input queue: it flushes the pending input, echoes
            // its caret form, and queues a signal for the daemon to deliver to the foreground group.
            if signals_on {
                let signal: Option<TerminalSignal> = if intr == Some(byte) {
                    Some(TerminalSignal::Interrupt)
                } else if quit == Some(byte) {
                    Some(TerminalSignal::Quit)
                } else if susp == Some(byte) {
                    Some(TerminalSignal::Suspend)
                } else {
                    None
                };
                if let Some(signal) = signal {
                    if echo_on {
                        echo.extend_from_slice(&Self::caret(byte));
                    }
                    if !no_flush {
                        self.flush_input();
                    }
                    self.signals.push(signal);
                    continue;
                }
            }

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
        self.input_closed = true;
        self.raw_run_open = false;
    }

    /// Drains the terminal-generated signals recognized since the last call.
    ///
    /// The console daemon calls this after [`LineDiscipline::push_input`] to learn which
    /// terminal-generated signals (`^C`/`^\`/`^Z`) the just-fed input produced, so it can ask the
    /// process manager daemon to deliver them to the foreground process group. The returned vector
    /// is empty when no signal character was seen.
    pub fn take_signals(&mut self) -> Vec<TerminalSignal> {
        ::core::mem::take(&mut self.signals)
    }

    /// Discards all buffered input, both the pending (uncommitted) line and the cooked queue.
    ///
    /// A terminal-generated signal flushes the input queue (POSIX behavior when `NOFLSH` is clear),
    /// so a partially typed line is dropped rather than delivered to whatever reads next.
    fn flush_input(&mut self) {
        self.pending.clear();
        self.ready.clear();
        self.raw_run_open = false;
    }

    /// Returns the two-byte caret echo (`^X`) for a control byte.
    ///
    /// A control byte `c` echoes as `^` followed by `c ^ 0x40`, so `^C` (`0x03`) prints as `^C`,
    /// `^\` (`0x1c`) as `^\`, and `^Z` (`0x1a`) as `^Z`, matching how an interactive terminal shows
    /// the signal characters it consumes.
    fn caret(byte: u8) -> [u8; 2] {
        [b'^', byte ^ 0x40]
    }

    /// Returns whether a read can complete without waiting for more input.
    pub fn is_readable(&self) -> bool {
        !self.ready.is_empty()
            || self.input_closed
            || (!self.canonical()
                && self.termios.c_cc[termios::VMIN] == 0
                && self.termios.c_cc[termios::VTIME] == 0)
    }

    /// Copies the bytes an immediate read would return without consuming terminal state.
    pub fn peek(&self, buf: &mut [u8]) -> ConsoleReadOutcome {
        if buf.is_empty() {
            return ConsoleReadOutcome::Read(0);
        }

        match self.ready.front() {
            Some(Segment::Eof) => ConsoleReadOutcome::Eof,
            Some(Segment::Data(data)) => {
                let n: usize = buf.len().min(data.len());
                for (dst, src) in buf.iter_mut().zip(data.iter().take(n)) {
                    *dst = *src;
                }
                ConsoleReadOutcome::Read(n)
            },
            None if self.input_closed => ConsoleReadOutcome::Eof,
            None if !self.canonical()
                && self.termios.c_cc[termios::VMIN] == 0
                && self.termios.c_cc[termios::VTIME] == 0 =>
            {
                ConsoleReadOutcome::Read(0)
            },
            None => ConsoleReadOutcome::WouldBlock,
        }
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
        if self.ready.is_empty()
            && !self.canonical()
            && self.termios.c_cc[termios::VMIN] == 0
            && self.termios.c_cc[termios::VTIME] == 0
        {
            return ConsoleReadOutcome::Read(0);
        }

        loop {
            match self.ready.front_mut() {
                // No cooked input is available.
                None if self.input_closed => return ConsoleReadOutcome::Eof,
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
mod tests;
