/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_REGEX_H
#define _NANVIX_REGEX_H

/**
 * @file regex.h
 * @brief POSIX regular expressions.
 *
 * Declares the POSIX regular-expression interface (regcomp/regexec/regerror/
 * regfree). Implemented by the libc_regex Rust crate as a Thompson/Pike NFA
 * simulation with submatch tracking, supporting BRE and ERE, anchors, character
 * classes, quantifiers, capturing groups, and alternation.
 */

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

/* Signed offset wide enough to hold a ptrdiff_t/ssize_t value, as POSIX requires. */
typedef ptrdiff_t regoff_t;

/** @brief Compiled regular expression. */
typedef struct {
    size_t re_nsub; /**< Number of parenthesized subexpressions. */
    void *__priv;   /**< Opaque pointer to the compiled program.  */
    int __cflags;   /**< Compile flags captured at regcomp() time. */
} regex_t;

/** @brief Match offsets for a single (sub)expression. */
typedef struct {
    regoff_t rm_so; /**< Byte offset of the match start, or -1. */
    regoff_t rm_eo; /**< Byte offset past the match end, or -1. */
} regmatch_t;

/*==================================================================================================
 * Flags and Error Codes
 *==================================================================================================*/

/* cflags for regcomp(). */
#define REG_EXTENDED 0x0001
#define REG_ICASE    0x0002
#define REG_NEWLINE  0x0004
#define REG_NOSUB    0x0008
#define REG_MINIMAL  0x0010

/* eflags for regexec(). */
#define REG_NOTBOL 0x0100
#define REG_NOTEOL 0x0200

/* Result and error codes. */
#define REG_NOERROR  0
#define REG_NOMATCH  1
#define REG_BADPAT   2
#define REG_ECOLLATE 3
#define REG_ECTYPE   4
#define REG_EESCAPE  5
#define REG_ESUBREG  6
#define REG_EBRACK   7
#define REG_EPAREN   8
#define REG_EBRACE   9
#define REG_BADBR    10
#define REG_ERANGE   11
#define REG_ESPACE   12
#define REG_BADRPT   13

/*==================================================================================================
 * Regular Expression Functions
 *==================================================================================================*/

extern int regcomp(regex_t *restrict preg, const char *restrict pattern, int cflags);
extern int regexec(const regex_t *restrict preg, const char *restrict string, size_t nmatch,
                   regmatch_t pmatch[restrict], int eflags);
extern size_t regerror(int errcode, const regex_t *restrict preg, char *restrict errbuf,
                       size_t errbuf_size);
extern void regfree(regex_t *preg);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_REGEX_H */
