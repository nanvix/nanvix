#include <limits.h>
#include <sys/types.h>
#include <nanvix/config.h>
#include <nanvix/pm.h>



#ifndef SEM_H_
#define SEM_H_


#define UNLINKED 	0001000
#define PERMISSIONS 0000777

#define SEM_IS_VALID(idx) \
		( (idx>=0 && idx<SEM_OPEN_MAX) && semtable[idx].name[0]!='\0')

#define SEM_IS_FREE(idx) \
		(semtable[idx].nbproc=-1)

#define SEM_VALID_VALUE(val) \
		(val<=SEM_VALUE_MAX)

#ifndef _ASM_FILE_

	/* Kernel semaphores */
	struct ksem {
		short value;              			/* Value of the semaphore                    	*/
		char name[MAX_SEM_NAME];    		/* Name of the named semaphore               	*/
		unsigned short state;           	/* 0-8 : mode, 9 : unlinked bit					*/
		char nbproc;                 		/* Number of processes sharing the semaphore 	*/
		pid_t currprocs[PROC_MAX];			/* Processes using the semaphores				*/
	};

	/* Semaphores table */
	extern struct ksem semtable[SEM_OPEN_MAX];

	extern struct process* semwaiters[PROC_MAX];

	/* 
	 *  Semaphore buffer used to avoid
	 *  multiple reading
	 */
	extern struct ksem sembuf;

	extern struct inode *semdirectory;

	/* Frees a semaphore */
	void freesem(struct ksem *sem);

	/* Verify if a semaphore name is valid */
	int namevalid(const char* name);

	/* Give the index of the semaphore if it exists */
	int existance(const char* semname);

	/* Returns the inode for a specific semaphore name */
	struct inode *get_sem(const char* semname);

	/**
	 * @brief Returns a semaphore and set the sembuf buffer to the semaphore contained
	 * 		  in the semaphore inode
	 * 
	 * @param semname Semaphore name
	 */
	struct inode *existence_inode(const char* semname);

#endif

#endif