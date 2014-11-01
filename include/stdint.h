/* 
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * stdint.h - Integer types
 */

#ifndef STDINT_H_
#define STDINT_H_

#ifndef _ASM_FILE_

	/* Signed integers. */
	typedef char int8_t;   /* 8-bit integer.  */
	typedef short int16_t; /* 16-bit integer. */
	typedef int int32_t;   /* 32-bit integer. */
	
	/* Unsigned integers. */
	typedef unsigned char uint8_t;           /* 8-bit integer.  */
	typedef unsigned short uint16_t;         /* 16-bit integer. */
	typedef unsigned int uint32_t;           /* 32-bit integer. */
	typedef unsigned long long int uint64_t; /* 64-bit integer. */

#endif /* _ASM_FILE_*/

#endif /* STDINT_H_ */
