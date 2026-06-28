/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_NET_IF_H
#define _NANVIX_NET_IF_H

/**
 * @file net/if.h
 * @brief Network interface name limits.
 *
 * Defines `IF_NAMESIZE`/`IFNAMSIZ`, the maximum length of a network interface
 * name including the terminating null byte, and `struct if_nameindex`, which
 * pairs an interface index with its name.
 */

#define IF_NAMESIZE 16
#define IFNAMSIZ IF_NAMESIZE

struct if_nameindex {
    unsigned int if_index;
    char *if_name;
};

#endif /* _NANVIX_NET_IF_H */
