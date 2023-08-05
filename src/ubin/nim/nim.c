/*
 * Copyright(C) 2015-2016 Davidson Francis <davidsondfgl@hotmail.com>
 *              2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
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

#include <dev/tty.h>
#include <stropts.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#define clear() ioctl(1, TTY_CLEAR)

#define COMPUTER 1
#define PLAYER   2

#define bool int
#define true  1
#define false 0

int comb[4] = {1,3,5,7};
int nBin[9];
int sticksCount = 16;
int turn = 0;

void title()
{
	clear();
	printf("      ____  _____  ___  _____    ____  _____    _  _  ____  __  __ \n");
	printf("     (_  _)(  _  )/ __)(  _  )  (  _ \\(  _  )  ( \\( )(_  _)(  \\/  )\n");
	printf("    .-_)(   )(_)(( (_-. )(_)(    )(_) ))(_)(    )  (  _)(_  )    ( \n");
	printf("    (____) (_____)\\___/(_____)  (____/(_____)  (_)\\_)(____)(_/\\/\\_)\n\n");
}

void draw()
{
	int tam = 4;
	int x,y;
	for(x=0; x<4; x++)
	{
		printf("%d: \n",x+1);
		switch(comb[x])
		{
			case 0:
				for(y=1; y<tam; y++)
					printf("\n");
				break;
			case 1:
				for(y=1; y<tam; y++)
					printf("      **\n");
				printf("\n");
				break;
			case 2:
				for(y=1; y<tam; y++)
					printf("      **  **\n");
				printf("\n");
				break;
			case 3:
				for(y=1; y<tam; y++)
					printf("      **  **  **\n");
				printf("\n");
				break;
			case 4:
				for(y=1; y<tam; y++)
					printf("      **  **  **  **\n");
				printf("\n");
				break;
			case 5:
				for(y=1; y<tam; y++)
					printf("      **  **  **  **  **\n");
				printf("\n");
				break;
			case 6:
				for(y=1; y<tam; y++)
					printf("      **  **  **  **  **  **\n");
				printf("\n");
				break;
			case 7:
				for(y=1; y<tam; y++)
					printf("      **  **  **  **  **  **  **\n");
				printf("\n");
				break;
			case 8:
				for(y=1; y<tam; y++)
					printf("      **  **  **  **  **  **  **  **\n");
				printf("\n");
				break;
			default:
				break;
		}
	}
}

void youLose()
{
	title();
	printf("\nEh dificil dizer isso mas, vc perdeu, ;(\n\n");
}

void youWin()
{
	title();
	printf("                              BRAVO!!!\n");
	printf("                           voce me venceu\n\n\n");
	printf("     ######     #    ######     #    ######  ####### #     #  #####\n");
	printf("     #     #   # #   #     #   # #   #     # #       ##    # #     #\n");
	printf("     #     #  #   #  #     #  #   #  #     # #       # #   # #       \n");
	printf("     ######  #     # ######  #     # ######  #####   #  #  #  #####\n");
	printf("     #       ####### #   #   ####### #     # #       #   # #       #\n");
	printf("     #       #     # #    #  #     # #     # #       #    ## #     #\n");
	printf("     #       #     # #     # #     # ######  ####### #     #  #####\n\n\n");
}

void reverse(char* s)
{
    int i, j;
    char c;

    for (i = 0, j = strlen(s)-1; i<j; i++, j--)
    {
        c = s[i];
        s[i] = s[j];
        s[j] = c;
    }
}

void itoa(int n, char* s)
{
    int i, sign;

    if ((sign = n) < 0)
        n = -n;

    i = 0;
    do
    {
        s[i++] = n % 10 + '0';
    }while ((n /= 10) > 0);

    if (sign < 0)
        s[i++] = '-';

    s[i] = '\0';
    reverse(s);
}

int get()
{
	char ret;
	ret = getchar();
	getchar();
	return ret;
}

int newSum(int column)
{
	int sum = 0;

	for(int x=0; x<4; x++)
	{
		if(x == column)
			continue;

		sum = sum + nBin[ comb[x] ];
	}
	return sum;
}

bool isSecure(int sum)
{
	bool answer = true;
	char number[4];

	itoa(sum, number);

	if(sum == 1 || sum == 3)
		answer = true;
	else if (sum == 2)
		answer = false;
	else
	{
		for (int x=0; number[x] != '\0'; x++)
		{
			if( (number[x]-48) % 2 != 0 )
			{
				answer = false;
				break;
			}
		}
	}

	return answer;
}

void think()
{
	title();
	draw();

	if (turn == COMPUTER)
	{
		printf("\nBom, agora e minha vez, posso jogar? (1:sim)  ");
		get();

		int  sum;
		bool isSec = false;

		for (int x=1; x<7 && !isSec; x++)
		{
			for (int y=0; y<4 && !isSec; y++)
			{
            	if ( (comb[y] == 0) || (comb[y] - x < 0) )
            		continue;

            	sum = newSum(y) + nBin[ comb[y]-x ];

            	if (isSecure(sum) == true)
            	{
            		isSec = true;
            		comb[y] -= x;
            		sticksCount -= x;
            	}
            }
    	}

    	if(isSec == false)
    	{
    		for(int i=0; i<4; i++)
    		{
    			if(comb[i] != 0)
    			{
    				comb[i] -= 1;
    				sticksCount--;
    				break;
    			}
    		}
    	}

    	turn = PLAYER;
	}
	else if (turn == PLAYER)
	{
		int row, count;

		printf("Ok, entao digite a fileira que deseja operar (1-4): ");
		row = get() - 48;

		printf("Ok, agora me informe quantas pecas deseja remover: ");
		count = get() - 48;

		comb[row-1] -= count;
		sticksCount -= count;

		turn = COMPUTER;
	}
}

int main()
{
	setvbuf(stdout, NULL, _IONBF, BUFSIZ);
	int option;

	nBin[0] = 0;
	nBin[1] = 1;
	nBin[2] = 10;
	nBin[3] = 11;
	nBin[4] = 100;
	nBin[5] = 101;
	nBin[6] = 110;
	nBin[7] = 111;
	nBin[8] = 1000;

	title();

	printf("\n\nEste jogo consiste em retirar os \"palitos\", sendo que voce"
		" pode remover quantos palitos quiser de uma unica fileira, e, quem"
		" remover o ultimo palito, perde:\n\n");

	draw();

	printf("Quem comeca, computador:1, voce:2 ? ");
	option = get() - 48;
	turn = option;

	while(sticksCount > 0)
		think();

	if(turn == COMPUTER)
		youLose();
	else if(turn == PLAYER)
		youWin();

	return 0;
}
