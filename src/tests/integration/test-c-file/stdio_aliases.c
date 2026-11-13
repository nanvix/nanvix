/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Number of standard descriptor aliases.
#define NUM_ALIASES 3

// Paths of the standard descriptor aliases, indexed by the descriptor that each one names.
static const char *const alias_paths[NUM_ALIASES] = {"/dev/stdin", "/dev/stdout", "/dev/stderr"};

//==================================================================================================
// Private Functions
//==================================================================================================

// Returns whether two stat results report the same file.
static bool same_file(const struct stat *a, const struct stat *b)
{
    return ((a->st_dev == b->st_dev) && (a->st_ino == b->st_ino) && (a->st_mode == b->st_mode));
}

// Asserts that the alias of `fd` reports the metadata of `fd` through stat() and access().
static void assert_alias_follows(int fd)
{
    struct stat expected = {0};
    struct stat actual = {0};
    assert(fstat(fd, &expected) == 0);
    assert(stat(alias_paths[fd], &actual) == 0);
    assert(same_file(&actual, &expected));
    assert(actual.st_size == expected.st_size);
    assert(access(alias_paths[fd], F_OK) == 0);
}

// Saves the standard descriptors in `saved`.
static void save_stdio(int saved[NUM_ALIASES])
{
    for (int fd = 0; fd < NUM_ALIASES; fd++) {
        saved[fd] = dup(fd);
        assert(saved[fd] >= 0);
    }
}

// Restores the standard descriptors from `saved` and closes the saved copies.
static void restore_stdio(const int saved[NUM_ALIASES])
{
    for (int fd = 0; fd < NUM_ALIASES; fd++) {
        assert(dup2(saved[fd], fd) == fd);
        assert(close(saved[fd]) == 0);
    }
}

// Tests that /dev lists the aliases and that they follow the console descriptors.
static void test_console_aliases(void)
{
    DIR *dev = opendir("/dev");
    assert(dev != NULL);
    bool found[NUM_ALIASES] = {false, false, false};
    struct dirent *entry;
    while ((entry = readdir(dev)) != NULL) {
        for (int fd = 0; fd < NUM_ALIASES; fd++) {
            if (strcmp(entry->d_name, alias_paths[fd] + strlen("/dev/")) == 0) {
                assert(!found[fd]);
                found[fd] = true;
            }
        }
    }
    assert(found[0] && found[1] && found[2]);
    assert(closedir(dev) == 0);

    for (int fd = 0; fd < NUM_ALIASES; fd++) {
        assert_alias_follows(fd);
    }

    // An alias of a console descriptor is a terminal that writes to the console.
    int err = open("/dev/stderr", O_WRONLY);
    assert(err > STDERR_FILENO);
    assert(isatty(err) == 1);
    struct stat expected = {0};
    struct stat actual = {0};
    assert(fstat(STDERR_FILENO, &expected) == 0);
    assert(fstat(err, &actual) == 0);
    assert(same_file(&actual, &expected));
    const char message[] = "testing standard descriptor aliases\n";
    assert(write(err, message, sizeof(message) - 1) == (ssize_t)(sizeof(message) - 1));
    assert(close(err) == 0);
}

// Tests that the aliases follow a pipe and outlive the descriptors that they name.
static void test_pipe_aliases(void)
{
    int saved[NUM_ALIASES] = {-1, -1, -1};
    save_stdio(saved);
    int pipefds[2] = {-1, -1};
    assert(pipe(pipefds) == 0);
    assert(dup2(pipefds[0], STDIN_FILENO) == STDIN_FILENO);
    assert(dup2(pipefds[1], STDOUT_FILENO) == STDOUT_FILENO);
    assert_alias_follows(STDIN_FILENO);
    assert_alias_follows(STDOUT_FILENO);

    // The requested access must be allowed by the pipe end.
    errno = 0;
    assert(open("/dev/stdin", O_WRONLY) == -1);
    assert(errno == EACCES);
    errno = 0;
    assert(open("/dev/stdin", O_RDWR) == -1);
    assert(errno == EACCES);
    errno = 0;
    assert(open("/dev/stdout", O_RDONLY) == -1);
    assert(errno == EACCES);

    // Creation flags do not create or replace anything.
    int in = open("/dev/stdin", O_RDONLY);
    int out = open("/dev/stdout", O_WRONLY | O_CREAT | O_TRUNC, 0600);
    assert(in >= 0);
    assert(out >= 0);
    struct stat pipe_st = {0};
    struct stat alias_st = {0};
    assert(fstat(pipefds[1], &pipe_st) == 0);
    assert(fstat(out, &alias_st) == 0);
    assert(S_ISFIFO(alias_st.st_mode));
    assert(same_file(&alias_st, &pipe_st));

    char buffer[4] = {0};
    assert(write(out, "pipe", 4) == 4);
    assert(read(pipefds[0], buffer, sizeof(buffer)) == 4);
    assert(memcmp(buffer, "pipe", 4) == 0);
    assert(write(pipefds[1], "back", 4) == 4);
    assert(read(in, buffer, sizeof(buffer)) == 4);
    assert(memcmp(buffer, "back", 4) == 0);

    // The alias descriptors keep the pipe open once the descriptors that they named are gone.
    restore_stdio(saved);
    assert(close(pipefds[0]) == 0);
    assert(close(pipefds[1]) == 0);
    assert(write(out, "kept", 4) == 4);
    assert(read(in, buffer, sizeof(buffer)) == 4);
    assert(memcmp(buffer, "kept", 4) == 0);
    assert(close(out) == 0);
    assert(read(in, buffer, sizeof(buffer)) == 0);
    assert(close(in) == 0);
}

// Tests the errors reported by the aliases.
static void test_alias_errors(void)
{
    // An alias of a closed descriptor names nothing.
    int saved = dup(STDIN_FILENO);
    assert(saved >= 0);
    assert(close(STDIN_FILENO) == 0);
    struct stat st = {0};
    errno = 0;
    assert(open("/dev/stdin", O_RDONLY) == -1);
    assert(errno == ENOENT);
    errno = 0;
    assert(stat("/dev/stdin", &st) == -1);
    assert(errno == ENOENT);
    errno = 0;
    assert(access("/dev/stdin", F_OK) == -1);
    assert(errno == ENOENT);
    assert(dup2(saved, STDIN_FILENO) == STDIN_FILENO);
    assert(close(saved) == 0);

    // The aliases always exist and cannot be created exclusively.
    errno = 0;
    assert(open("/dev/stdout", O_WRONLY | O_CREAT | O_EXCL, 0600) == -1);
    assert(errno == EEXIST);

    // The aliases name descriptors that are not directories.
    errno = 0;
    assert(open("/dev/stdin/", O_RDONLY) == -1);
    assert(errno == ENOTDIR);
    errno = 0;
    assert(open("/dev/stdout", O_RDONLY | O_DIRECTORY) == -1);
    assert(errno == ENOTDIR);
    errno = 0;
    assert(stat("/dev/stderr/", &st) == -1);
    assert(errno == ENOTDIR);
    errno = 0;
    assert(open("/dev/stderr/child", O_RDONLY) == -1);
    assert(errno == ENOTDIR);
    errno = 0;
    assert(chdir("/dev/stdin") == -1);
    assert(errno == ENOTDIR);

    // The aliases cannot be removed or renamed.
    errno = 0;
    assert(unlink("/dev/stdout") == -1);
    assert(errno == EACCES);
    errno = 0;
    assert(rename("/dev/stderr", "stdio-alias-renamed.tmp") == -1);
    assert(errno == EACCES);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests the standard descriptor aliases on console, pipe, and closed descriptors.
void test_stdio_aliases(void)
{
    test_console_aliases();
    test_pipe_aliases();
    test_alias_errors();
}

// Tests that the standard descriptor aliases follow redirection to a file in the working
// directory, while /dev/console and /dev/tty stay bound to the console.
void test_stdio_alias_redirection(void)
{
    const char path[] = "stdio-alias.tmp";
    int saved[NUM_ALIASES] = {-1, -1, -1};
    save_stdio(saved);
    int file = open(path, O_CREAT | O_RDWR | O_TRUNC, 0600);
    assert(file >= 0);
    assert(write(file, "redirected", 10) == 10);
    assert(lseek(file, 0, SEEK_SET) == 0);
    for (int fd = 0; fd < NUM_ALIASES; fd++) {
        assert(dup2(file, fd) == fd);
    }

    for (int fd = 0; fd < NUM_ALIASES; fd++) {
        assert_alias_follows(fd);
        struct stat st = {0};
        assert(stat(alias_paths[fd], &st) == 0);
        assert(S_ISREG(st.st_mode));
        assert(st.st_size == 10);
    }
    struct stat console_st = {0};
    assert(stat("/dev/console", &console_st) == 0);
    assert(S_ISCHR(console_st.st_mode));
    int console = open("/dev/console", O_WRONLY);
    assert(console >= 0);
    assert(isatty(console) == 1);
    assert(close(console) == 0);
    int tty = open("/dev/tty", O_RDWR);
    assert(tty >= 0);
    assert(isatty(tty) == 1);
    assert(close(tty) == 0);

    // Opening an alias shares the open file description: truncation and append flags have no
    // effect, and the status flags and file offset are shared.
    int out = open("/dev/stdout", O_WRONLY | O_CREAT | O_TRUNC | O_APPEND, 0600);
    assert(out >= 0);
    struct stat st = {0};
    assert(fstat(out, &st) == 0);
    assert(st.st_size == 10);
    int status_flags = fcntl(out, F_GETFL);
    assert(status_flags != -1);
    assert((status_flags & O_ACCMODE) == O_RDWR);
    assert((status_flags & O_APPEND) == 0);
    char buffer[10] = {0};
    assert(read(file, buffer, 5) == 5);
    assert(lseek(out, 0, SEEK_CUR) == 5);
    assert(write(out, "-X", 2) == 2);
    assert(lseek(file, 0, SEEK_CUR) == 7);

    // The descriptor flags of an alias are its own.
    int err = open("/dev/stderr", O_RDWR | O_CLOEXEC);
    assert(err >= 0);
    assert((fcntl(err, F_GETFD) & FD_CLOEXEC) != 0);
    assert((fcntl(STDERR_FILENO, F_GETFD) & FD_CLOEXEC) == 0);
    assert((fcntl(out, F_GETFD) & FD_CLOEXEC) == 0);

    // The alias descriptors keep the file open once the descriptors that they named are gone,
    // and the aliases follow the standard descriptors back to the console.
    restore_stdio(saved);
    assert(close(file) == 0);
    for (int fd = 0; fd < NUM_ALIASES; fd++) {
        assert_alias_follows(fd);
    }
    assert(lseek(err, 0, SEEK_SET) == 0);
    assert(read(err, buffer, sizeof(buffer)) == 10);
    assert(memcmp(buffer, "redir-Xted", 10) == 0);
    assert(close(out) == 0);
    assert(close(err) == 0);
    assert(unlink(path) == 0);
}
