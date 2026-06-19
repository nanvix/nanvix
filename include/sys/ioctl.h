/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYS_IOCTL_H
#define _NANVIX_SYS_IOCTL_H

/**
 * @file sys/ioctl.h
 * @brief Device control operations.
 *
 * Declares `ioctl()` and the terminal window-size request used by interactive
 * ports. Nanvix has no interactive terminal in standalone mode, so the
 * terminal requests fail; the definitions exist so such ports compile and link.
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Structures
 *==================================================================================================*/

/** @brief Terminal window size. */
struct winsize {
    unsigned short ws_row;    /**< Rows, in characters.    */
    unsigned short ws_col;    /**< Columns, in characters. */
    unsigned short ws_xpixel; /**< Horizontal size, pixels. */
    unsigned short ws_ypixel; /**< Vertical size, pixels.   */
};

/*==================================================================================================
 * Requests
 *==================================================================================================*/

#define TIOCGWINSZ 0x5413
#define TIOCSWINSZ 0x5414
#define FIONREAD 0x541b

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern int ioctl(int fd, unsigned long request, ...);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYS_IOCTL_H */
