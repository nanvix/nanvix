/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SYSLOG_H
#define _NANVIX_SYSLOG_H

/**
 * @file syslog.h
 * @brief System logging interface.
 *
 * Nanvix has no system log daemon in standalone mode, so these routines are
 * no-ops; the definitions exist so that ports referencing them compile and link.
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Options (openlog)
 *==================================================================================================*/

#define LOG_PID 0x01
#define LOG_CONS 0x02
#define LOG_ODELAY 0x04
#define LOG_NDELAY 0x08
#define LOG_NOWAIT 0x10
#define LOG_PERROR 0x20

/*==================================================================================================
 * Facilities
 *==================================================================================================*/

#define LOG_KERN (0 << 3)
#define LOG_USER (1 << 3)
#define LOG_MAIL (2 << 3)
#define LOG_DAEMON (3 << 3)
#define LOG_AUTH (4 << 3)
#define LOG_SYSLOG (5 << 3)
#define LOG_LPR (6 << 3)
#define LOG_LOCAL0 (16 << 3)

/*==================================================================================================
 * Priorities
 *==================================================================================================*/

#define LOG_EMERG 0
#define LOG_ALERT 1
#define LOG_CRIT 2
#define LOG_ERR 3
#define LOG_WARNING 4
#define LOG_NOTICE 5
#define LOG_INFO 6
#define LOG_DEBUG 7

#define LOG_MASK(pri) (1 << (pri))
#define LOG_UPTO(pri) ((1 << ((pri) + 1)) - 1)

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern void openlog(const char *ident, int option, int facility);
extern void closelog(void);
extern void syslog(int priority, const char *format, ...);
extern int setlogmask(int mask);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SYSLOG_H */
