/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_NL_TYPES_H
#define _NANVIX_NL_TYPES_H

/**
 * @file nl_types.h
 * @brief Message catalog and language information types.
 */

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Type used by nl_langinfo() to identify a locale item. */
typedef int nl_item;

/** @brief Message catalog descriptor. */
typedef void *nl_catd;

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define NL_SETD 1
#define NL_CAT_LOCALE 1

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_NL_TYPES_H */
