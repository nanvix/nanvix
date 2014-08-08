/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdlib/malloc.c - malloc() and free() implementations.
 */

#include <sys/types.h>
#include <errno.h>
#include <stdlib.h>
#include <unistd.h>

/* expand() in at least NALLOC blocks. */
#define NALLOC 511

/* Size of block structure. */
#define SIZEOF_BLOCK (sizeof(struct block))

/*
 * Memory block.
 */
struct block
{
	struct block *nextp; /* Next free block.  */
	unsigned nblocks;    /* Size (in blocks). */
};

/* Free list of blocks. */
static struct block head;
static struct block *freep = NULL;

/*
 * Frees memory.
 */
void free(void *ptr)
{
	struct block *p;  /* Working block.     */
	struct block *bp; /* Block being freed. */
	
	bp = (struct block *)ptr - 1;
	
	/* Look for insertion point. */
	for (p = freep; p > bp || p->nextp < bp; p = p->nextp) {
		/* Freed block at start or end. */
		if (p >= p->nextp && (bp > p || bp < p->nextp))
			break;
	}
	
	/* Merge with upper block. */
	if (bp + bp->nblocks == p->nextp)
	{
		bp->nblocks += p->nextp->nblocks + 1;
		bp->nextp = p->nextp->nextp;
	}  else
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

/*
 * Expands the heap.
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

/*
 * Allocates memory.
 */
void *malloc(size_t size)
{
	struct block *p;     /* Working block.            */
	struct block *prevp; /* Previous working block.   */
	unsigned nblocks;    /* Request size (in blocks). */
	
	/* Too big request. */
	if (size > size + (SIZEOF_BLOCK - 1)) {
		errno = -ENOMEM;
		return (NULL);
	}
	
	nblocks = (size + (SIZEOF_BLOCK - 1))/SIZEOF_BLOCK + 1;
	
	/* Create free list. */
	if ((prevp = freep) == NULL) {
		head.nextp = freep = prevp = &head;
		head.nblocks = 0;
	}
	
	/* Look for a free block that is big enough. */
	for (p = prevp->nextp; /* void */ ; prevp = p, p = p->nextp) {
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
