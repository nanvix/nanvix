/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <_ansi.h>
#include <errno.h>
#include <stddef.h>
#include <stdlib.h>
#include <unistd.h>
#include <_syslist.h>
#include <reent.h>

#if defined (unix) || defined (__CYGWIN__)
static int _EXFUN(do_system, (struct _reent *ptr _AND _CONST char *s));
#endif

int
_DEFUN(_system_r, (ptr, s),
     struct _reent *ptr _AND
     _CONST char *s)
{
  ((void)ptr);
#if defined(HAVE_SYSTEM)
  return _system (s);
  ptr = ptr;
#elif defined(NO_EXEC)
  if (s == NULL)
    return 0;
  errno = ENOSYS;
  return -1;
#else

  /* ??? How to handle (s == NULL) here is not exactly clear.
     If _fork_r fails, that's not really a justification for returning 0.
     For now we always return 0 and leave it to each target to explicitly
     handle otherwise (this can always be relaxed in the future).  */

#if defined (unix) || defined (__CYGWIN__)
  if (s == NULL)
    return 1;
  return do_system (ptr, s);
#else
  if (s == NULL)
    return 0;
  errno = ENOSYS;
  return -1;
#endif

#endif
}

#ifndef _REENT_ONLY

/**
 * @brief Issues a command.
 *
 * @details Executes a command specified in command by
 * calling /bin/sh -c command, and returns after the 
 * command has been completed. During execution of the
 * command, SIGCHLD will be blocked, and SIGINT and SIGQUIT
 * will be ignored.
 *
 * @return If the value of command is NULL, system() returns
 * nonzero if the shell is available, and zero if not.
 */
int system(const char *s)
{
  return _system_r (_REENT, s);
}

#endif

#if defined (unix) && !defined (__CYGWIN__) && !defined(__rtems__)
extern char **environ;

/* Only deal with a pointer to environ, to work around subtle bugs with shared
   libraries and/or small data systems where the user declares his own
   'environ'.  */
static char ***p_environ = &environ;

static int
_DEFUN(do_system, (ptr, s),
     struct _reent *ptr _AND
     _CONST char *s)
{
  char *argv[4];
  int pid, status;

  argv[0] = "sh";
  argv[1] = "-c";
  argv[2] = (char *) s;
  argv[3] = NULL;

  if ((pid = _fork_r (ptr)) == 0)
    {
      _execve ("/bin/sh", argv, *p_environ);
      exit (100);
    }
  else if (pid == -1)
    return -1;
  else
    {
      int rc = _wait_r (ptr, &status);
      if (rc == -1)
	return -1;
      status = (status >> 8) & 0xff;
      return status;
    }
}
#endif

#if defined (__CYGWIN__)
static int do_system(struct _reent *ptr, const char *s)
{
  char *argv[4];
  int pid, status;

  argv[0] = "sh";
  argv[1] = "-c";
  argv[2] = (char *) s;
  argv[3] = NULL;

  if ((pid = vfork ()) == 0)
    {
      /* ??? It's not clear what's the right path to take (pun intended :-).
	 There won't be an "sh" in any fixed location so we need each user
	 to be able to say where to find "sh".  That suggests using an
	 environment variable, but after a few more such situations we may
	 have too many of them.  */
      char *sh = getenv ("SH_PATH");
      if (sh == NULL)
	sh = "/bin/sh";
      _execve (sh, argv, environ);
      exit (100);
    }
  else if (pid == -1)
    return -1;
  else
    {
      extern int _wait (int *);
      int rc = _wait (&status);
      if (rc == -1)
	return -1;
      status = (status >> 8) & 0xff;
      return status;
    }
}
#endif
