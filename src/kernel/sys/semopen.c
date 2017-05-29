#include <string.h>
#include <fcntl.h>
#include <stdarg.h>	/*va_start ... */
#include <sys/sem.h>
#include <nanvix/klib.h>


/* add the semaphore to the sem table if it doesn't exist
 * returns the added/existing semaphore address
 */

	sem_t* add_entry(int value, char* name, int mode){

		sem_t *s;

		for (s = &semtable[0]; s < &semtable[MAX_SEMAPHORES]; s++)
		{
			/* searching for a free slot */
			for (int i=0; i<MAX_SEMAPHORES; i++)
			{
				if(semtable[i].nbproc==0){
					s->value=value;
					kstrcpy(name,s->name);
					s->mode=mode;
					s->nbproc=0;
					return &semtable[i];
				}
			}
		}

		return NULL; /* semaphore table full */
	}

	/*
	 * returns 0 if doesn't exists
	 * returns non-zero otherwise
	 */
	sem_t* existance(char* semname){
		
		sem_t *s;

		for (s = &semtable[0]; s < &semtable[MAX_SEMAPHORES]; s++)
		{
			//if(s->name == semname){
			if(kstrcmp(s->name,semname)){
				/* add process to semaphore  */
				return s;
			}
		}

		return 0;
	}

	PUBLIC sem_t* sys_semopen(char* name, int oflag, ...){

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

 			/* correct this to have everything in the if */
 			s = existance(name);

 			if(s!=0)				/* if this name already exists */
 			{
 				if(oflag & O_EXCL)
 				{
 					return NULL; 			/* error */
 				}
 				else
 				{

 				}
 			}
 			else
 			{
	 			/* Semaphore creation if doesn't 'exist */
	 			va_start(arg, oflag);
	 			mode = va_arg(arg, mode_t);
	 			value = va_arg(arg, int);
	 			va_end(arg);
 				s=add_entry (value,name,mode);
 			}
 		}

 		s->nbproc++;

		return s;
	}