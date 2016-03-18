/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
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

#include <stddef.h>
#include <stdio.h>
#include <string.h>

#include "tsh.h"
#include "history.h"

/*
 * Initializes the command stack
 */
struct STACK *initialize_stack(int capacity)
{
	struct STACK *hist;
	
	hist = malloc(sizeof(struct STACK));
	if (hist == NULL)
	{
		fprintf(stderr, "%s: failed to allocate history buffer\n", TSH_NAME);
		exit(EXIT_FAILURE);
	}

	hist->top = -1;
	hist->capacity = capacity;
	for (int i = 0; i < hist->capacity; i++)
	{
		hist->hist[i] = calloc(LINELEN, sizeof(char));
		if (hist->hist[i] == NULL)
		{
			fprintf(stderr, "%s: failed to allocate history buffer\n", TSH_NAME);
			exit(EXIT_FAILURE);
		}
	}

	return (hist);
}

/*
 * Utility to determine if the stack is full
 */
static int is_full(struct STACK *stack)
{
	return (stack->top == (stack->capacity - 1));
}

/**
 * @brief Pushes a new element on the history.
 */
void push_hist(struct STACK *stack, char *s)
{
	if (is_full(stack))
	{
		/* Shift elements. */
		for (int i = 0; i < (STACK_SIZE - 1); i++)
			strncpy(stack->hist[i], stack->hist [i + 1], LINELEN);
	}
	
	strncpy(stack->hist[++stack->top], s, LINELEN);
}
