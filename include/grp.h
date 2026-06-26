/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_GRP_H
#define _NANVIX_GRP_H

/**
 * @file grp.h
 * @brief Group database.
 *
 * The `group` layout mirrors the Rust definition in the sysapi crate (grp.rs).
 */

#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/** @brief Group database entry. */
struct group {
    char *gr_name;    /**< Group name.                       */
    char *gr_passwd;  /**< Encrypted password.               */
    gid_t gr_gid;     /**< Group ID.                         */
    char **gr_mem;    /**< Null-terminated array of members. */
};

extern struct group *getgrgid(gid_t gid);
extern struct group *getgrnam(const char *name);
extern int getgrouplist(const char *user, gid_t group, gid_t *groups, int *ngroups);
extern struct group *getgrent(void);
extern void setgrent(void);
extern void endgrent(void);
extern int initgroups(const char *user, gid_t group);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_GRP_H */
