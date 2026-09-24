/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */


#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

// Exercise both the caller-owned and malloc-returned output contracts.
static void expect_path(const char *path, const char *expected)
{
    char buffer[PATH_MAX];
    assert(realpath(path, buffer) == buffer);
    assert(strcmp(buffer, expected) == 0);
    char *allocated = realpath(path, NULL);
    assert(allocated != NULL);
    assert(strcmp(allocated, expected) == 0);
    free(allocated);
}

static void expect_error(const char *path, int expected_errno)
{
    char buffer[PATH_MAX];
    errno = 0;
    assert(realpath(path, buffer) == NULL);
    assert(errno == expected_errno);
    errno = 0;
    assert(realpath(path, NULL) == NULL);
    assert(errno == expected_errno);
}

void test_realpath(void)
{
    fprintf(stderr, "testing realpath() ... ");
    char cwd[PATH_MAX];
    assert(getcwd(cwd, sizeof(cwd)) != NULL);
    expect_path(".", cwd);
    expect_path("/", "/");
    expect_path("/../../", "/");
    expect_error("", ENOENT);

    char oversized[PATH_MAX + 1];
    memset(oversized, 'x', PATH_MAX);
    oversized[PATH_MAX] = '\0';
    expect_error(oversized, ENAMETOOLONG);
    fprintf(stderr, "passed\n");
}

// Runs in the existing hostfs fixture: FAT does not support symbolic links.
void test_realpath_hostfs(void)
{
    fprintf(stderr, "testing realpath() with symlinks ... ");
    char saved_cwd[PATH_MAX];
    assert(getcwd(saved_cwd, sizeof(saved_cwd)) != NULL);
    assert(mkdir("realpath-test", 0700) == 0);
    assert(chdir("realpath-test") == 0);
    assert(mkdir("real", 0700) == 0);
    assert(mkdir("real/sub", 0700) == 0);
    int fd = open("real/marker", O_CREAT | O_EXCL | O_WRONLY, 0600);
    assert(fd >= 0);
    assert(close(fd) == 0);

    char cwd[PATH_MAX];
    char marker[PATH_MAX];
    char subdir[PATH_MAX];
    assert(getcwd(cwd, sizeof(cwd)) != NULL);
    int len = snprintf(marker, sizeof(marker), "%s/real/marker", cwd);
    assert(len > 0 && (size_t)len < sizeof(marker));
    len = snprintf(subdir, sizeof(subdir), "%s/real/sub", cwd);
    assert(len > 0 && (size_t)len < sizeof(subdir));

    assert(symlinkat("real/sub", AT_FDCWD, "alias") == 0);
    assert(symlinkat("alias", AT_FDCWD, "chain") == 0);
    assert(symlinkat("../marker", AT_FDCWD, "real/sub/up") == 0);
    assert(symlinkat(marker, AT_FDCWD, "absolute") == 0);
    assert(symlinkat("missing", AT_FDCWD, "dangling") == 0);
    assert(symlinkat("real/marker", AT_FDCWD, "file-link") == 0);
    assert(symlinkat("cycle-b", AT_FDCWD, "cycle-a") == 0);
    assert(symlinkat("cycle-a", AT_FDCWD, "cycle-b") == 0);

    // alias/.. is real, not the fixture's root. The link must be expanded first.
    expect_path("alias/../marker", marker);
    expect_path("chain/../marker", marker);
    expect_path("alias/up", marker);
    expect_path("absolute", marker);
    expect_path("real//./sub/../marker", marker);
    expect_path("alias//./", subdir);
    expect_error("missing", ENOENT);
    expect_error("missing/../real/marker", ENOENT);
    expect_error("dangling", ENOENT);
    expect_error("real/marker/", ENOTDIR);
    expect_error("real/marker/.", ENOTDIR);
    expect_error("file-link/../marker", ENOTDIR);
    expect_error("cycle-a", ELOOP);

    // Relative lookup from an ordinary physical cwd, without assuming a physical hostfs getcwd.
    assert(chdir("real/sub") == 0);
    expect_path("../marker", marker);
    assert(chdir(cwd) == 0);

    // The guest expansion budget allows exactly 40 links, including a final file link.
    for (int i = 40; i >= 0; --i) {
        char name[16];
        char target[32];
        assert(snprintf(name, sizeof(name), "link-%d", i) > 0);
        if (i == 40) {
            assert(snprintf(target, sizeof(target), "real/marker") > 0);
        } else {
            assert(snprintf(target, sizeof(target), "link-%d", i + 1) > 0);
        }
        assert(symlinkat(target, AT_FDCWD, name) == 0);
    }
    expect_path("link-1", marker);
    expect_error("link-0", ELOOP);
    for (int i = 0; i <= 40; ++i) {
        char name[16];
        assert(snprintf(name, sizeof(name), "link-%d", i) > 0);
        assert(unlink(name) == 0);
    }

    assert(unlink("cycle-b") == 0);
    assert(unlink("cycle-a") == 0);
    assert(unlink("file-link") == 0);
    assert(unlink("dangling") == 0);
    assert(unlink("absolute") == 0);
    assert(unlink("real/sub/up") == 0);
    assert(unlink("chain") == 0);
    assert(unlink("alias") == 0);
    assert(unlink("real/marker") == 0);
    assert(rmdir("real/sub") == 0);
    assert(rmdir("real") == 0);
    assert(chdir(saved_cwd) == 0);
    assert(rmdir("realpath-test") == 0);
    errno = 0;
    fprintf(stderr, "passed\n");
}
