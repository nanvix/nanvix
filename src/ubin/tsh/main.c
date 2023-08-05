/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2016 Davidson Francis <davidsondfgl@gmail.com>
 *              2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdlib.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <termios.h>
#include <ctype.h>

#include "builtin.h"
#include "tsh.h"
#include "history.h"

/* Cursor keys. */
#define KUP    0x97
#define KDOWN  0x98

/* Shell flags. */
int shflags = 0;

/* Shell return value. */
int shret = EXIT_SUCCESS;

/* Input file. */
FILE *input = NULL;

/* TTY modes */
struct termios canonical;
struct termios raw;

/* Global command history stack struct */
static struct history *hist = NULL;

/*
 * Switches to canonical (default) mode.
 */
static void switch_canonical(void)
{
	if (tcsetattr(fileno(stdin), TCSANOW, &canonical) < 0)
	{
		fprintf(stderr, "%s: failed to switch to canonical mode\n", TSH_NAME);
		exit(EXIT_FAILURE);
	}
}

/*
 * Switches to raw mode
 */
static void switch_raw(void)
{
	if (tcsetattr(fileno(stdin), TCSANOW, &raw) < 0)
	{
		fprintf(stderr, "%s: failed to switch to raw mode\n", TSH_NAME);
		exit(EXIT_FAILURE);
	}
}

/*
 * Configures raw mode.
 */
static void configure_tty(void)
{
	/* Get current termios */
	if (tcgetattr(fileno(stdin), &canonical) < 0)
	{
		fprintf(stderr, "%s: failed to get tty options\n", TSH_NAME);
		exit(EXIT_FAILURE);
	}

	/* Configure raw mode. */
	memcpy(&raw, &canonical, sizeof(struct termios));
	raw.c_lflag &= ~(ECHO | ICANON | IEXTEN | ISIG);
	raw.c_cc[VMIN] = 1;
	raw.c_cc[VTIME] = 0;

	switch_raw();
}

/*
 * Prints a syntax error message.
 */
static void syntax(void)
{
	fprintf(stderr, "%s: Syntax error\n", TSH_NAME);
}

/*
 * Prints signal message.
 */
static void sigmsg(int sig)
{
	char *msg;

	/* Parse signal. */
	switch (sig)
	{
		case SIGKILL: msg = "Killed"; break;
		case SIGABRT: msg = "Aborted"; break;
		case SIGBUS:  msg = "Bus error"; break;
		case SIGFPE:  msg = "Bad math"; break;
		case SIGHUP:  msg = "Hangup"; break;
		case SIGILL:  msg = "Illegal instruction"; break;
		case SIGINT:  msg = "Interrupted"; break;
		case SIGPIPE: msg = "Broken pipe"; break;
		case SIGQUIT: msg = "Quit"; break;
		case SIGSEGV: msg = "Memory violation"; break;
		case SIGTERM: msg = "Terminated"; break;
		case SIGALRM: msg = "Alarm"; break;
		case SIGTRAP: msg = "Trace"; break;
		default:      msg = "Other signal"; break;
	}

	fputs(msg, stderr);
#ifdef WIFCORED
	if (WIFCORED(s))
		fputs(" -- Core dumped", stderr);
#endif /* WIFCORED */
	fputs("\n", stderr);
}

/*
 * Closes redirection files.
 */
static void closeredir(int *redir)
{
	if (redir[0] != -1)
		close(redir[0]);
	if (redir[1] != -1)
		close(redir[1]);
}

/*
 * Runs a command.
 */
static void runcmd(const char **args, int argc, int *redir, int flags)
{
	int i;         /* Loop index.       */
	int status;    /* Exit status.      */
	pid_t pid;     /* Child process ID. */
	builtin_t cmd; /* Built-in command. */

	/* Checks built-in. */
	if ((cmd = getbuiltin(args[0])) != NULL)
	{
		closeredir(redir);
		if ((shret = cmd(argc, args)) != EXIT_SUCCESS)
			sherror();
		return;
	}

	pid = fork();

	/* Failed to fork. */
	if (pid < 0)
	{
		fprintf(stderr, "%s: failed to fork child\n", TSH_NAME);
		shret = errno;
		goto error;
	}

	/* Parent process. */
	if (pid > 0)
	{
		closeredir(redir);

		/* Piping... */
		if (flags & CMD_PIPE)
			return;

		/* Asynchronous execution. */
		if (flags & CMD_ASYNC)
		{
			printf("[%d]+\n", pid);
			return;
		}

		/* Wait child. */
		while (wait(&status) != pid)
			/* noop */;

		/* Abnormal termination. */
		if (status != EXIT_SUCCESS)
		{
			/* Signal. */
			if (WIFSIGNALED(status))
				sigmsg(shret = WTERMSIG(status));

			/* Voluntary. */
			else if (WIFEXITED(status))
				shret = WEXITSTATUS(status);

			/* Stopped. */
			else if  (WIFSTOPPED(status))
				printf("[%d]+\tStopped\n", pid);
		}

		return;
	}

	/*
	 * Child process.
	 */

	/* Reset signals. */
	signal(SIGINT, SIG_DFL);
	signal(SIGQUIT, SIG_DFL);
	signal(SIGTERM, SIG_DFL);
	signal(SIGTSTP, SIG_DFL);
	if (flags & CMD_ASYNC)
	{
		signal(SIGINT, SIG_IGN);
		signal(SIGQUIT, SIG_IGN);
		if (redir[0] == -1)
		{
			close(0);
			open("/dev/null", O_RDONLY);
		}
	}

	/* Redirections. */
	for (i = 0; i < 2; i++)
	{
		if (redir[i] != -1)
		{
			if (dup2(redir[i], i) == -1)
			{
				fprintf(stderr, "%s: failed to redirect\n", args[0]);
				exit(EXIT_FAILURE);
			}
			close(redir[i]);
		}
	}

	execvp(args[0], (char * const *)args);
	fprintf(stderr, "%s: failed to execute\n", args[0]);
	exit(EXIT_FAILURE);

error:
	sherror();
	closeredir(redir);
}

/*
 * Eat characters.
 */
static char *eatchars(char *str)
{
	/* Eat characters. */
	while (*++str != '\0')
	{
		if ((*str == '<') || (*str == '>'))
			break;

		if ((*str == ' ') || (*str == '\t'))
			break;
	}

	return (str);
}

/*
 * Gets redirection file name.
 */
static int getredir(char **redir, char *str)
{
	char *base;

	base = str;
	*base = '\0';

	/* Eat initial whitespace. */
	while (*++str != '\0')
	{
		if ((*str == ' ') || (*str == '\t'))
			continue;

		break;
	}

	/* Syntax error. */
	if ((*str == '<') || (*str == '>'))
	{
		syntax();
		return (-1);
	}

	/* Early EOF. */
	if (*str == '\0')
	{
		syntax();
		return (-1);
	}

	*redir = str;
	str = eatchars(str);

	return (str - base);
}

/*
 * Parses a command block
 */
static void pcmd(char *cmd, int *redir, int flags)
{
	int n;                             /* Number of characters to skip. */
	char *p;                           /* Working character.            */
	int argc;                          /* Arguments count.              */
	char *infile;                      /* Input file for redirection.   */
	char *outfile;                     /* Output file for redirection.  */
	const char *args[CMD_MAXARGS + 1]; /* Command arguments.            */

	/* File permissions. */
	#define MAY_READ (S_IRUSR | S_IRGRP | S_IROTH)
	#define MAY_WRITE (S_IWUSR | S_IWGRP | S_IWOTH)

	argc = 0;
	infile = NULL;
	outfile = NULL;

	/*
	 * Parse command breaking it
	 * down into arguments.
	 */
	for (p = cmd; *p != '\0'; /* empty*/)
	{
		/* Parse character. */
		switch (*p)
		{
			/* Eat whitespace. */
			case ' ':
			case '\t':
				*p++ = '\0';
				break;

			/* Redirect input. */
			case '<':
				if ((argc == 0) || (infile != NULL))
				{
					syntax();
					goto error1;
				}
				if ((n = getredir(&infile, p)) < 0)
					goto error1;
				p += n;
				break;

			/* Redirect output. */
			case '>':
				if ((argc == 0) || (outfile != NULL))
				{
					syntax();
					goto error1;
				}
				if ((n = getredir(&outfile, p)) < 0)
					goto error1;
				p += n;
				break;

			/* Get argument. */
			default:
				/* Too many command arguments. */
				if (argc == CMD_MAXARGS)
				{
					fprintf(stderr, "%s: too many arguments\n", args[0]);
					goto error1;
				}

				args[argc++] = p;
				args[argc] = NULL;

				/* Eat command characters. */
				p = eatchars(p);
				break;
		}
	}

	/* Ignore empty commands. */
	if (argc > 0)
	{
		/* Input redirection. */
		if (infile != NULL)
		{
			if (redir[0] != -1)
				close(redir[0]);

			redir[0] = open(infile, O_RDONLY);

			if (redir[0] == -1)
			{
				fprintf(stderr, "%s: failed to open\n", infile);
				shret = errno;
				goto error0;
			}
		}

		/* Output redirection. */
		if (outfile != NULL)
		{
			if (redir[1] != -1)
				close(redir[1]);

			redir[1] = open(outfile, O_WRONLY|O_CREAT|O_TRUNC, MAY_READ|MAY_WRITE);

			if (redir[1] == -1)
			{
				fprintf(stderr, "%s: failed to open\n", outfile);
				shret = errno;
				goto error0;
			}
		}

		runcmd((const char **)args, argc, redir, flags);
		return;
	}

error1:
	shret = EXIT_FAILURE;
error0:
	sherror();
	closeredir(redir);
}

/*
 * Parses a pipe block.
 */
static void ppipe(char *pipeblk, int flags)
{
	char *p;         /* Working character.            */
	int pipefd[2];   /* Pipe file descriptors.        */
	int redir[2]; /* Redirection file descriptors. */
	char *lastcmd;   /* Last command found.           */

	pipefd[0] = pipefd[1] = -1;
	redir[0] = redir[1] = -1;

	/*
	 * Parse pipe block breaking it
	 * down into commands.
	 */
	for (p = lastcmd = pipeblk; /* empty */ ; p++)
	{
		/* Check syntax. */
		switch (*lastcmd)
		{
			/* Syntax error. */
			case '|':
				syntax();
				goto error1;

			/* Syntax error. */
			case '\0':
				syntax();
				goto error1;

			/* Keep parsing. */
			default:
				break;
		}

		/* Parse character. */
		switch (*p)
		{
			/* Pipe. */
			case '|':
				*p = '\0';
				redir[0] = pipefd[0];
				if (pipe(pipefd) == -1)
				{
					fprintf(stderr, "%s: failed to pipe\n", TSH_NAME);
					shret = errno;
					goto error0;
				};
				redir[1] = pipefd[1];
				pcmd(lastcmd, redir, flags | CMD_PIPE);
				lastcmd = p + 1;
				break;

			/* Done parsing. */
			case '\0':
				redir[0] = pipefd[0];
				redir[1] = -1;
				pcmd(lastcmd, redir, flags);
				return;

			/* Keep parsing. */
			default:
				break;
		}
	}

error1:
	shret = EXIT_FAILURE;
error0:
	sherror();
	closeredir(redir);
}

/*
 * Parses a command line.
 */
static void pline(char *line)
{
	char *p;        /* Working character.     */
	char *lastpipe; /* Last pipe block found. */

	/*
	 * Parse command line breaking it
	 * down into pipe blocks.
	 */
	for (p = lastpipe = line; /* empty */ ; p++)
	{
		/* Check syntax. */
		switch (*lastpipe)
		{
			/* Fall through. */
			case '&':
				shret = EXIT_FAILURE;
				syntax();
				sherror();
			case '\0':
				return;

			/* Keep parsing. */
			default:
				break;
		}

		/* Parse character. */
		switch (*p)
		{
			/* Parse pipe block. */
			case '&':
				*p = '\0';
				ppipe(lastpipe, CMD_ASYNC);
				lastpipe = p + 1;
				break;

			/* Done parsing */
			case '\0':
				ppipe(lastpipe, 0);
				return;

			/* Keep parsing. */
			default:
				break;
		}
	}
}

/*
 * Cheks if a string has at least one graph character.
 */
static int has_graph(const char *str)
{
	for (/* noop */; *str != '\0'; str++)
	{
		if (isgraph(*str))
				return (1);
	}

	return (0);
}

/*
 * Reads a command line.
 */
static int readline(char *line, int length, FILE *stream)
{
	int fd;   /* File descriptor.                */
	int size; /* # of characters left in buffer. */
	char *p;  /* Write pointer.                  */

	fd = fileno(stream);
	size = length;
	p =  line;

	while (size > 0)
	{
		unsigned char ch;

		/* Nothing read. */
		if (read(fd, &ch, 1) != 1)
			return (-1);

		/* Erase. */
		if ((ch == ERASE_CHAR(raw)) && (size < length))
		{
			*p-- = '\0';
			size++;
			putchar(ch);
		}

		/* Kill. */
		else if (ch == KILL_CHAR(raw))
		{
			/* Clear buffer. */
			while (size < length)
				putchar('\b'), size++;
			p = line;
		}

		/* End of file. */
		else if (ch == EOF_CHAR(raw))
			return (0);

		/* UP and DOWN. */
		else if ((ch == KUP) || (ch == KDOWN))
		{
			char *oldline;

			oldline = (ch == KUP) ? history_previous(hist) : history_next(hist);

			/* Clear actual command & screen */
			while (size < length)
				putchar('\b'), size++;

			/* Restore last command */
			size = length - strlen(oldline);
			p = line + strlen(oldline);
			strncpy(line, oldline, LINELEN);
			printf("%s", line);
		}

		/* Keys */
		else
		{
			/* Check for line overflow. */
			if (size == 1)
			{
				*p++ = '\0';
				break;
			}

			/* End of line. */
			if (ch == EOL_CHAR(raw))
			{
				*p++ = '\0';
				/* Add command to stack. */
				if (has_graph(line))
					history_push(hist, line);
				size--;
				putchar('\n');
				break;
			}

			/* Printable character. */
			if (isprint(ch))
			{
				*p++ = ch;
				size--;
				putchar(ch);
			}
		}
	}

	return (1);
}

/*
 * Prints program usage and exits.
 */
static void usage(void)
{
	printf("Usage: %s [options]\n", TSH_NAME);
	printf("Options:\n");
	printf("  <command file> Command file to read from\n");
	printf("  --help         Displays this information and exits\n");
	printf("  --version      Prints program version and exits\n");

	exit(EXIT_SUCCESS);
}

/*
 * Read program arguments.
 */
static void readargs(int argc, char **argv)
{
	int i;        /* Loop index.       */
	char *arg;    /* Working argument. */
	char *infile; /* Input file name.  */

	infile = NULL;

	/* Read program arguments. */
	for (i = 0; i < argc; i++)
	{
		arg = argv[i];

		/* Print version. */
		if (!strcmp(arg, "--version"))
		{
			printf("%s %s\n", TSH_NAME, TSH_VERSION);
			puts(SH_COPYRIGHT);
			exit(EXIT_SUCCESS);
		}

		/* Display help. */
		else if (!strcmp(arg, "--help"))
			usage();

		/* Set input. */
		else
			infile = arg;
	}

	/* Read from standard input. */
	if (input == NULL)
	{
		input = stdin;

		/* Interactive shell. */
		if (isatty(fileno(stdin)))
		{
			shflags |= SH_INTERACTIVE;
			signal(SIGINT, SIG_IGN);
			signal(SIGQUIT, SIG_IGN);
			signal(SIGTERM, SIG_IGN);
			signal(SIGTSTP, SIG_IGN);
			setvbuf(stdout, NULL, _IONBF, 0);
		}
	}

	/* Read input from file. */
	else
	{
		input = freopen(infile, "r", stdin);

		/* Failed to open input file. */
		if (input == NULL)
		{
			fprintf(stderr, "%s: failed to open\n", infile);
			exit(errno);
		}
	}
}

/*
 * Interprets and executes commands.
 */
int main(int argc, char **argv)
{
#ifdef OPEN_MAX
	int i;              /* Loop index.   */
	uid_t myuid;        /* uid of shell. */
	char line[LINELEN]; /* Input line.   */
#else
	uid_t myuid;        /* uid of shell. */
	char line[LINELEN]; /* Input line.   */
#endif /* OPEN_MAX */

	readargs(argc, argv);

	myuid = getuid();

#ifdef OPEN_MAX

	/* Make sure other files are close. */
	for (i = 3; i < OPEN_MAX; i++)
		close(i);

#endif /* OPEN_MAX */

	/* Print copyright message. */
	if (shflags & SH_INTERACTIVE)
		puts(SH_COPYRIGHT);

	/* Configure tty to work in raw mode. */
	configure_tty();

	/* Initialize command stack. */
	hist = history_init(HISTORY_SIZE);

	/* Read and interpret commands. */
	while (1)
	{
		/* Print prompt character. */
		if (shflags & SH_INTERACTIVE)
			printf("%c ", (myuid == 0) ? '#' : '%');

		/* Read command line. */
		switch (readline(line, LINELEN, input))
		{
			/* Error while reading. */
			case -1:
				sherror();
				break;

			/* End of file. */
			case 0:
				puts("\n");
				goto out;

			/* Parse command line. */
			case 1:
				switch_canonical();
				pline(line);
				switch_raw();
				break;
		}
	}

out:

	switch_canonical();
	return (shret);
}
