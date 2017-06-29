#include <sys/sem.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <limits.h>
#include <nanvix/fs.h>

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
int namevalid(const char* pathname)
{
	size_t len;
	const char *p;
	char filename[50];

	len = kstrlen(pathname);
	
	if (!chkmem(pathname, len, MAY_WRITE))
		return -1;

	p = pathname;

	p = break_path(p, filename);

	if (p == NULL)
		return -1;

	while (*p != '\0')
	{
		p = break_path(p, filename);
	}

	len = kstrlen(filename);

	if (len > MAX_SEM_NAME)
		return -1;

	return 0;
}


/**
 * 	@brief Searching if a semaphore descriptor exists 
 *
 *	@parameters sempath The complet semaphore descriptor path
 */
int existence_semaphore(const char* sempath)
{
	struct inode* seminode;

	seminode = inode_name(sempath);

	if (seminode == NULL)
		return -1;

	/* Semaphore exists */
	inode_put(seminode);
	inode_unlock(seminode);

	return 0;
}

/**
 * 	@brief Checks the existece of a semaphore
 *		   in the semaphore table.
 *		  
 * 	@returns The index of the semaphore in the
 *        	 semaphore table if it exists
 *           -1 otherwise.
 */
int search_semaphore (const char* semname)
{
	for (int idx = 0; idx < SEM_OPEN_MAX; idx++)
	{
		if (!kstrcmp(semname,semtable[idx].name))
		{
			return idx;
		}
	}

	return -1;
}

/**
 *	@brief Removes a semaphore from the file system
 *
 *	@parameters pathname The semaphore entire pathname
 */
int remove_semaphore (const char *pathname)
{
	const char *filename;
	struct inode *semdirectory;

	semdirectory = inode_dname(pathname, &filename);
	dir_remove(semdirectory, filename);
	inode_unlock(semdirectory);

	return 0;
}