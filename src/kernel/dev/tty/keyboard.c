/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2016 Davidson Francis <davidsondfgl@gmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <signal.h>
#include <stdint.h>
#include <termios.h>
#include "tty.h"

/* ASCII characters. */
#define ESC         27
#define BACKSPACE  '\b'
#define TAB        '\t'
#define ENTER      '\n'
#define RETURN     '\r'
#define NEWLINE   ENTER

/* Non-ASCII special scan-codes. */
#define KESC          1
#define KF1        0x80
#define KF2   (KF1 + 1)
#define KF3   (KF2 + 1)
#define KF4   (KF3 + 1)
#define KF5   (KF4 + 1)
#define KF6   (KF5 + 1)
#define KF7   (KF6 + 1)
#define KF8   (KF7 + 1)
#define KF9   (KF8 + 1)
#define KF10  (KF9 + 1)
#define KF11 (KF10 + 1)
#define KF12 (KF11 + 1)

/* Cursor keys. */
#define KINS           0x90
#define KDEL     (KINS + 1)
#define KHOME    (KDEL + 1)
#define KEND    (KHOME + 1)
#define KPGUP    (KEND + 1)
#define KPGDN   (KPGUP + 1)
#define KLEFT   (KPGDN + 1)
#define KUP     (KLEFT + 1)
#define KDOWN     (KUP + 1)
#define KRIGHT  (KDOWN + 1)
#define KCTRL  (KRIGHT + 1)

/* Meta keys. */
#define KMETA_ALT   0x0200
#define KMETA_CTRL  0x0400
#define KMETA_SHIFT 0x0800
#define KMETA_CAPS  0x1000
#define KMETA_NUM   0x2000
#define KMETA_SCRL  0x4000
#define KMETA_ANY   (KMETA_ALT | KMETA_CTRL | KMETA_SHIFT)

/* Other keys. */
#define KRLEFT_CTRL   0x1D
#define KRRIGHT_CTRL  0x1D
#define KRLEFT_ALT    0x38
#define KRRIGHT_ALT   0x38
#define KRLEFT_SHIFT  0x2A
#define KRRIGHT_SHIFT 0x36
#define KRCAPS_LOCK   0x3A
#define KRSCROLL_LOCK 0x46
#define KRNUM_LOCK    0x45
#define KRDEL         0x53

#ifdef KEYBOARD_US

/**
 * @brief Keymap: US International non-shifted key map.
 */
PRIVATE uint8_t ascii_non_shift[] = {
	'\0', ESC, '1', '2', '3', '4', '5', '6', '7', '8', '9',
	'0', '-', '=', BACKSPACE, TAB, 'q', 'w', 'e', 'r', 't',
	'y', 'u', 'i', 'o', 'p',  '[', ']', ENTER, 0, 'a', 's',
	'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '`', 0, '\\',
	'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/', 0, 0, 0,
	' ', 0, KF1, KF2, KF3, KF4, KF5, KF6, KF7, KF8, KF9, KF10,
	0, 0, KHOME, KUP, KPGUP,'-', KLEFT, '5', KRIGHT, '+', KEND,
	KDOWN, KPGDN, KINS, KDEL, 0, 0, 0, KF11, KF12
};

/**
 * @brief US International shifted keymap.
 */
PRIVATE uint8_t ascii_shift[] = {
	'\0', ESC, '!', '@', '#', '$', '%', '^', '&', '*', '(', ')',
	'_', '+', BACKSPACE, TAB, 'Q', 'W',   'E', 'R', 'T', 'Y', 'U',
	'I', 'O', 'P',   '{', '}', ENTER, KCTRL, 'A', 'S', 'D', 'F', 'G',
	'H', 'J', 'K', 'L', ':', '\"', '~', 0, '|','Z', 'X', 'C', 'V',
	'B', 'N', 'M', '<', '>', '?', 0, 0, 0, ' ', 0, KF1,   KF2,
	KF3, KF4, KF5, KF6, KF7, KF8, KF9, KF10, 0, 0, KHOME, KUP,
	KPGUP, '-', KLEFT, '5',   KRIGHT, '+', KEND, KDOWN, KPGDN,
	KINS, KDEL, 0, 0, 0, KF11, KF12
};

#else

/**
 * @brief French non-shifted keymap.
 */
PRIVATE uint8_t ascii_non_shift[] = {
	0, ESC, '&', 'e', '\"', '\'', '(', '-', 'e', '_', 'c', 'a', ')', '=', BACKSPACE,
	TAB, 'a', 'z', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p',  '^', '$', ENTER,
	0, 'q', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', 'm', 0, 0,
	0, '*', 'w', 'x', 'c', 'v', 'b', 'n', ',', ';', ':', 0, 0,
	0, 0, ' ', 0, KF1, KF2, KF3, KF4, KF5, KF6, KF7, KF8, KF9, KF10,
	0, 0, KHOME, KUP, KPGUP, 0, KLEFT, 0, KRIGHT, 0, KEND,
	KDOWN, KPGDN, KINS, KDEL, 0, 0, 0, KF11, KF12
};

/**
 * @brief French shifted keymap.
 */
PRIVATE uint8_t ascii_shift[] = {
	0, ESC, '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 0, '+', BACKSPACE,
	TAB, 'A', 'Z', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', '\"', 0, ENTER,
	0, 'Q', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 0, 0,
	0, 0, 'W', 'X', 'C', 'V', 'B', 'N', '?', '.', '/', 0, 0,
	0, 0, ' ', 0, KF1,   KF2, KF3, KF4, KF5, KF6, KF7, KF8, KF9, KF10,
	0, 0, KHOME, KUP, KPGUP, 0, KLEFT, 0, KRIGHT, 0, KEND,
	KDOWN, KPGDN, KINS, KDEL, 0, 0, 0, KF11, KF12
};

#endif

/**
 * @brief Keyboard flags.
 */
enum flags
{
	ANY   = (1 << 0), /**< Any key pressed.   */
	SHIFT = (1 << 1), /**< Shift key pressed. */
	CTRL  = (1 << 2)  /**< CTRL key pressed.  */
};

/**
 * @brief Keyboard flags.
 */
PRIVATE enum flags mode = 0;

/*
 * Parses key hit and decodes it to a scan code.
 */
PRIVATE uint8_t parse_key_hit(void)
{
	uint8_t scancode;
	uint8_t port_value;

    scancode = inputb(0x60);

    port_value = inputb(0x61);
    outputb(0x61, port_value | 0x80);
    outputb(0x61, port_value & ~0x80);

	/* A key was released. */
    if(scancode & 0x80)
    {
        scancode &= 0x7F;

        /* Parse scan code. */
        switch (scancode)
        {
			/* Shift. */
			case KRLEFT_SHIFT:
			case KRRIGHT_SHIFT:
				mode &= ~SHIFT;
				break;

			/* CTRL. */
			case KRLEFT_CTRL:
				mode &= ~CTRL;
				break;

			/* Any other. */
			default:
				mode &= ~ANY;
				break;
		}
    }

   	/* A key was pressed. */
    else
    {
        /* Parse scan code. */
        switch (scancode)
        {
			/* Shift. */
			case KRLEFT_SHIFT:
			case KRRIGHT_SHIFT:
				mode |= SHIFT;
				break;

			/* CTRL. */
			case KRLEFT_CTRL:
				mode |= CTRL;
				break;

			/* Any other. */
			default:
				mode |= ANY;
				break;
		}
    }

    return (scancode);
}

/*
 * Gets the ASCII code for the pressed key.
 */
PRIVATE uint8_t get_ascii(void)
{
    uint8_t scancode;
    uint8_t code;

    scancode = parse_key_hit();

	/* Printable character. */
    if (mode & ANY)
    {
		code = (mode & SHIFT) ? ascii_shift[scancode]:ascii_non_shift[scancode];

		/* CTRL pressed. */
		if (mode & CTRL)
			return ((code < 96) ? code - 64 : code - 96);

		return (code);
	}

    return (0);
}

/**
 * @brief Handles a keyboard interrupt.
 *
 * @details Handles a keyboard interrupt, parsing scan codes received from the
 *          keyboard controller to ascii codes.
 */
PUBLIC void do_keyboard_hit(void)
{
	uint8_t ascii_code;

	ascii_code = get_ascii();

	/* Ignore. */
	if (ascii_code == 0)
		return;

	/* Parse ASCII code. */
	switch(ascii_code)
	{
		/* Fall through. */
		case KINS:
        case KDEL:
        case KHOME:
        case KEND:
		case KPGUP:
        case KPGDN:
        case KLEFT:
        case KRIGHT:
        	/*  TODO: implement. */
            break;

		/* Fall through. */
        case KUP:
        case KDOWN:
        	tty_int(ascii_code);
        	break;

		default:
		    tty_int(ascii_code);
        	break;
	}
}

/*
 * Initializes the keyboard driver.
 */
PUBLIC void keyboard_init(void)
{
	set_hwint(INT_KEYBOARD, &do_keyboard_hit);

    while (inputb(0x64) & 1)
		inputb(0x60);
}
