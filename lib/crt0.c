/* Copyright(C) 2011-2014 Pedro H. Penna  <pedrohenriquepenna@gmail.com>
 * 
 * crt0.c - Program startup.
 */

/*
 * Main routine.
 */
extern int main(int argc, char **argv);

/* Standard output. */
int stdout;

/*
 * Entry point of the program.
 */
void _start(int argc, char **argv)
{
	((void)argc);
	((void)argv);
	
	stdout = open("/dev/tty", O_RDWR);
	
	main(0, NULL);
	
	while(1);
}
