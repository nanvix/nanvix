/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#endif /* TSH_H_ */
