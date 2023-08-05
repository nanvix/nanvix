/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/*
 * Copyright (C) 1991-1996 Free Software Foundation, Inc.
 *
 * This file is part of the GNU C Library.
 *
 * The GNU C Library free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * The GNU C Library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston,
 * MA 02110-1301, USA.
 */

/**
 * @file
 *
 * @brief getsubopt() implementation.
 */

#include <string.h>

/**
 * @brief Parses suboption arguments from a string.
 *
 * @param keylistp List of strings to parse.
 * @param valuep   Address of a value string.
 *
 * @returns The index of the matched token string, or -1 if no token strings
 *          were matched.
 */
int getsubopt(char **optionp, char *const *keylistp, char **valuep)
{
	char *endp, *vstart;
	int cnt;

	if (**optionp == '\0')
		return (-1);

	/* Find end of next token.  */
	endp = strchr(*optionp, ',');
	if (endp == NULL)
		endp = *optionp + strlen(*optionp);

	/* Find start of value.  */
	vstart = memchr(*optionp, '=', endp - *optionp);
	if (vstart == NULL)
		vstart = endp;

	/*
	 * Try to match the characters between *OPTIONP
	 * and VSTART against one of the keylistp.
	 */
	for (cnt = 0; keylistp[cnt] != NULL; cnt++)
	{
		/* We found the current option in keylistp.  */
		if ((strncmp(*optionp, keylistp[cnt], vstart - *optionp) == 0)
			&& (keylistp[cnt][vstart - *optionp] == '\0'))
		{
			*valuep = vstart != endp ? vstart + 1 : NULL;

			if (*endp != '\0')
				*endp++ = '\0';
			*optionp = endp;

			return (cnt);
		}
	}

	/*
	 * The current suboption does not
	 * match any option.
	 */
	*valuep = *optionp;
	if (*endp != '\0')
		*endp++ = '\0';
	*optionp = endp;

	return (-1);
}
