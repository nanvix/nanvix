/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Regression test for signal interruption after an RPC request is committed.
 *
 * The parent fills a pipe and blocks while writing one more byte. One child
 * exits to deliver SIGCHLD without SA_RESTART; another child drains one byte
 * later so vfsd can complete the already-submitted write. The write must resume
 * waiting for that response instead of returning EINTR and stranding it in the
 * parent mailbox. A final fork verifies that the mailbox remains empty.
 */

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <string.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#define CHECK(step, condition)                                                 \
  do {                                                                         \
    if (!(condition)) {                                                        \
      return (step);                                                           \
    }                                                                          \
  } while (0)

static volatile sig_atomic_t signal_count;

static void handle_signal(int signum) {
  if (signum == SIGCHLD) {
    signal_count++;
  }
}

static int wait_for_child(pid_t child) {
  int status = 0;
  pid_t result;
  do {
    result = waitpid(child, &status, 0);
  } while (result == -1 && errno == EINTR);

  return result == child && WIFEXITED(status) && WEXITSTATUS(status) == 0;
}

int main(int argc, char *argv[]) {
  (void)argc;
  (void)argv;

  struct sigaction action;
  struct sigaction old_action;
  memset(&action, 0, sizeof(action));
  action.sa_handler = handle_signal;
  CHECK(1, sigemptyset(&action.sa_mask) == 0);
  CHECK(2, sigaction(SIGCHLD, &action, &old_action) == 0);

  int pipefds[2];
  CHECK(3, pipe(pipefds) == 0);

  int startfds[2];
  CHECK(4, pipe(startfds) == 0);

  int flags = fcntl(pipefds[1], F_GETFL);
  CHECK(5, flags != -1);
  CHECK(6, fcntl(pipefds[1], F_SETFL, flags | O_NONBLOCK) == 0);

  static const char fill[4096];
  for (;;) {
    ssize_t written = write(pipefds[1], fill, sizeof(fill));
    if (written == -1) {
      CHECK(7, errno == EAGAIN);
      break;
    }
    CHECK(8, written == (ssize_t)sizeof(fill));
  }
  CHECK(9, fcntl(pipefds[1], F_SETFL, flags & ~O_NONBLOCK) == 0);

  pid_t signal_child = fork();
  CHECK(10, signal_child != -1);
  if (signal_child == 0) {
    const struct timespec delay = {
        .tv_sec = 0,
        .tv_nsec = 10000000,
    };
    char start;
    if (close(startfds[1]) != 0 || close(pipefds[0]) != 0 ||
        close(pipefds[1]) != 0 || read(startfds[0], &start, 1) != 1 ||
        close(startfds[0]) != 0 || nanosleep(&delay, NULL) != 0) {
      _exit(1);
    }

    _exit(0);
  }

  pid_t drain_child = fork();
  CHECK(11, drain_child != -1);
  if (drain_child == 0) {
    const struct timespec delay = {
        .tv_sec = 0,
        .tv_nsec = 50000000,
    };
    char start;
    char byte;
    if (close(startfds[1]) != 0 || close(pipefds[1]) != 0 ||
        read(startfds[0], &start, 1) != 1 || close(startfds[0]) != 0 ||
        nanosleep(&delay, NULL) != 0 || read(pipefds[0], &byte, 1) != 1 ||
        close(pipefds[0]) != 0) {
      _exit(2);
    }
    _exit(0);
  }

  CHECK(12, close(startfds[0]) == 0);
  signal_count = 0;
  const char start[2] = {'s', 'd'};
  CHECK(13, write(startfds[1], start, sizeof(start)) == (ssize_t)sizeof(start));
  CHECK(14, close(startfds[1]) == 0);

  const char byte = 'x';
  errno = 0;
  CHECK(15, write(pipefds[1], &byte, 1) == 1);
  CHECK(16, signal_count > 0);
  CHECK(17, wait_for_child(signal_child));
  CHECK(18, wait_for_child(drain_child));
  CHECK(19, close(pipefds[0]) == 0);
  CHECK(20, close(pipefds[1]) == 0);

  pid_t child = fork();
  CHECK(21, child != -1);
  if (child == 0) {
    _exit(0);
  }
  CHECK(22, wait_for_child(child));
  CHECK(23, sigaction(SIGCHLD, &old_action, NULL) == 0);

  return (0);
}