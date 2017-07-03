#include <sys/sem.h>
#include <errno.h>
#include <nanvix/syscall.h>

#include <nanvix/klib.h>


/**
 * @brief Unlinks a semaphore for future deletion
 *		 
 * @param name Semaphore name
 *
 * @returns Returns 0 in case of successful completion
 *			returns error code otherwise
 */
PUBLIC int sys_semunlink(const char *name)
{
	int idx;
	struct inode* seminode;
	char semname[MAX_SEM_NAME-4];

	/* Name invalid */
	if (namevalid(name) == (-1))
		return (ENAMETOOLONG);

	sem_path(name, semname);
	seminode = inode_name(semname);

	if (seminode == NULL)
	{
		kprintf("invalid semaphore");
		return (-ENOENT);
	}

	if (!permission(seminode->mode, seminode->uid, seminode->gid, curr_proc, MAY_WRITE, 0))
	{
		inode_put(seminode);
		inode_unlock(seminode);
		return -(EACCES);
	}

	idx = search_semaphore(semname);

	if (seminode->count == 1)
	{
		inode_unlock(seminode);
		remove_semaphore(semname);
		freesem(&semtable[idx]);
		inode_put(seminode);
		return 0;
	}

	int len = kstrlen(semname);
	kstrcpy(&semname[len],".ul");

	if (search_semaphore (semname) != (-1))
	{
		inode_unlock(seminode);
		inode_put(seminode);
		return -1;
	}

	char filename[MAX_SEM_NAME-4];
	sem_name(semname, filename);
	/* Removing suffixe */
	semname[len] = '\0';
	/* Unlinking the semaphore */
	seminode->nlinks = 0;
	inode_rename(semname, filename);
	kstrcpy(&semname[len],".ul");
	kstrcpy(semtable[idx].name, semname);

	inode_unlock(seminode);
	inode_put(seminode);

	return 0;	/* Successful completion */
}
