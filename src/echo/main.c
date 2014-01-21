/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * echo - Writes arguments to standard output.
 */

/*
 * Writes arguments to standard output.
 */
int main(int argc, char argv**)
{
	int i;
	
	/* Print strings. */
	for (i = 1; i < argc; i++)
		printf("%s%s", argv[i], (i + 1 < argc) ? " " : "\n");
	
	return (0);
}
