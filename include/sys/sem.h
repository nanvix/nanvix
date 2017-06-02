#include <limits.h>
#include <sys/types.h>

#define SEM_IS_VALID(idx) \
		( (idx>=0 && idx<SEM_OPEN_MAX) && semtable[idx].name[0]!='\0')

#define SEM_IS_FREE(idx) \
		(semtable[idx].nbproc=-1)

#define SEM_VALID_VALUE(val) \
		(val<=SEM_VALUE_MAX)

#ifndef _ASM_FILE_

	/* Kernel semaphores */
	struct ksem {	
		int value;                   	/* value of the semaphore                    */
		char name[MAX_SEM_NAME];    	/* name of the named semaphore               */
		int mode;                   	/* permissions                               */
		int nbproc;                 	/* number of processes sharing the semaphore */
		int unlinked;
		uid_t uid;              		/* Semaphore User ID                 */
		gid_t gid;              		/* Semaphore Group idx               */
	};

	/* Semaphores table */
	extern struct ksem semtable[SEM_OPEN_MAX];

	/* Frees a semaphore */
	int freesem(int idx);

	/* Verify if a semaphore name is valid */
	int namevalid(const char* name);

#endif