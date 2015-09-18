/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2015-2015 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * <tsh.h> - Tiny UNIX Shell library.
 */

#ifndef TSH_H_
#define TSH_H_

	#include <stdlib.h>
	
	/* Software versioning. */
	#define TSH_NAME    "tsh" /* Name.    */
	#define TSH_VERSION "1.0" /* Version. */
	
	/* Software copyright message. */
	#define SH_COPYRIGHT \
		"Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>\n"
	
	/* Maximum line length. */
	#define LINELEN 256
	
	/* Max arguments of a command. */
	#define CMD_MAXARGS 32

	/* Command flags. */
	#define CMD_ASYNC 001 /* Asynchronous? */
	#define CMD_PIPE  002 /* Piping?       */

	/* Shell flags. */
	#define SH_INTERACTIVE 001 /* Interactive? */

	/*
	 * Aborts if not an interactive shell.
	 */
	#define sherror()                    \
		if (!(shflags & SH_INTERACTIVE)) \
			exit(EXIT_FAILURE);          \
	
	/* Shell flags. */
	extern int shflags;
	
	/* Shell return value. */
	extern int shret;

	/* Control Characters. */
	#define ERASE_CHAR(termios) ((termios).c_cc[VERASE])
	#define KILL_CHAR(termios) ((termios).c_cc[VKILL])
	#define EOL_CHAR(termios) ((termios).c_cc[VEOL])
	#define EOF_CHAR(termios) ((termios).c_cc[VEOF])

	/* ASCII characters. */
	#define ESC         27
	#define BACKSPACE  '\b'
	#define TAB        '\t'
	#define ENTER      '\n'
	#define NEWLINE   ENTER

	/* Cursor keys. */
	#define KUP    0x97
	#define KDOWN  0x98

	/* Command stack. */
	#define STACK_SIZE  16

	#define CLEAR_BUFFER()             \
		for(int i=size; i<length; i++) \
		{                              \
			*p-- = 0;                  \
			putchar(BACKSPACE);        \
		}                

#endif /* TSH_H_ */
