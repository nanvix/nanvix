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
	
	puts("Nanvix Shell - Copyright(C) 2011-2014 Pedro H. Penna");
	
	while (1)
	{
		puts("$ ");
		gets(line);
	}
	
	return (0);
}
