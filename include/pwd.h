/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_PWD_H
#define _NANVIX_PWD_H

/**
 * @file pwd.h
 * @brief Password database.
 *
 * The `passwd` layout mirrors the Rust definition in the sysapi crate (pwd.rs).
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/** @brief Password database entry. */
struct passwd {
    char *pw_name;   /**< User name.            */
    char *pw_passwd; /**< Encrypted password.   */
    uid_t pw_uid;    /**< User ID.              */
    gid_t pw_gid;    /**< Group ID.             */
    char *pw_gecos;  /**< Real name / comment.  */
    char *pw_dir;    /**< Home directory.       */
    char *pw_shell;  /**< Login shell.          */
};

extern struct passwd *getpwuid(uid_t uid);
extern struct passwd *getpwnam(const char *name);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_PWD_H */
