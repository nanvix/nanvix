/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <netinet/in.h>
#include <stdbool.h>
#include <stdio.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

//==================================================================================================
// Private Functions
//==================================================================================================

// Returns whether two timestamps are equal.
static bool same_time(const struct timespec *a, const struct timespec *b)
{
    return ((a->tv_sec == b->tv_sec) && (a->tv_nsec == b->tv_nsec));
}

// Returns whether two stat results report the same device and inode.
static bool same_file(const struct stat *a, const struct stat *b)
{
    return ((a->st_dev == b->st_dev) && (a->st_ino == b->st_ino));
}

// Returns whether two stat results report the same file with the same timestamps.
static bool same_identity(const struct stat *a, const struct stat *b)
{
    return (same_file(a, b) && same_time(&a->st_atim, &b->st_atim) &&
            same_time(&a->st_mtim, &b->st_mtim) && same_time(&a->st_ctim, &b->st_ctim));
}

// Returns the current time of the clock that timestamps socket creation.
static struct timespec now(void)
{
    struct timespec ts = {0};
    assert(clock_gettime(CLOCK_REALTIME, &ts) == 0);
    return (ts);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests fstat() on sockets.
void test_socket_stat(void)
{
    fprintf(stderr, "testing socket fstat() ... ");

    // Bracket the creation of the socket between two clock readings.
    const struct timespec before = now();
    int sockfd = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    assert(sockfd >= 0);
    struct stat st = {0};
    assert(fstat(sockfd, &st) == 0);
    const struct timespec after = now();

    // A socket reports as a socket, with no size or blocks.
    assert(S_ISSOCK(st.st_mode));
    assert(st.st_size == 0);
    assert(st.st_blocks == 0);

    // All three times are the creation time of the socket.
    assert(same_time(&st.st_atim, &st.st_ctim));
    assert(same_time(&st.st_mtim, &st.st_ctim));
    assert(st.st_ctim.tv_sec >= before.tv_sec);
    assert(st.st_ctim.tv_sec <= after.tv_sec);

    // Distinct sockets have distinct identities.
    int otherfd = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    assert(otherfd >= 0);
    struct stat other = {0};
    assert(fstat(otherfd, &other) == 0);
    assert(S_ISSOCK(other.st_mode));
    assert(!same_file(&other, &st));

    // Sockets are not on the device of pipes.
    int pipefds[2] = {-1, -1};
    assert(pipe(pipefds) == 0);
    struct stat pipe_st = {0};
    assert(fstat(pipefds[0], &pipe_st) == 0);
    assert(pipe_st.st_dev != st.st_dev);
    assert(close(pipefds[0]) == 0);
    assert(close(pipefds[1]) == 0);

    // Repeated calls report the same identity and timestamps, even once the clock has advanced past
    // the creation time of the socket.
    const struct timespec delay = {
        .tv_sec = 1,
        .tv_nsec = 0,
    };
    assert(nanosleep(&delay, NULL) == 0);
    struct stat again = {0};
    assert(fstat(sockfd, &again) == 0);
    assert(same_identity(&again, &st));

    // A duplicated descriptor shares the identity and timestamps of the socket, rather than keeping
    // those of the socket that it displaces.
    assert(dup2(sockfd, otherfd) == otherfd);
    struct stat dup_st = {0};
    assert(fstat(otherfd, &dup_st) == 0);
    assert(same_identity(&dup_st, &st));
    assert(!same_file(&dup_st, &other));
    assert(close(otherfd) == 0);

    // A child inherits the socket with the same identity and timestamps.
    pid_t pid = fork();
    assert(pid >= 0);
    if (pid == 0) {
        struct stat child_st = {0};
        bool inherited = (fstat(sockfd, &child_st) == 0) && same_identity(&child_st, &st);
        _exit(inherited ? 0 : 1);
    }
    int status = 0;
    assert(waitpid(pid, &status, 0) == pid);
    assert(WIFEXITED(status) && (WEXITSTATUS(status) == 0));
    assert(fstat(sockfd, &again) == 0);
    assert(same_identity(&again, &st));
    assert(close(sockfd) == 0);

    // A new socket does not inherit the identity of a closed one, even if it reuses its
    // descriptors.
    int newfd = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    assert(newfd >= 0);
    struct stat new_st = {0};
    assert(fstat(newfd, &new_st) == 0);
    assert(S_ISSOCK(new_st.st_mode));
    assert(!same_file(&new_st, &st));
    assert(close(newfd) == 0);

    fprintf(stderr, "passed\n");
}
