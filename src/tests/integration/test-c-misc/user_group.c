/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <grp.h>
#include <pwd.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether the synthetic user and group databases describe the root identity.
void test_user_group(void)
{
    const char *home = getenv("HOME");
    struct passwd *pw = getpwnam("root");
    assert(pw != NULL);
    assert(strcmp(pw->pw_name, "root") == 0);
    assert(pw->pw_uid == getuid());
    assert(pw->pw_gid == getgid());
    assert(strcmp(pw->pw_dir, home == NULL || home[0] == '\0' ? "/" : home) == 0);
    assert(strcmp(pw->pw_shell, "/bin/sh") == 0);

    pw = getpwuid(getuid());
    assert(pw != NULL);
    assert(pw->pw_uid == getuid());
    assert(pw->pw_gid == getgid());

    assert(getpwuid(getuid() + 1) == NULL);
    assert(getpwnam("user") == NULL);

    struct group *gr = getgrgid(getgid());
    assert(gr != NULL);
    assert(strcmp(gr->gr_name, "root") == 0);
    assert(gr->gr_gid == getgid());
    assert(gr->gr_mem != NULL);
    assert(gr->gr_mem[0] == NULL);

    gr = getgrnam("root");
    assert(gr != NULL);
    assert(gr->gr_gid == getgid());

    assert(getgrgid(getgid() + 1) == NULL);
    assert(getgrnam("user") == NULL);

    gid_t groups[1] = {0};
    int ngroups = 1;
    assert(getgrouplist("root", getgid(), groups, &ngroups) == 1);
    assert(ngroups == 1);
    assert(groups[0] == getgid());
    assert(initgroups("root", getgid()) == 0);
    assert(getgrouplist("user", getgid(), groups, &ngroups) == -1);
    assert(getgrouplist("root", getgid() + 1, groups, &ngroups) == -1);
    assert(initgroups("user", getgid()) == -1);
    assert(initgroups("root", getgid() + 1) == -1);
}
