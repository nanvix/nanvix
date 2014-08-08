/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * login - Logins a user
 * 
 * The login command logs a user in the Nanvix operating system.
 */

#include <dev/tty.h>
#include <stdio.h>
#include <stdlib.h>
#include <stropts.h>
#include <unistd.h>

/*
 * Logins a user.
 */
int main(int argc, char *const argv[])
{
	char *arg[2];
	
	((void)argc);
	((void)argv);

	arg[0] = "-";
	arg[1] = NULL;
	
	(void) setvbuf(stdin, NULL, _IONBF, 0);
	(void) setvbuf(stdout, NULL, _IOLBF, 0);
	
	ioctl(fileno(stdout), TTY_CLEAR);
	
	printf("Nanvix - A Free Educational Operating System\n\n");
	printf("The programs included with Nanvix system are free software\n");
	printf("under the GNU General Public License Version 3.\n\n");
	printf("Nanvix comes with ABSOLUTELY NO WARRANTY, to the extent\n");
	printf("permitted by applicable law.\n\n");
	fflush(stdout);

	execve("/bin/tsh", (char *const*)arg, (char *const*)environ);
	
	return (EXIT_SUCCESS);
}
