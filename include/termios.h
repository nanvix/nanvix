/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_TERMIOS_H
#define _NANVIX_TERMIOS_H

/**
 * @file termios.h
 * @brief General terminal interface.
 *
 * Declares `struct termios` and the terminal-attribute interfaces. In standalone
 * mode `tcgetattr`/`tcsetattr` are served by the vfsd console backend; hosted
 * deployments have no guest terminal device, so they fail with `ENOSYS`. The
 * definitions let ports with an interactive mode (e.g. the QuickJS REPL) compile
 * and link.
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

typedef unsigned int tcflag_t;
typedef unsigned char cc_t;
typedef unsigned int speed_t;

#define NCCS 32

struct termios {
    tcflag_t c_iflag; /**< Input modes.   */
    tcflag_t c_oflag; /**< Output modes.  */
    tcflag_t c_cflag; /**< Control modes. */
    tcflag_t c_lflag; /**< Local modes.   */
    cc_t c_line;      /**< Line discipline. */
    cc_t c_cc[NCCS];  /**< Control characters. */
    speed_t c_ispeed; /**< Input speed.   */
    speed_t c_ospeed; /**< Output speed.  */
};

/*==================================================================================================
 * Input flags (c_iflag)
 *==================================================================================================*/

#define IGNBRK 0x0001
#define BRKINT 0x0002
#define IGNPAR 0x0004
#define PARMRK 0x0008
#define INPCK 0x0010
#define ISTRIP 0x0020
#define INLCR 0x0040
#define IGNCR 0x0080
#define ICRNL 0x0100
#define IUCLC 0x0200
#define IXON 0x0400
#define IXANY 0x0800
#define IXOFF 0x1000
#define IMAXBEL 0x2000
#define IUTF8 0x4000

/*==================================================================================================
 * Output flags (c_oflag)
 *==================================================================================================*/

#define OPOST 0x0001
#define OLCUC 0x0002
#define ONLCR 0x0004
#define OCRNL 0x0008
#define ONOCR 0x0010
#define ONLRET 0x0020
#define OFILL 0x0040
#define OFDEL 0x0080
#define NLDLY 0x0100
#define NL0 0x0000
#define NL1 0x0100
#define CRDLY 0x0600
#define CR0 0x0000
#define CR1 0x0200
#define CR2 0x0400
#define CR3 0x0600
#define TABDLY 0x1800
#define TAB0 0x0000
#define TAB1 0x0800
#define TAB2 0x1000
#define TAB3 0x1800
#define XTABS 0x1800
#define BSDLY 0x2000
#define BS0 0x0000
#define BS1 0x2000
#define VTDLY 0x4000
#define VT0 0x0000
#define VT1 0x4000
#define FFDLY 0x8000
#define FF0 0x0000
#define FF1 0x8000

/*==================================================================================================
 * Baud rates (c_cflag)
 *==================================================================================================*/

#define CBAUD 0x100f
#define B0 0x0000
#define B50 0x0001
#define B75 0x0002
#define B110 0x0003
#define B134 0x0004
#define B150 0x0005
#define B200 0x0006
#define B300 0x0007
#define B600 0x0008
#define B1200 0x0009
#define B1800 0x000a
#define B2400 0x000b
#define B4800 0x000c
#define B9600 0x000d
#define B19200 0x000e
#define B38400 0x000f
#define EXTA B19200
#define EXTB B38400
#define CBAUDEX 0x1000
#define B57600 0x1001
#define B115200 0x1002
#define B230400 0x1003
#define B460800 0x1004
#define B500000 0x1005
#define B576000 0x1006
#define B921600 0x1007
#define B1000000 0x1008
#define B1152000 0x1009
#define B1500000 0x100a
#define B2000000 0x100b
#define B2500000 0x100c
#define B3000000 0x100d
#define B3500000 0x100e
#define B4000000 0x100f

/*==================================================================================================
 * Control flags (c_cflag)
 *==================================================================================================*/

#define CSIZE 0x0030
#define CS5 0x0000
#define CS6 0x0010
#define CS7 0x0020
#define CS8 0x0030
#define CSTOPB 0x0040
#define CREAD 0x0080
#define PARENB 0x0100
#define PARODD 0x0200
#define HUPCL 0x0400
#define CLOCAL 0x0800
#define CIBAUD 0x100f0000
#define CMSPAR 0x40000000
#define CRTSCTS 0x80000000

/*==================================================================================================
 * Local flags (c_lflag)
 *==================================================================================================*/

#define ISIG 0x0001
#define ICANON 0x0002
#define XCASE 0x0004
#define ECHO 0x0008
#define ECHOE 0x0010
#define ECHOK 0x0020
#define ECHONL 0x0040
#define NOFLSH 0x0080
#define TOSTOP 0x0100
#define ECHOCTL 0x0200
#define ECHOPRT 0x0400
#define ECHOKE 0x0800
#define FLUSHO 0x1000
#define PENDIN 0x4000
#define IEXTEN 0x8000
#define EXTPROC 0x10000

/*==================================================================================================
 * Control-character indices (c_cc)
 *==================================================================================================*/

#define VINTR 0
#define VQUIT 1
#define VERASE 2
#define VKILL 3
#define VEOF 4
#define VTIME 5
#define VMIN 6
#define VSWTC 7
#define VSTART 8
#define VSTOP 9
#define VSUSP 10
#define VEOL 11
#define VREPRINT 12
#define VDISCARD 13
#define VWERASE 14
#define VLNEXT 15
#define VEOL2 16

/*==================================================================================================
 * Optional actions for tcsetattr()
 *==================================================================================================*/

#define TCSANOW 0
#define TCSADRAIN 1
#define TCSAFLUSH 2

/*==================================================================================================
 * Queue selectors for tcflush()
 *==================================================================================================*/

#define TCIFLUSH 0
#define TCOFLUSH 1
#define TCIOFLUSH 2

/*==================================================================================================
 * Actions for tcflow()
 *==================================================================================================*/

#define TCOOFF 0
#define TCOON 1
#define TCIOFF 2
#define TCION 3

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern int tcgetattr(int fd, struct termios *termios_p);
extern int tcsetattr(int fd, int optional_actions, const struct termios *termios_p);
extern int tcsendbreak(int fd, int duration);
extern int tcdrain(int fd);
extern int tcflush(int fd, int queue_selector);
extern int tcflow(int fd, int action);
extern pid_t tcgetsid(int fd);
extern speed_t cfgetispeed(const struct termios *termios_p);
extern speed_t cfgetospeed(const struct termios *termios_p);
extern int cfsetispeed(struct termios *termios_p, speed_t speed);
extern int cfsetospeed(struct termios *termios_p, speed_t speed);
extern int cfsetspeed(struct termios *termios_p, speed_t speed);
extern void cfmakeraw(struct termios *termios_p);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_TERMIOS_H */
