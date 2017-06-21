#include <sys/sem.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>


#include <nanvix/const.h>
#include <sys/types.h>

/**
 * @brief Copies part of a string.
 * 
 * @param str1 Target string.
 * @param str2 Source string.
 * @param n    Number of characters to be copied.
 * 
 * @returns A pointer to the target string.
 */
PUBLIC char *kstrcncat(const char *str1, const char *str2, char *str3)
{
	const char *p1; /* Indexes str1. */
	const char *p2; /* Indexes str2. */
	char *p3; /* Indexes str3. */
	
	p1 = str1;
	p2 = str2;
	p3 = str3;

	while (*p1 != '\0')
	{
		*p3 = *p1;
		p1++;
		p3++;
	}

	while (*p2 != '\0')
	{
		*p3 = *p2;
		p2++;
		p3++;
	}

	*p3 = '\0';

	return (str3);
}

/**
 *	@brief make a semaphore slot available for
 *		   a new semaphore creation
 *
 *	@returns 0 upon successful completion,
 *			 -1 if the semaphore is not valid
 */
void freesem(struct ksem *sem)
{
	sem->value=0;
	(sem->name)[0]='\0';
	sem->state=0;
	sem->nbproc=0;

	for (int i = 0; i<PROC_MAX; i++)
		sem->currprocs[i] = -1;
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

struct inode *existence_inode(const char* semname)
{
	struct inode *seminode = NULL;
	seminode = get_sem(semname);

	if (seminode != NULL)
	{
		file_read(seminode, &sembuf, sizeof(struct ksem),0);
		return seminode;
	}
	else
	{
		return NULL;
	}
}

