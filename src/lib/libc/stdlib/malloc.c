/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/**
 * @file
 *
 * @brief malloc() and free() implementation.
 */

#include <sys/types.h>
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/**
 * @brief expand() in at least NALLOC blocks.
 */
#define NALLOC 511

/**
 * @brief Size of block structure.
 */
#define SIZEOF_BLOCK (sizeof(struct block))

/**
 * @brief Memory block.
 */
struct block
{
	struct block *nextp; /* Next free block.  */
	unsigned nblocks;    /* Size (in blocks). */
};

/**
 * @brief Free list of blocks.
 */
static struct block head;
static struct block *freep = NULL;

/**
 * @brief Frees allocated memory.
 *
 * @param ptr Memory area to free.
 */
void free(void *ptr)
{
	struct block *p;  /* Working block.     */
	struct block *bp; /* Block being freed. */

	/* Nothing to be done. */
	if (ptr == NULL)
		return;

	bp = (struct block *)ptr - 1;

	/* Look for insertion point. */
	for (p = freep; p > bp || p->nextp < bp; p = p->nextp)
	{
		/* Freed block at start or end. */
		if (p >= p->nextp && (bp > p || bp < p->nextp))
			break;
	}

	/* Merge with upper block. */
	if (bp + bp->nblocks == p->nextp)
	{
		bp->nblocks += p->nextp->nblocks + 1;
		bp->nextp = p->nextp->nextp;
	}
	else
		bp->nextp = p->nextp;

	/* Merge with lower block. */
	if (p + p->nblocks == bp)
	{
		p->nblocks += bp->nblocks + 1;
		p->nextp = bp->nextp;
	}
	else
		p->nextp = bp;

	freep = p;
}

/**
 * @brief Expands the heap.
 *
 * @details Expands the heap by @p nblocks.
 *
 * @param nblocks Number of blocks to expand.
 *
 * @returns Upon successful completion a pointed to the expansion is returned.
 *          Upon failure, a null pointed is returned instead and errno is set
 *          to indicate the error.
 */
static void *expand(unsigned nblocks)
{
	struct block *p;

	/* Expand in at least NALLOC blocks. */
	if (nblocks < NALLOC)
		nblocks = NALLOC;

	/* Request more memory to the kernel. */
	if ((p = sbrk((nblocks + 1)*SIZEOF_BLOCK)) == (void *)-1)
		return (NULL);

	p->nblocks = nblocks;
	free(p + 1);

	return (freep);
}

/**
 * @brief Allocates memory.
 *
 * @param size Number of bytes to allocate.
 *
 * @returns Upon successful completion with size not equal to 0, malloc()
 *          returns a pointer to the allocated space. If size is 0, either a
 *          null pointer or a unique pointer that can be successfully passed to
 *          free() is returned. Otherwise, it returns a null pointer and set
 *          errno to indicate the error.
 */
void *malloc(size_t size)
{
	struct block *p;     /* Working block.            */
	struct block *prevp; /* Previous working block.   */
	unsigned nblocks;    /* Request size (in blocks). */

	/* Nothing to be done. */
	if (size == 0)
		return (NULL);

	nblocks = (size + (SIZEOF_BLOCK - 1))/SIZEOF_BLOCK + 1;

	/* Create free list. */
	if ((prevp = freep) == NULL)
	{
		head.nextp = freep = prevp = &head;
		head.nblocks = 0;
	}

	/* Look for a free block that is big enough. */
	for (p = prevp->nextp; /* void */ ; prevp = p, p = p->nextp)
	{
		/* Found. */
		if (p->nblocks >= nblocks)
		{
			/* Exact. */
			if (p->nblocks == nblocks)
				prevp->nextp = p->nextp;

			/* Split block. */
			else
			{
				p->nblocks -= nblocks;
				p += p->nblocks;
				p->nblocks = nblocks;
			}

			freep = prevp;

			return (p + 1);
		}

		/* Wrapped around free list. */
		if (p == freep)
		{
			/* Expand heap. */
			if ((p = expand(nblocks)) == NULL)
				break;
		}
	}

	return (NULL);
}

/**
 * @brief Reallocates a memory chunk.
 *
 * @param ptr  Pointer to old object.
 * @param size Size of new object.
 *
 * @returns Upon successful completion, realloc() returns a pointer to the
 *           allocated space. Upon failure, a null pointer is returned instead.
 *
 * @todo Check if we can simply expand.
 */
void *realloc(void *ptr, size_t size)
{
	void *newptr;

	/* Nothing to be done. */
	if (size == 0)
	{
		errno = EINVAL;
		return (NULL);
	}

	newptr = malloc(size);
	if (ptr != NULL)
		memcpy(newptr, ptr, size);

	free(ptr);

	return (newptr);
}
