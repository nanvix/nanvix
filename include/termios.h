/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <termios.h> - 
 */

#ifndef TERMIOS_H_
#define TERMIOS_H_

	 #include <stdint.h>

    /* Local modes. */
    #define ECHO   0x1 /* Enable echo.     */
    #define ICANON 0x2 /* Canonical input. */

	/* Size of c_cc[] */
	#define NCCS 17

    /* Terminal input output data types. */
    typedef unsigned cc_t;     /* Used for terminal special characters. */
    typedef unsigned speed_t;  /* Used for terminal baud rates.         */
    typedef unsigned tcflag_t; /* Used for terminal modes.              */

    /*
     * Terminal IO options.
     */
    struct termios
    {
		tcflag_t c_iflag;    /* Input mode flags.   */
		tcflag_t c_oflag;    /* Output mode flags.  */
		tcflag_t c_cflag;    /* Control mode flags. */
		tcflag_t c_lflag;    /* Local mode flags.   */
		tcflag_t c_cc[NCCS]; /* Control characters. */
    };

#endif /* TERMIOS_H_ */
