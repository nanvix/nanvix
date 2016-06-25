/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

#include <stdio.h>
#include <unistd.h>
#include <string.h>
#include <fcntl.h>
#include <stdlib.h>
#include <framebuffer.h>

/* Software versioning. */
#define VERSION_MAJOR 1 /* Major version. */
#define VERSION_MINOR 0 /* Minor version. */
#define MAX_LINE 20

/*
 * Program arguments.
 */
static struct
{
	char *filename; /* File. */
} args = { NULL };

unsigned char *fileBuff;

/*
 * Prints program version and exits.
 */
static void version(void)
{
	printf("ppmv - PPM Viewer %d.%d\n\n", VERSION_MAJOR, VERSION_MINOR);
	printf("Copyright(C) 2016-2016 Davidson Francis\n");
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
	printf("Usage: ppmv <file>\n\n");
	printf("Brief: Draws a PPM Image.\n\n");
	printf("Options:\n");
	printf("  --help             Display this information and exit\n");
	printf("  --version          Display program version and exit\n");
	
	exit(EXIT_SUCCESS);
}

/*
 * Gets program arguments.
 * @param argc Arguments count.
 * @param argv Argument list.
 */
static void getargs(int argc, char *const argv[])
{
	int i;     /* Loop index.       */
	char *arg; /* Current argument. */
		
	/* Read command line arguments. */
	for (i = 1; i < argc; i++)
	{
		arg = argv[i];
		
		/* Parse command line argument. */
		if (!strcmp(arg, "--help")) {
			usage();
		}
		else if (!strcmp(arg, "--version")) {
			version();
		}
		else {
			args.filename = arg;
		}
	}
	
	/* Missing argument. */
	if ((args.filename == NULL))
	{
		fprintf(stderr, "ppmv: missing argument\n");
		usage();
	}
}

/* -------------------------------------------------*
 *               Util functions.                    *
 * -------------------------------------------------*/

/*
 * Advance the pointer until find new line.
 * @param i Start point.
 * @returns Where it stopped.
 */
int skipLine(int i)
{
	for (; ; i++)
		if (fileBuff[i] == '\n')
			break;
	
	return (i+1);
}

/*
 * Gets a number by string.
 * @param dest where to put the number.
 * @param start start point.
 * @returns Where it stopped.
 */
int readNumber(char *dest, int start)
{
	for (; ; start++)
	{
		if (fileBuff[start] >= '0' && fileBuff[start] <= '9')
			*dest++ = fileBuff[start];
		else
			break;
	}
	*dest = '\0';
	return start;
}

/*
 * Convert string number to int.
 * @param String to convert.
 * @returns Number in decimal.
 *
 * @note The original atoi it seems not work
 * if the number has leading zeros.
 */
int atoii(char *src)
{
	int number = 0;
	int multiply = 1;

	for(int i = strlen(src)-1; i >= 0; i--, multiply *= 10)
		number = number + ( (src[i]-'0') * multiply);
	
	return number;
}

/*
 * Read and draw an image from a specified file.
 */
int drawImage(char* file)
{
	int i;                     /* Loop index.           */
	char line[MAX_LINE];       /* Max line size.        */
	int size;                  /* File size.            */
	int fd;                    /* File descriptor.      */
	int width;                 /* Image width.          */
	int height;                /* Image height.         */
	int colorMax;              /* Max amount of colors. */
	struct frameBuff_opts fob; /* Buffer image.         */

	/* Open the file. */
	fd = open(file,O_RDONLY);
	if( fd < 0 )
	{
		fprintf(stderr,"Failed to open the file, this file exists?\n");
		return (-1);
	}

	/* Gets file size. */
	size = lseek(fd, 0, SEEK_END);
	lseek(fd, 0, SEEK_SET);
	fileBuff = malloc(size*sizeof(unsigned char));

	/* Read the entire file. */
	if( read(fd, fileBuff, size) != size )
	{
		fprintf(stderr,"An error ocurred while reading file!\n");
		return (-1);
	}

	/* Magic number. */
	if( fileBuff[0] != 'P' || fileBuff[1] != '6' )
	{
		fprintf(stderr,"Invalid PPM Image!\n");
		return (-1);
	}

	/* Skip comments. */
	i = 3;
	while( fileBuff[i] == '#' )
		i = skipLine(i);

	/* Read width and height. */
	i = readNumber(line,i) + 1;
	width = atoii(line);

	i = readNumber(line,i) + 1;
	height = atoii(line);

	/* Maximum color val. */
	i = readNumber(line,i) + 1;
	colorMax = atoii(line);
	colorMax++;

	/* Fill the buffer. */
	fob.x = CENTER;
	fob.y = CENTER;

	fob.buff = fileBuff;
	fob.start = i;
	fob.width = width;
	fob.height = height;
	fob.size = width * height * 3;
	fob.tColor = 0xFF00FF;

	/* Draws. */
	drawBuffer(&fob);
	return (EXIT_SUCCESS);
}

/**
 * Main function.
 */
int main(int argc, char *const argv[])
{
	getargs(argc, argv);
	return drawImage(args.filename);
}
