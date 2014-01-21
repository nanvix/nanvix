/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sh - Nanvix shell.
 */

#include <stdio.h>

/*
 * Reads and execute commands.
 */
int main(int argc, char **argv)
{
	char line[100];
	
	((void)argc);
	((void)argv);
	
	printf("Nanvix Shell - Copyright(C) 2011-2014 Pedro H. Penna\n");
	
	while (1)
	{
		printf("$ ");
		gets(line);
		puts(line);
	}
	
	return (0);
}
