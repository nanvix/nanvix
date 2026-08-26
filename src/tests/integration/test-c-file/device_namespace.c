/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <assert.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

// Tests the synthetic namespace and null device.
void test_device_namespace(void)
{
    struct stat directory = {0};
    struct stat null = {0};
    assert(stat("/dev", &directory) == 0);
    assert(S_ISDIR(directory.st_mode));
    assert(stat("/dev/null", &null) == 0);
    assert(S_ISCHR(null.st_mode));
    assert(null.st_dev == directory.st_dev);
    assert(null.st_ino != directory.st_ino);
    assert(null.st_rdev != 0);
    assert(null.st_size == 0);
    assert(null.st_blocks == 0);

    struct stat missing = {0};
    errno = 0;
    assert(stat("/dev/missing", &missing) == -1);
    assert(errno == ENOENT);

    DIR *root = opendir("/");
    assert(root != NULL);
    bool found_dev = false;
    struct dirent *entry;
    while ((entry = readdir(root)) != NULL) {
        if (strcmp(entry->d_name, "dev") == 0) {
            found_dev = true;
            assert(entry->d_ino == directory.st_ino);
        }
    }
    assert(found_dev);
    assert(closedir(root) == 0);

    DIR *dev = opendir("/dev");
    assert(dev != NULL);
    entry = readdir(dev);
    assert(entry != NULL);
    assert(strcmp(entry->d_name, "null") == 0);
    assert(entry->d_ino == null.st_ino);
    assert(readdir(dev) == NULL);
    assert(closedir(dev) == 0);

    int nullfd = open("/dev/null", O_RDWR);
    assert(nullfd >= 0);
    int status_flags = fcntl(nullfd, F_GETFL);
    assert((status_flags & O_ACCMODE) == O_RDWR);
    assert(fcntl(nullfd, F_SETFL, status_flags | O_NONBLOCK) == 0);
    status_flags = fcntl(nullfd, F_GETFL);
    assert((status_flags & O_ACCMODE) == O_RDWR);
    assert(status_flags & O_NONBLOCK);
    assert(write(nullfd, "discarded", 9) == 9);
    assert(write(nullfd, "again", 5) == 5);
    assert(lseek(nullfd, 17, SEEK_SET) == 0);
    char buffer[8] = {0};
    assert(read(nullfd, buffer, sizeof(buffer)) == 0);
    struct stat opened = {0};
    assert(fstat(nullfd, &opened) == 0);
    assert(opened.st_dev == null.st_dev);
    assert(opened.st_ino == null.st_ino);
    assert(opened.st_rdev == null.st_rdev);
    assert(close(nullfd) == 0);

    nullfd = open("/dev/null", O_WRONLY | O_CREAT | O_TRUNC, 0600);
    assert(nullfd >= 0);
    assert(write(nullfd, "discarded", 9) == 9);
    errno = 0;
    assert(read(nullfd, buffer, sizeof(buffer)) == -1);
    assert(errno == EBADF);
    assert(close(nullfd) == 0);

    errno = 0;
    assert(open("/dev/null", O_WRONLY | O_CREAT | O_EXCL, 0600) == -1);
    assert(errno == EEXIST);

    nullfd = open("/dev/null", O_RDONLY);
    assert(nullfd >= 0);
    errno = 0;
    assert(write(nullfd, "rejected", 8) == -1);
    assert(errno == EBADF);
    struct pollfd pollfd = {
        .fd = nullfd,
        .events = POLLIN | POLLOUT,
        .revents = 0,
    };
    assert(poll(&pollfd, 1, 0) == 1);
    assert((pollfd.revents & (POLLIN | POLLOUT)) == (POLLIN | POLLOUT));
    assert(close(nullfd) == 0);

    errno = 0;
    assert(unlink("/dev/null") == -1);
    assert(errno == EACCES);
    errno = 0;
    assert(rename("/dev/null", "device-out.tmp") == -1);
    assert(errno == EACCES);
    errno = 0;
    assert(rmdir("/dev") == -1);
    assert(errno == EACCES);

    int source = open("device-rename.tmp", O_CREAT | O_WRONLY, 0600);
    assert(source >= 0);
    assert(close(source) == 0);
    errno = 0;
    assert(rename("device-rename.tmp", "/dev/replacement") == -1);
    assert(errno == EACCES);
    assert(unlink("device-rename.tmp") == 0);
}
