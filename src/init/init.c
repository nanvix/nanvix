/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * init.c - System's init process.
 */

#include <errno.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>

/*
 * Spawns other system process.
 */
int main(int argc, char **argv)
{
	int status;   /* Child exit status code. */
	pid_t pid;    /* Child process ID.       */
	char *arg[2]; /* Child arguments.        */	
	
	((void)argc);
	((void)argv);
	
	/* Open standard output streams. */
	open("/dev/tty", O_RDWR);
	dup(0);
	dup(0);

	arg[0] = "-";
	arg[1] = NULL;
	
	puts("Hello World!");

	/* init never ends. */
	while (1)
	{
		pid = fork();
			
		/* Failed to fork. */
		if (pid < 0)
		{
			puts("fork() failed on init");
			continue;
		}
			
		/* Child process. */
		if (pid == 0)
			_exit(execve("/bin/sh", (const char **)arg, (const char **)environ));			
		
		while(1);
			
		/* Wait child processes. */
		while (1)
		{
			if (pid == wait(&status))
				break;
		
		}
		
		puts("\nchild process died");
		sync();
	}
	
	return (0);
}
