/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_UNISTD_H
#define _NANVIX_UNISTD_H

/**
 * @file unistd.h
 * @brief Standard symbolic constants and types.
 *
 * Declares miscellaneous standard constants and the core POSIX process and I/O
 * interfaces. Constants mirror the Rust definitions in the sysapi crate
 * (unistd.rs); prototypes are generated from the syscall and posix crates.
 */

#include <stddef.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

/* POSIX feature-test macros. */
#ifndef _POSIX_VERSION
#define _POSIX_VERSION 200809L
#endif
#ifndef _POSIX_THREADS
#define _POSIX_THREADS 200809L
#endif

#define STDIN_FILENO 0 /**< File descriptor of standard input. */
#define STDOUT_FILENO 1 /**< File descriptor of standard output. */
#define STDERR_FILENO 2 /**< File descriptor of standard error. */
#ifndef SEEK_SET
#define SEEK_SET 0 /**< Seek relative to start-of-file. */
#endif

#ifndef SEEK_CUR
#define SEEK_CUR 1 /**< Seek relative to current position. */
#endif

#ifndef SEEK_END
#define SEEK_END 2 /**< Seek relative to end-of-file. */
#endif

#define _SC_ARG_MAX 0 /**< Maximum length of arguments to exec(). */
#define _SC_CHILD_MAX 1 /**< Maximum number of processes per user. */
#define _SC_CLK_TCK 2 /**< Number of clock ticks per second. */
#define _SC_NGROUPS_MAX 3 /**< Maximum number of supplementary group IDs. */
#define _SC_OPEN_MAX 4 /**< Maximum number of open files per process. */
#define _SC_VERSION 7 /**< POSIX.1 version. */
#define _SC_PAGESIZE 8 /**< Size of a page in bytes. */
#define _SC_PAGE_SIZE 8 /**< Size of a page in bytes (alias of _SC_PAGESIZE). */
#define _SC_NPROCESSORS_CONF 9 /**< Number of configured processors. */
#define _SC_NPROCESSORS_ONLN 10 /**< Number of online processors. */
#define _SC_PHYS_PAGES 11 /**< Total number of physical pages. */
#define _SC_AVPHYS_PAGES 12 /**< Number of available physical pages. */

/*==================================================================================================
 * File and Process Operations
 *==================================================================================================*/

extern ssize_t read(int fd, void *buffer, size_t count);
extern ssize_t write(int fd, const void *buffer, size_t count);
extern int close(int fd);
extern void _exit(int status) __attribute__((__noreturn__));
extern int pipe(int pipefd[2]);
extern int dup(int oldfd);
extern int dup2(int oldfd, int newfd);
extern pid_t fork(void);
extern int execve(const char *path, char *const argv[], char *const envp[]);
extern int execvp(const char *file, char *const argv[]);
extern int execv(const char *path, char *const argv[]);
extern int execl(const char *path, const char *arg, ...);
extern int execlp(const char *file, const char *arg, ...);
extern int execle(const char *path, const char *arg, ...);
extern pid_t getppid(void);
extern int faccessat(int dirfd, const char *pathname, int mode, int flags);
extern int fchownat(int dirfd, const char *pathname, uid_t owner, gid_t group, int flags);
extern int lchown(const char *pathname, uid_t owner, gid_t group);
extern int linkat(int olddirfd, const char *oldpath, int newdirfd, const char *newpath, int flags);
extern int symlinkat(const char *target, int newdirfd, const char *linkpath);
extern ssize_t readlinkat(int dirfd, const char *pathname, char *buf, size_t bufsiz);
extern int truncate(const char *path, off_t length);
extern long fpathconf(int fd, int name);
extern long pathconf(const char *path, int name);

extern char **environ;
extern off_t lseek(int fd, off_t offset, int whence);
extern int unlink(const char *path);
extern int link(const char *oldpath, const char *newpath);
extern int chroot(const char *path);
extern int rmdir(const char *path);
extern long sysconf(int name);

/*==================================================================================================
 * User and Group Identification
 *==================================================================================================*/

extern pid_t getpid(void);
extern uid_t getuid(void);
extern uid_t geteuid(void);
extern gid_t getgid(void);
extern gid_t getegid(void);
extern int setuid(uid_t uid);
extern int seteuid(uid_t uid);
extern int setgid(gid_t gid);
extern int setegid(gid_t gid);
extern int setresuid(uid_t ruid, uid_t euid, uid_t suid);
extern int setresgid(gid_t rgid, gid_t egid, gid_t sgid);
extern int setgroups(size_t size, const gid_t *list);
extern int gethostname(char *name, size_t namelen);
extern int chown(const char *path, uid_t owner, gid_t group);
extern int fchown(int fd, uid_t owner, gid_t group);

/*==================================================================================================
 * File System Operations
 *==================================================================================================*/

extern ssize_t pread(int fd, void *buffer, size_t count, off_t offset);
extern ssize_t pwrite(int fd, const void *buffer, size_t count, off_t offset);
extern int ftruncate(int fd, off_t length);
extern int fsync(int fd);
extern int fdatasync(int fd);
extern char *getcwd(char *buf, size_t size);
extern int chdir(const char *path);
extern int fchdir(int fd);

/* access() mode bits. */
#define F_OK 0
#define X_OK 1
#define W_OK 2
#define R_OK 4
extern int access(const char *path, int amode);
extern unsigned int sleep(unsigned int seconds);
extern unsigned int alarm(unsigned int seconds);
extern int usleep(unsigned int usec);
extern int unlinkat(int dirfd, const char *pathname, int flags);
extern int isatty(int fd);
extern ssize_t readlink(const char *path, char *buf, size_t bufsize);
extern int getentropy(void *buffer, size_t length);
extern int symlink(const char *target, const char *linkpath);

/*==================================================================================================
 * Command-Line Option Parsing State
 *==================================================================================================*/

extern char *optarg;
extern int optind;
extern int opterr;
extern int optopt;

/*==================================================================================================
 * Command-Line Option Parsing
 *==================================================================================================*/

extern int getopt(int argc, char *const argv[], const char *optstring);

/*==================================================================================================
 * Process, Session, and System Operations
 *==================================================================================================*/

extern pid_t setsid(void);
extern pid_t getsid(pid_t pid);
extern int setpgid(pid_t pid, pid_t pgid);
extern int setpgrp(void);
extern pid_t getpgid(pid_t pid);
extern pid_t getpgrp(void);
extern int tcsetpgrp(int fd, pid_t pgrp);
extern pid_t tcgetpgrp(int fd);
extern int nice(int inc);
extern int sethostname(const char *name, size_t len);
extern pid_t vfork(void);
extern void sync(void);
extern int syncfs(int fd);
extern int getgroups(int size, gid_t *list);
extern long gethostid(void);
extern int getlogin_r(char *buf, size_t bufsize);
extern int ttyname_r(int fd, char *buf, size_t buflen);
extern int getpagesize(void);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_UNISTD_H */
