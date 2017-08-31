#include <sys/sem.h>
#include <errno.h>
#include <nanvix/syscall.h>

/**
 * @brief	Unlinks a semaphore.
 *		 
 * @param	name Semaphore's absolute path.
 *
 * @returns 0 in case of successful completion
 *			Corresponding error code otherwise.
 */
PUBLIC int sys_semunlink(const char *name)
{
	int idx;
	struct inode* seminode;
	char semname[MAX_SEM_NAME-4];

	/* Name invalid. */
	if (namevalid(name) == (-1))
		return (ENAMETOOLONG);

	sem_path(name, semname);
	seminode = inode_name(semname);

	if (seminode == NULL)
		return (-ENOENT);

	if (seminode->nlinks == 0)
	{
		inode_put(seminode);
		inode_unlock(seminode);
		return (-ENOENT);
	}

	if (!permission(seminode->mode, seminode->uid, seminode->gid, curr_proc, MAY_WRITE, 0))
	{
		inode_put(seminode);
		inode_unlock(seminode);
		return -(EACCES);
	}

	idx = search_semaphore(semname);

	inode_unlock(seminode);
	remove_semaphore(semname);

	/* If no process uses the semaphore : delete the semaphore descriptor and the table entry. */
	if (seminode->count == 1)
	{
		/* Freeing from kernel semaphore table. */
		freesem(&semtable[idx]);
		inode_put(seminode);
		return 0;
	}

	/* Unlinking the semaphore */
	semtable[idx].name[0] = '\0';
	inode_put(seminode);

	return 0;	/* Successful completion */
}
