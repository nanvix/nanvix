/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Tests that VFS operations return correct errno for path edge cases.
 */

//==================================================================================================
// Configuration
//==================================================================================================

#define _POSIX_C_SOURCE 200809L

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Helpers
//==================================================================================================

static void create_file(const char *path)
{
    int fd = open(path, O_CREAT | O_WRONLY, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(close(fd) == 0);
}

//==================================================================================================
// mkdir errno tests
//==================================================================================================

// mkdir('/') should fail with EEXIST.
static void test_mkdir_root(void)
{
    fprintf(stderr, "  mkdir root -> EEXIST ... ");
    assert(mkdir("/", S_IRWXU) == -1);
    assert(errno == EEXIST);
    fprintf(stderr, "passed\n");
}

// mkdir(existing_file) should fail with EEXIST.
static void test_mkdir_over_file(void)
{
    fprintf(stderr, "  mkdir over file -> EEXIST ... ");
    create_file("pe_file");
    assert(mkdir("pe_file", S_IRWXU) == -1);
    assert(errno == EEXIST);
    assert(unlink("pe_file") == 0);
    fprintf(stderr, "passed\n");
}

// mkdir(file/child) should fail with ENOTDIR.
static void test_mkdir_under_file(void)
{
    fprintf(stderr, "  mkdir under file -> ENOTDIR ... ");
    create_file("pe_base");
    assert(mkdir("pe_base/child", S_IRWXU) == -1);
    assert(errno == ENOTDIR);
    assert(unlink("pe_base") == 0);
    fprintf(stderr, "passed\n");
}

// mkdir(file/child/grandchild) should fail with ENOTDIR even though the
// non-directory component (file) is not the immediate parent.
static void test_mkdir_under_file_deep(void)
{
    fprintf(stderr, "  mkdir under file (deep) -> ENOTDIR ... ");
    create_file("pe_deep");
    assert(mkdir("pe_deep/child/grandchild", S_IRWXU) == -1);
    assert(errno == ENOTDIR);
    assert(unlink("pe_deep") == 0);
    fprintf(stderr, "passed\n");
}

//==================================================================================================
// open errno tests
//==================================================================================================

// open(file/child, O_CREAT) should fail with ENOTDIR.
static void test_open_under_file(void)
{
    fprintf(stderr, "  open under file -> ENOTDIR ... ");
    create_file("pe_obase");
    int fd = open("pe_obase/child", O_CREAT | O_WRONLY, S_IRUSR | S_IWUSR);
    assert(fd == -1);
    assert(errno == ENOTDIR);
    assert(unlink("pe_obase") == 0);
    fprintf(stderr, "passed\n");
}

// open(nonexistent_dir/file, O_CREAT) should fail with ENOENT.
static void test_open_nonexistent_parent(void)
{
    fprintf(stderr, "  open nonexistent parent -> ENOENT ... ");
    int fd = open("pe_nodir/file", O_CREAT | O_WRONLY, S_IRUSR | S_IWUSR);
    assert(fd == -1);
    assert(errno == ENOENT);
    fprintf(stderr, "passed\n");
}

// open(nonexistent_dir/file, O_WRONLY|O_CREAT|O_TRUNC) should fail with ENOENT.
// Matches shutil.copyfile destination open pattern.
static void test_copyfile_nonexistent_dir(void)
{
    fprintf(stderr, "  copyfile nonexistent dir -> ENOENT ... ");
    int fd = open("pe_nodir2/dst", O_WRONLY | O_CREAT | O_TRUNC, 0666);
    assert(fd == -1);
    assert(errno == ENOENT);
    fprintf(stderr, "passed\n");
}

// open("nonexistent/", O_CREAT) — trailing slash forces dir semantics → ENOENT.
static void test_open_trailing_slash_nonexistent(void)
{
    fprintf(stderr, "  open trailing slash nonexistent -> ENOENT ... ");
    int fd = open("pe_nosuch/", O_WRONLY | O_CREAT | O_TRUNC, 0666);
    assert(fd == -1);
    assert(errno == ENOENT);
    fprintf(stderr, "passed\n");
}

// open("existing_file/", O_RDONLY) — trailing slash on file → ENOTDIR.
static void test_open_trailing_slash_on_file(void)
{
    fprintf(stderr, "  open trailing slash on file -> ENOTDIR ... ");
    // Use a non-empty file to exercise the zero-copy read path.
    int fd = open("pe_tslash", O_CREAT | O_WRONLY, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    assert(write(fd, "data", 4) == 4);
    assert(close(fd) == 0);

    fd = open("pe_tslash/", O_RDONLY);
    assert(fd == -1);
    assert(errno == ENOTDIR);
    assert(unlink("pe_tslash") == 0);
    fprintf(stderr, "passed\n");
}

//==================================================================================================
// Empty path tests
//==================================================================================================

// stat("") should fail with ENOENT, not EINVAL.
static void test_stat_empty_path(void)
{
    fprintf(stderr, "  stat empty path -> ENOENT ... ");
    struct stat st;
    assert(stat("", &st) == -1);
    assert(errno == ENOENT);
    fprintf(stderr, "passed\n");
}

// open("", O_DIRECTORY) should fail with ENOENT (scandir repro).
static void test_opendir_empty_path(void)
{
    fprintf(stderr, "  opendir empty path -> ENOENT ... ");
    int fd = open("", O_RDONLY | O_DIRECTORY);
    assert(fd == -1);
    assert(errno == ENOENT);
    fprintf(stderr, "passed\n");
}

//==================================================================================================
// Entry Point
//==================================================================================================

void test_path_errno(void)
{
    fprintf(stderr, "testing path errno ...\n");

    test_mkdir_root();
    test_mkdir_over_file();
    test_mkdir_under_file();
    test_mkdir_under_file_deep();
    test_open_under_file();
    test_open_nonexistent_parent();
    test_copyfile_nonexistent_dir();
    test_open_trailing_slash_nonexistent();
    test_open_trailing_slash_on_file();
    test_stat_empty_path();
    test_opendir_empty_path();

    fprintf(stderr, "path errno: all passed\n");
}
