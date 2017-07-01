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
	char filename[MAX_SEM_NAME-4];

	// char filename[50];

	/* Name invalid */
	if (namevalid(name) == (-1))
		return (ENAMETOOLONG);

	sem_path(name, semname);
	sem_name(name, filename);
	kprintf("%s %s",semname,filename);
	seminode = inode_name(semname);

	if (seminode == NULL)
	{
		/* The semaphore descriptor doesn't exist */
		return (-ENOENT);
	}

	/* Already unlinked, sem_unlink has no effect */
	if (seminode->nlinks == 0)
	{
		inode_put(seminode);
		inode_unlock(seminode);
		return -1;
	}

	if (!permission(seminode->mode, seminode->uid, seminode->gid, curr_proc, MAY_WRITE, 0))
	{
		inode_put(seminode);
		inode_unlock(seminode);
		return -(EACCES);
	}

 	idx = search_semaphore(semname);
	inode_unlock(seminode);

	if (seminode->count == 1)
	{
		kprintf("REMOVE %s",semname);
		remove_semaphore(semname);
		freesem(&semtable[idx]);
	}
	else
	{
		/* Unlinking the semaphore */
		seminode->nlinks = 0;
		int len = kstrlen(filename);
		kstrcpy(&filename[len],".ul");
		kprintf("filename %s", filename);
		inode_rename(semname, filename);
		kstrcpy(semtable[idx].name, filename);
	}
	inode_unlock(seminode);

	inode_put(seminode);

	return 0;	/* Successful completion */
}
