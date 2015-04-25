/*
 * Copyright(C) 2011-2015 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * @details Causes the space pointed to by @p ptr to be deallocated; that is,
 *          made available for further allocation. If @p ptr is a null pointer,
 *          no action occurs. Otherwise, if the argument does not match a
 *          pointer earlier returned by a function that allocates memory as if
 *          by malloc(), or if the space has been deallocated by a call to 
 *          ree() or realloc(), the behavior is undefined.
 * 
 *          Any use of a pointer that refers to freed space results in
 *          undefined behavior.
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
 * @details Allocates unused space for an object whose size in bytes is
 *          specified by @p size and whose value is unspecified.
 * 
 *          The order and contiguity of storage allocated by successive calls to
 *          malloc() is unspecified. The pointer returned if the allocation
 *          succeeds is suitably aligned so that it may be assigned to a pointer
 *          to any type of object and then used to access such an object in the
 *          space allocated - until the space is explicitly freed or
 *          reallocated. Each such allocation yields a pointer to an object
 *          disjoint from any other object. The pointer returned points to the
 *          start (lowest byte address) of the allocated space. If the space
 *          cannot be allocated, a null pointer is returned. If the size of the
 *          space requested is 0, the behavior is implementation-defined: the
 *          value returned may be either a null pointer or a unique pointer.
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
	
	/* Too big request. */
	if (size > size + (SIZEOF_BLOCK - 1))
	{
		errno = -ENOMEM;
		return (NULL);
	}
	
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
		if (p == freep) {
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
 * @details Deallocates the old object pointed to by @p ptr and return a pointer
 *          to a new object that has the size specified by @p size. The contents
 *          of the new object are the same as that of the old object prior to
 *          deallocation, up to the lesser of the new and old sizes. Any bytes
 *          in the new object beyond the size of the old object have
 *          indeterminate values. If the size of the space requested is zero,
 *          the behavior is implementation-defined: either a null pointer is
 *          returned, or the behavior is as if the size were some non-zero
 *          value, except that the returned pointer is not used to access an
 *          object. If the space cannot be allocated, the object remains
 *          unchanged.
 * 
 *          If ptr is a null pointer, realloc() shall be equivalent to malloc()
 *          for the specified size.
 * 
 *          If ptr does not match a pointer returned earlier by calloc(),
 *          malloc(), or realloc() or if the space has previously been
 *          deallocated by a call to free() or realloc(), the behavior is
 *          undefined.
 * 
 *          The order and contiguity of storage allocated by successive calls
 *          to realloc() is unspecified. The pointer returned if the allocation 
 *          succeeds is suitably aligned so that it may be assigned to a pointer
 *          to any type of object and then used to access such an object in the
 *          space allocated - until the space is explicitly freed or
 *          reallocated. Each such allocation yields a pointer to an object
 *          disjoint from any other object. The pointer returned points to the
 *          start (lowest byte address) of the allocated space. If the space
 *          cannot be allocated, a null pointer is returned.
 * 
 * @returns Upon successful completion, realloc() returns a pointer to the
 *          (possibly moved) allocated space.  If size is 0, either:
 *          	- A null pointer is returned and errno set to an implementation-
 *                defined value.
 *              - A unique pointer that can be successfully passed to free() is
 *                returned, and the memory object pointed to by ptr is freed.
 *                The application shall ensure that the pointer is not used to
 *                access an object.
 * 
 *          If there is not enough available memory, realloc() returns  a null
 *          pointer and set errno to ENOMEM. If realloc() returns a null pointer
 *          and errno has been set to ENOMEM, the memory referenced by @p ptr
 *          is not changed.
 * 
 * @todo Check if we can simply expand.
 */
void *realloc(void *ptr, size_t size)
{
	void *newptr;
	
	newptr = malloc(size);
	if (ptr != NULL)
		memcpy(newptr, ptr, size);
		
	free(ptr);
	
	return (newptr);
}
