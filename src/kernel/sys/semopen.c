#include <string.h>
#include <fcntl.h>
#include <stdarg.h>	/*va_start ... */
#include <sys/sem.h>


/* add the semaphore to the sem table if it doesn't exist
 * returns the added/existing semaphore address
 */

	sem_t* add_entry(sem_t sem){

		sem_t *s;

		for (s = &semtable[0]; s < &semtable[MAX_SEMAPHORES]; s++)
		{
			if(s->name == sem.name){
				/* add process to semaphore  */
				return s;
			}
		}

		for (int i=0; i<MAX_SEMAPHORES; i++)
		{
			if(semtable[i].nbproc==0){
				semtable[i]=sem;
				return &semtable[i];
			}
		}

		return NULL; /* semaphore table full */
	}

	int existance(char* semname){
		sem_t *s;

		for (s = &semtable[0]; s < &semtable[MAX_SEMAPHORES]; s++)
		{
			if(s->name == semname){
				/* add process to semaphore  */
				return 1;
			}
		}

		return 0;
	}

	extern sem_t* sys_semopen(char* name, int oflag, ...){

 		mode_t mode;
 		int value;
 		va_list arg;	/* Variable argument */
 		sem_t *s;		/* Opened semaphore */

 		if(oflag & O_CREAT)
 		{
 			/* if O_CREAT and O_EXCL flags are set
 			 * and the semaphore name already exists
 			 * return an error
 			 */
 			if(existance(name))				/* if this name already exists */
 			{
 				if(oflag & O_EXCL){
 					return NULL; 			/* error */
 				}
 				else{
 					sem_t sem;
 					sem.name = name;
 					s=add_entry(sem);
 				}
 			}
 			else
 			{
	 			/* Semaphore creation if doesn't 'exist */
	 			va_start(arg, oflag);
	 			mode = va_arg(arg, mode_t);
	 			value = va_arg(arg, int);
	 			va_end(arg);
	 			sem_t sem = {value,name,mode,0};
 				s=add_entry (sem);
 			}
 		}

 		s->nbproc++;

		return s;
	}