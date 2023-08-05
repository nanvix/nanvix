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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#ifndef TERMIOS_H_
#define TERMIOS_H_

	 #include <stdint.h>

	/* termios ioctl() requests. */

    /* Local mode flags. */
    #define ECHO    0001 /* Enable echo.                                */
	#define ECHOE	0002 /* Echo erase character when backspace.        */
	#define ECHOK	0004 /* Echo KILL.                                  */
	#define ECHONL	0010 /* Echo NL.                                    */
	#define ICANON	0020 /* Canonical input.                            */
	#define IEXTEN	0040 /* Enable extended input character processing. */
	#define ISIG	0100 /* Enable signals.                             */

	/**
	 * @name Control Characters.
	 */
	/**@{*/
	#define VEOF    0 /**< EOF character.   */
	#define VEOL    1 /**< EOL character.   */
	#define VERASE  2 /**< ERASE character. */
	#define VINTR   3 /**< INTR character.  */
	#define VKILL   4 /**< KILL character.  */
	#define VMIN    5 /**< MIN value.       */
	#define VQUIT   6 /**< QUIT character.  */
	#define VSTART  7 /**< START character. */
	#define VSTOP   8 /**< STOP character.  */
	#define VSUSP   9 /**< SUSP character.  */
	#define VTIME  10 /**< TIME value.      */
	/**@}*/

	/**
	 * @name Control Characters
	 */
	/**@{*/
	#define INTR_CHAR(term) ((term).c_cc[VINTR])
	#define STOP_CHAR(term) ((term).c_cc[VSTOP])
	#define SUSP_CHAR(term) ((term).c_cc[VSUSP])
	#define START_CHAR(term) ((term).c_cc[VSTART])
	#define QUIT_CHAR(term) ((term).c_cc[VQUIT])
	#define ERASE_CHAR(term) ((term).c_cc[VERASE])
	#define KILL_CHAR(term) ((term).c_cc[VKILL])
	#define EOL_CHAR(term) ((term).c_cc[VEOL])
	#define EOF_CHAR(term) ((term).c_cc[VEOF])
	#define MIN_CHAR(term) ((term).c_cc[VMIN])
	#define TIME_CHAR(term) ((term).c_cc[VTIME])
	/**@}*/

	/* Size of c_cc[] */
	#define NCCS 11

	/* tcsetattr uses these */
	#define	TCSANOW		1 /* The change occurs immediately. */
	#define	TCSADRAIN	2 /* The change occurs after all output written to the
	                         object referred to by FileDescriptor has been
	                         transmitted. */

	#define	TCSAFLUSH	3 /* The change occurs after all output written to the
	                         object referred to by FileDescriptor has been
	                         transmitted. All input that has been received but
	                         not read is discarded before the change is made.*/

    /* Terminal input output data types. */
    typedef unsigned cc_t;     /* Used for terminal special characters. */
    typedef unsigned speed_t;  /* Used for terminal baud rates.         */
    typedef unsigned tcflag_t; /* Used for terminal modes.              */

    /*
     * Terminal IO options.
     */
    struct termios
    {
		tcflag_t c_iflag;    /* Input mode flags (see above).   */
		tcflag_t c_oflag;    /* Output mode flags (see above).  */
		tcflag_t c_cflag;    /* Control mode flags (see above). */
		tcflag_t c_lflag;    /* Local mode flags (see above).   */
		tcflag_t c_cc[NCCS]; /* Control characters.             */
    };

	/* Forward definitions. */
	extern int tcgetattr(int, struct termios *);
	extern int tcsetattr(int, int, const struct termios *);

#endif /* TERMIOS_H_ */
