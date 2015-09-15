/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2015-2015 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * <termios.h> - 
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

	/* Size of c_cc[] */
	#define NCCS 11

	/* tcsetattr uses these */
	#define	TCSANOW		0 /* The change occurs immediately. */
	#define	TCSADRAIN	1 /* The change occurs after all output written to the
	                         object referred to by FileDescriptor has been 
	                         transmitted. */

	#define	TCSAFLUSH	2 /* The change occurs after all output written to the
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

    /*
     * Aditional parameters from tcsetattr
     */
    struct set_termios_attr
    {
    	int optional_actions;
    	const struct termios *termiosp;
    };
    
	/*
	 * Gets the parameters associated with the terminal
	 */
	extern int tcgetattr(int fd, struct termios *termiosp);

	/*
	 * Sets the parameters associated with the terminal
	 */
	extern int tcsetattr(int fd, int optional_actions,
		const struct termios *termiosp);

#endif /* TERMIOS_H_ */
