// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::sysapi::termios::{
    ECHO,
    ICANON,
    ISIG,
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

/// Tests that readiness follows queued input without consuming it.
#[test]
fn readability_tracks_queued_input() {
    let mut ld: LineDiscipline = LineDiscipline::default();
    assert!(!ld.is_readable(), "an empty terminal is not readable");

    clear_lflag(&mut ld, ICANON | ECHO);
    ld.push_input(b"x");
    assert!(ld.is_readable(), "raw input is immediately readable");
    assert!(ld.is_readable(), "checking readiness does not consume input");

    let mut peeked: [u8; 1] = [0];
    assert_eq!(ld.peek(&mut peeked), ConsoleReadOutcome::Read(1));
    assert_eq!(peeked, *b"x");

    assert_eq!(read_bytes(&mut ld, 1), b"x");
    assert!(!ld.is_readable(), "draining input clears readiness");

    ld.push_eof();
    assert!(ld.is_readable(), "queued EOF makes a read complete immediately");
    assert_eq!(ld.read(&mut [0u8; 1]), ConsoleReadOutcome::Eof);
    assert_eq!(ld.read(&mut [0u8; 1]), ConsoleReadOutcome::Eof);
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
    assert_eq!(read_bytes(&mut ld, 16), b"ab", "VEOF delivers the pending bytes without a newline");
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
    assert_eq!(read_bytes(&mut ld, 16), b"a\n", "the first read stops at the first line boundary");
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
    assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::Eof);
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

/// Tests that `^C` raises an interrupt signal, flushes pending input, and echoes as `^C`.
#[test]
fn interrupt_char_raises_signal_and_flushes_input() {
    let mut ld: LineDiscipline = LineDiscipline::default();

    // A partial line is typed but not yet committed.
    assert_eq!(ld.push_input(b"ab"), b"ab");
    assert!(ld.take_signals().is_empty(), "no signal yet");

    // ^C (VINTR) raises an interrupt and echoes its caret form.
    let echo: Vec<u8> = ld.push_input(b"\x03");
    assert_eq!(echo, b"^C", "the interrupt character echoes as a caret sequence");
    assert_eq!(
        ld.take_signals(),
        ::alloc::vec![TerminalSignal::Interrupt],
        "VINTR raises an interrupt signal"
    );

    // The pending line was flushed: nothing is readable.
    assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
}

/// Tests that `NOFLSH` preserves queued input when a terminal signal is recognized.
#[test]
fn noflsh_preserves_input_on_signal() {
    let mut ld: LineDiscipline = LineDiscipline::default();
    ld.termios.c_lflag |= termios::NOFLSH;
    ld.push_input(b"ready\npartial");
    ld.push_input(&[ld.termios.c_cc[termios::VINTR]]);

    assert_eq!(ld.take_signals(), alloc::vec![TerminalSignal::Interrupt]);
    assert_eq!(read_bytes(&mut ld, 16), b"ready\n");
    ld.push_input(b"\n");
    assert_eq!(read_bytes(&mut ld, 16), b"partial\n");
}

/// Tests that noncanonical `VMIN=0,VTIME=0` reads complete immediately when empty.
#[test]
fn raw_zero_min_zero_time_is_immediately_readable() {
    let mut ld: LineDiscipline = LineDiscipline::default();
    clear_lflag(&mut ld, ICANON | ECHO);
    ld.termios.c_cc[termios::VMIN] = 0;
    ld.termios.c_cc[termios::VTIME] = 0;

    assert!(ld.is_readable());
    assert_eq!(ld.read(&mut [0u8; 1]), ConsoleReadOutcome::Read(0));
}

/// Tests that `^\` and `^Z` map to the quit and suspend signals respectively.
#[test]
fn quit_and_suspend_chars_map_to_signals() {
    let mut ld: LineDiscipline = LineDiscipline::default();

    ld.push_input(b"\x1c"); // ^\ (VQUIT)
    assert_eq!(ld.take_signals(), ::alloc::vec![TerminalSignal::Quit]);

    ld.push_input(b"\x1a"); // ^Z (VSUSP)
    assert_eq!(ld.take_signals(), ::alloc::vec![TerminalSignal::Suspend]);
}

/// Tests that a single fed chunk can produce several signals in order.
#[test]
fn multiple_signal_chars_are_reported_in_order() {
    let mut ld: LineDiscipline = LineDiscipline::default();
    ld.push_input(b"\x03\x1a"); // ^C then ^Z
    assert_eq!(
        ld.take_signals(),
        ::alloc::vec![TerminalSignal::Interrupt, TerminalSignal::Suspend]
    );
}

/// Tests that signal characters are ordinary input when `ISIG` is cleared.
#[test]
fn signal_char_is_ordinary_input_when_isig_cleared() {
    let mut ld: LineDiscipline = LineDiscipline::default();
    clear_lflag(&mut ld, ISIG);

    ld.push_input(b"a\x03b\n");
    assert!(ld.take_signals().is_empty(), "no signal is raised when ISIG is cleared");
    assert_eq!(
        read_bytes(&mut ld, 16),
        b"a\x03b\n",
        "the interrupt byte is delivered as ordinary input"
    );
}

/// Tests that terminal signals are recognized in raw mode (`ICANON` cleared, `ISIG` set).
#[test]
fn signals_recognized_in_raw_mode() {
    let mut ld: LineDiscipline = LineDiscipline::default();
    clear_lflag(&mut ld, ICANON);

    // A raw byte is buffered, then ^C arrives.
    ld.push_input(b"x");
    ld.push_input(b"\x03");

    assert_eq!(ld.take_signals(), ::alloc::vec![TerminalSignal::Interrupt]);
    // The buffered raw byte was flushed by the signal.
    assert_eq!(ld.read(&mut [0u8; 16]), ConsoleReadOutcome::WouldBlock);
}

/// Tests that `take_signals` drains the queue: a second call returns nothing.
#[test]
fn take_signals_drains_the_queue() {
    let mut ld: LineDiscipline = LineDiscipline::default();
    ld.push_input(b"\x03");
    assert_eq!(ld.take_signals(), ::alloc::vec![TerminalSignal::Interrupt]);
    assert!(ld.take_signals().is_empty(), "signals are drained once taken");
}

/// Tests that a signal character is not echoed when `ECHO` is cleared.
#[test]
fn signal_char_not_echoed_without_echo() {
    let mut ld: LineDiscipline = LineDiscipline::default();
    clear_lflag(&mut ld, ECHO);

    let echo: Vec<u8> = ld.push_input(b"\x03");
    assert!(echo.is_empty(), "no echo is produced when ECHO is cleared");
    assert_eq!(ld.take_signals(), ::alloc::vec![TerminalSignal::Interrupt]);
}
