/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_GETOPT_H
#define _NANVIX_GETOPT_H

/**
 * @file getopt.h
 * @brief Command-line option parsing (GNU long options).
 *
 * Declares the GNU long-option extensions to getopt(). The base getopt()
 * interface and the optarg/optind/opterr/optopt globals are declared in
 * <unistd.h>, which this header includes.
 */

#include <unistd.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Long Option Type
 *==================================================================================================*/

/** @brief Describes one long option for getopt_long(). */
struct option {
    const char *name; /**< Long option name, without the leading "--". */
    int has_arg;      /**< One of no_argument, required_argument, optional_argument. */
    int *flag;        /**< If non-null, *flag is set to val and 0 is returned. */
    int val;          /**< Value to return (or store through flag) when matched. */
};

/*==================================================================================================
 * Long Option has_arg Values
 *==================================================================================================*/

#define no_argument       0
#define required_argument 1
#define optional_argument 2

/*==================================================================================================
 * Long Option Functions
 *==================================================================================================*/

extern int getopt_long(int argc, char *const argv[], const char *optstring,
                       const struct option *longopts, int *longindex);
extern int getopt_long_only(int argc, char *const argv[], const char *optstring,
                            const struct option *longopts, int *longindex);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_GETOPT_H */
