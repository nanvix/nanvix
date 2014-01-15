/* Copyright(C) 2011-2014 Pedro H. Penna  <pedrohenriquepenna@gmail.com>
 * 
 * crt0.c - Program startup.
 */

#include <unistd.h>

/*
 * Main routine.
 */
extern int main(int argc, char **argv);

/*
 * Entry point of the program.
 */
void _start(int argc, char **argv, char **envp)
{
	environ = envp;
	
	_exit(main(argc, argv));
}
