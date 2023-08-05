/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#include <sys/types.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */

/* Default signal name. */
#define SIGNAME_DEFAULT "SIGTERM"

/*
 * Program arguments.
 */
static struct
{
	int sig;   /* Signal number. */
	pid_t pid; /* ID of process. */
} args = { 0, 0 };

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("kill (Nanvix Coreutils) %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
	printf("Copyright(C) 2011-2014 Pedro H. Penna\n");
	printf("This is free software under the ");
	printf("GNU General Public License Version 3.\n");
	printf("There is NO WARRANTY, to the extent permitted by law.\n\n");

	exit(EXIT_SUCCESS);
}

/*
 * Prints program usage and exits.
 */
static void usage(void)
{
	printf("Usage: kill [options] <pid>\n\n");
	printf("Brief: Sends a signal to a process.\n\n");
	printf("Options:\n");
	printf("  --help             Display this information and exit\n");
	printf("  --signal <signame> Signal to be sent\n");
	printf("  --version          Display program version and exit\n");

	exit(EXIT_SUCCESS);
}

/*
 * Gets signal number.
 */
static int getsig(char *signame)
{
	if (!strcmp(signame, "SIGKILL"))     return(SIGKILL);
	else if (!strcmp(signame, "SIGSTOP")) return(SIGSTOP);
	else if (!strcmp(signame, "SIGURG"))  return(SIGURG);
	else if (!strcmp(signame, "SIGABRT")) return(SIGABRT);
	else if (!strcmp(signame, "SIGBUS"))  return(SIGBUS);
	else if (!strcmp(signame, "SIGCHLD")) return(SIGCHLD);
	else if (!strcmp(signame, "SIGCONT")) return(SIGCONT);
	else if (!strcmp(signame, "SIGFPE"))  return(SIGFPE);
	else if (!strcmp(signame, "SIGHUP"))  return(SIGHUP);
	else if (!strcmp(signame, "SIGILL"))  return(SIGILL);
	else if (!strcmp(signame, "SIGINT"))  return(SIGINT);
	else if (!strcmp(signame, "SIGPIPE")) return(SIGPIPE);
	else if (!strcmp(signame, "SIGQUIT")) return(SIGQUIT);
	else if (!strcmp(signame, "SIGSEGV")) return(SIGSEGV);
	else if (!strcmp(signame, "SIGTERM")) return(SIGTERM);
	else if (!strcmp(signame, "SIGTSTP")) return(SIGTSTP);
	else if (!strcmp(signame, "SIGTTIN")) return(SIGTTIN);
	else if (!strcmp(signame, "SIGTTOU")) return(SIGTTOU);
	else if (!strcmp(signame, "SIGALRM")) return(SIGALRM);
	else if (!strcmp(signame, "SIGUSR1")) return(SIGUSR1);
	else if (!strcmp(signame, "SIGUSR2")) return(SIGUSR2);
	else if (!strcmp(signame, "SIGTRAP")) return(SIGTRAP);

	return (-1);
}

/*
 * Gets program arguments.
 */
static void getargs(int argc, char *const argv[])
{
	int i;         /* Loop index.         */
	char *arg;     /* Current argument.   */
	int state;     /* Processing state.   */
	char *signame; /* Name of the signal. */

	/* State values. */
	#define READ_ARG 0 /* Read argument.  */
	#define SET_SIG  1 /* Set signal.     */

	signame = SIGNAME_DEFAULT;
	state = READ_ARG;

	/* Read command line arguments. */
	for (i = 1; i < argc; i++)
	{
		arg = argv[i];

		/* Set value. */
		if (state != READ_ARG)
		{
			switch (state)
			{
				/* Set signal number. */
				case SET_SIG:
					signame = arg;
					state = READ_ARG;
					break;

				/* Bad usage.*/
				default:
					usage();
			}

			continue;
		}

		/* Parse command line argument. */
		if (!strcmp(arg, "--help")) {
			usage();
		}
		else if (!strcmp(arg, "--signal")) {
			state = SET_SIG;
		}
		else if (!strcmp(arg, "--version")) {
			version();
		}
		else {
			args.pid = atoi(arg);
		}
	}

	args.sig = getsig(signame);

	/* Bad signal number. */
	if (args.sig < 0)
	{
		fprintf(stderr, "kill: unknown signal number\n");
		exit(EXIT_FAILURE);
	}
}

/*
 * Sends a signal to a process.
 */
int main(int argc, char *const argv[])
{
	getargs(argc, argv);

	kill(args.pid, args.sig);

	return (EXIT_SUCCESS);
}
