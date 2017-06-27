#include <sys/sem.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>


#include <nanvix/const.h>
#include <sys/types.h>

/**
 *	@brief Make a semaphore slot available for
 *		   a new semaphore creation
 *
 *	@returns 0 upon successful completion,
 *			 -1 if the semaphore is not valid
 */
void freesem(struct ksem *sem)
{
	(sem->name)[0]='\0';
	sem->state=0;
	sem->nbproc=0;
	sem->num = 0;

	for (int i = 0; i<PROC_MAX; i++)
		sem->currprocs[i] = -1;

	for (int i = 0; i<PROC_MAX; i++)
		sem->semwaiters[i] = NULL;
}

/**
 *	@brief Check if a semaphore name is valid
 *
 *	@returns 0 If valid, -1 otherwise
 */
int namevalid(const char* name)
{
	size_t len=kstrlen(name);

	if(len<1 || !chkmem(name, len, MAY_WRITE) )
	{
		return -1;
	}

	return 0;
}


/**
 * 	@Brief Searching if a semaphore descriptor exists from its name
 * 		   by searching in the semaphore directory
 */
struct inode *get_sem(const char* semname)
{
	ino_t num;

	if (semdirectory == NULL)
	{
		kprintf("Error with the semaphore directory");
	}

	num = dir_search(semdirectory, semname);

	if(num != INODE_NULL)
	{
		struct inode *seminode;
		seminode = inode_get(semdirectory->dev,num);
		inode_unlock(seminode);

		return seminode;
	}
	else
	{
		return NULL;
	}
}

/**
 * 	@Brief Searching if a semaphore descriptor exists from its name
 * 		   by searching in the file system
 */
int existence_semaphore(const char* semname)
{
	ino_t num;

	if (semdirectory == NULL)
	{
		kprintf("Error with the semaphore directory");
	}

	num = dir_search(semdirectory, semname);

	if(num == INODE_NULL)
	{
		/* Semaphore doesn't exist */
		return -1;
	}
	else
	{
		/* Semaphore exists */
		return 0;
	}
}

/**
 * 	@Brief checks the existece of a semaphore
 *		   in the semaphore table.
 *		  
 * 	@Returns the index of the semaphore in the
 *        	 semaphore table if it exists
 *           SEM_FAILED otherwise.
 */
int search_semaphore (const char* semname)
{
	for (int idx = 0; idx < SEM_OPEN_MAX; idx++)
	{
		if(!kstrcmp(semname,semtable[idx].name))
		{
			return idx;
		}
	}

	return -1;
}