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

/**
 * @brief Creates a command history.
 */
struct history *history_init(int size)
{
	struct history *hist;

	hist = malloc(sizeof(struct history));
	if (hist == NULL)
	{
		fprintf(stderr, "%s: failed to allocate history buffer\n", TSH_NAME);
		exit(EXIT_FAILURE);
	}

	hist->top = 0;
	hist->size = size;
	hist->p = -1;
	for (int i = 0; i < hist->size; i++)
	{
		hist->log[i] = calloc(LINELEN, sizeof(char));
		if (hist->log[i] == NULL)
		{
			fprintf(stderr, "%s: failed to allocate history buffer\n", TSH_NAME);
			exit(EXIT_FAILURE);
		}
	}

	return (hist);
}

/**
 * @brief Destroys a command history.
 *
 * @brief hist Target command history.
 */
void history_destroy(struct history *hist)
{
	for (int i = 0; i < hist->size; i++)
		free(hist->log[i]);

	free(hist);
}

/**
 * @brief Asserts if a command history is full.
 *
 * @param hist Target command history.
 */
static inline int history_full(struct history *hist)
{
	return (hist->top == hist->size);
}

/**
 * @brief Returns the next command on a command history.
 */
char *history_next(struct history *hist)
{
	/* History is empty. */
	if (hist->top == 0)
		return ("");

	/*
	 * We've printed command at
	 * index 0 before.
	 */
	if (hist->p < 0)
		hist->p = 0;

	/* End of the history. */
	if (hist->p == (hist->top - 1))
		return ("");

	return (hist->log[++hist->p]);
}

/**
 * @brief Returns the previous command on a command history.
 */
char *history_previous(struct history *hist)
{
	/* History is empty. */
	if (hist->top == 0)
		return ("");

	/* Start of the history. */
	if (hist->p < 0)
		return (hist->log[0]);

	return (hist->log[hist->p--]);
}

/**
 * @brief Pushes a new element on the history.
 */
void history_push(struct history *hist, char *s)
{
	/* Dont do that. */
	if (*s == '\0')
		return;

	/* Shift elements. */
	if (history_full(hist))
	{
		for (int i = 0; i < hist->top; i++)
			strncpy(hist->log[i], hist->log[i + 1], LINELEN);
		hist->top--;
	}

	strncpy(hist->log[hist->p = hist->top++], s, LINELEN);
}
