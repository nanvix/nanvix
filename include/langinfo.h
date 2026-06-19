/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_LANGINFO_H
#define _NANVIX_LANGINFO_H

/**
 * @file langinfo.h
 * @brief Language information constants.
 *
 * Declares nl_langinfo() and the nl_item constants used to query locale-specific
 * data. Only the C/POSIX locale is supported. Implemented by the libc_langinfo
 * Rust crate.
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define CODESET 0
#define D_T_FMT 1
#define D_FMT 2
#define T_FMT 3
#define T_FMT_AMPM 4
#define AM_STR 5
#define PM_STR 6
#define DAY_1 7
#define DAY_2 8
#define DAY_3 9
#define DAY_4 10
#define DAY_5 11
#define DAY_6 12
#define DAY_7 13
#define ABDAY_1 14
#define ABDAY_2 15
#define ABDAY_3 16
#define ABDAY_4 17
#define ABDAY_5 18
#define ABDAY_6 19
#define ABDAY_7 20
#define MON_1 21
#define MON_2 22
#define MON_3 23
#define MON_4 24
#define MON_5 25
#define MON_6 26
#define MON_7 27
#define MON_8 28
#define MON_9 29
#define MON_10 30
#define MON_11 31
#define MON_12 32
#define ABMON_1 33
#define ABMON_2 34
#define ABMON_3 35
#define ABMON_4 36
#define ABMON_5 37
#define ABMON_6 38
#define ABMON_7 39
#define ABMON_8 40
#define ABMON_9 41
#define ABMON_10 42
#define ABMON_11 43
#define ABMON_12 44
#define RADIXCHAR 45
#define THOUSEP 46
#define YESEXPR 47
#define NOEXPR 48
#define CRNCYSTR 49

/*==================================================================================================
 * Types
 *==================================================================================================*/

#ifndef _NL_ITEM_DEFINED
#define _NL_ITEM_DEFINED
/** @brief Identifier for an item of locale-specific data. */
typedef int nl_item;
#endif

/*==================================================================================================
 * Functions
 *==================================================================================================*/

extern char *nl_langinfo(nl_item item);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_LANGINFO_H */
