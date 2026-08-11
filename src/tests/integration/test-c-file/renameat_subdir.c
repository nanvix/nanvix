/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

/* Must come first. */
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

// Create a file with some content.
static void create_file(const char *path)
{
    int fd = open(path, O_CREAT | O_WRONLY | O_TRUNC, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    const char data[] = "xxxxxxxxxxxxxxxx"; // 16 bytes, matching the repro
    assert(write(fd, data, sizeof(data) - 1) == (ssize_t)(sizeof(data) - 1));
    assert(close(fd) == 0);
}

// Assert a file exists.
static void assert_exists(const char *path)
{
    struct stat st;
    assert(stat(path, &st) == 0);
}

// Assert a file does not exist.
static void assert_not_exists(const char *path)
{
    struct stat st;
    assert(stat(path, &st) == -1);
    assert(errno == ENOENT);
}

//==================================================================================================
// Tests
//==================================================================================================

// Probe 1: rename within root (no subdirs).
static void test_rename_root(void)
{
    fprintf(stderr, "  rename within root ... ");

    create_file("rename_a");
    assert(rename("rename_a", "rename_b") == 0);
    assert_not_exists("rename_a");
    assert_exists("rename_b");
    assert(unlink("rename_b") == 0);

    fprintf(stderr, "passed\n");
}

// Probe 2: rename within one subdir.
static void test_rename_within_subdir(void)
{
    fprintf(stderr, "  rename within subdir ... ");

    assert(mkdir("d", S_IRWXU) == 0);
    create_file("d/a");
    assert(rename("d/a", "d/b") == 0);
    assert_not_exists("d/a");
    assert_exists("d/b");
    assert(unlink("d/b") == 0);
    assert(rmdir("d") == 0);

    fprintf(stderr, "passed\n");
}

// Probe 3: rename from subdir to root.
static void test_rename_subdir_to_root(void)
{
    fprintf(stderr, "  rename subdir -> root ... ");

    assert(mkdir("src_dir", S_IRWXU) == 0);
    create_file("src_dir/file");
    assert(rename("src_dir/file", "file_at_root") == 0);
    assert_not_exists("src_dir/file");
    assert_exists("file_at_root");
    assert(unlink("file_at_root") == 0);
    assert(rmdir("src_dir") == 0);

    fprintf(stderr, "passed\n");
}

// Probe 4: rename across two subdirs.
static void test_rename_across_subdirs(void)
{
    fprintf(stderr, "  rename across subdirs ... ");

    assert(mkdir("dir1", S_IRWXU) == 0);
    assert(mkdir("dir2", S_IRWXU) == 0);
    create_file("dir1/x");
    assert(rename("dir1/x", "dir2/y") == 0);
    assert_not_exists("dir1/x");
    assert_exists("dir2/y");
    assert(unlink("dir2/y") == 0);
    assert(rmdir("dir1") == 0);
    assert(rmdir("dir2") == 0);

    fprintf(stderr, "passed\n");
}

// Probe 5: rename replacing an existing file (POSIX atomic replace).
static void test_rename_replace_existing(void)
{
    fprintf(stderr, "  rename replace existing ... ");

    assert(mkdir("rd", S_IRWXU) == 0);

    // Make source and destination distinguishable so we can verify replacement.
    create_file("rd/old");
    int fd = open("rd/new", O_CREAT | O_WRONLY | O_TRUNC, S_IRUSR | S_IWUSR);
    assert(fd != -1);
    const char dst_data[] = "yyyyyyyyyyyyyyyy"; // 16 bytes
    assert(write(fd, dst_data, sizeof(dst_data) - 1) == (ssize_t)(sizeof(dst_data) - 1));
    assert(close(fd) == 0);

    assert(rename("rd/old", "rd/new") == 0);
    assert_not_exists("rd/old");

    fd = open("rd/new", O_RDONLY);
    assert(fd != -1);
    char buf[1];
    assert(read(fd, buf, sizeof(buf)) == 1);
    assert(close(fd) == 0);
    assert(buf[0] == 'x');

    assert(unlink("rd/new") == 0);
    assert(rmdir("rd") == 0);

    fprintf(stderr, "passed\n");
}

// Probe 6: rename(path, path) is a no-op (POSIX identity rename).
static void test_rename_identity(void)
{
    fprintf(stderr, "  rename identity ... ");

    create_file("id_file");
    assert(rename("id_file", "id_file") == 0);
    assert_exists("id_file");
    assert(unlink("id_file") == 0);

    fprintf(stderr, "passed\n");
}

//==================================================================================================
// Entry Point
//==================================================================================================

void test_renameat_subdir(void)
{
    fprintf(stderr, "testing renameat subdir ...\n");

    test_rename_root();
    test_rename_within_subdir();
    test_rename_subdir_to_root();
    test_rename_across_subdirs();
    test_rename_replace_existing();
    test_rename_identity();

    fprintf(stderr, "renameat subdir: all passed\n");
}
