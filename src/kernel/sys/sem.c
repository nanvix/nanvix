#include <sys/sem.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>

/**
 *	@brief make a semaphore slot available for
 *		   a new semaphore creation
 *
 *	@returns 0 upon successful completion,
 *			 -1 if the semaphore is not valid
 */
int freesem(int idx)
{
	if(!SEM_IS_VALID(idx))
	{
		return -1;
	}

	semtable[idx].value=0;
	(semtable[idx].name)[0]='\0';
	semtable[idx].mode=0;
	semtable[idx].nbproc=0;
	semtable[idx].unlinked=0;

	return 0;
}

/**
 *	@brief Check if a semaphore name is valid
 *
 *	@returns 0 if valid, -1 otherwise
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
 * @brief checks the existance of a semaphore
 *		  in the semaphore table
 *		  
 * @returns the index of the semaphore in the
 *          semaphore table if it exists
 *          SEM_FAILED otherwise
 */
int existance(const char* semname)
{
	int idx;

	for (idx=0; idx<SEM_OPEN_MAX; idx++)
	{			
		if(!(kstrcmp(semtable[idx].name,semname))){
			return idx;
		}
	}

	return -1;
}