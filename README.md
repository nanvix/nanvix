## WHAT IS NANVIX? [![Build Status](https://travis-ci.org/nanvix/nanvix.svg?branch=dev)](https://travis-ci.org/nanvix/nanvix)  [![Join us on Slack!](https://img.shields.io/badge/chat-on%20Slack-e01563.svg)](https://join.slack.com/t/nanvix/shared_invite/enQtMzY2Nzg5OTQ4NTAyLTAxMmYwOGQ0ZmU2NDg2NTJiMWU1OWVkMWJhMWY4NzMzY2E1NTIyMjNiOTVlZDFmOTcyMmM2NDljMTAzOGI1NGY)  

Nanvix is a Unix like operating system created by Pedro H. Penna to
address emerging manycore platforms. It targets cluster-based
architectures that rely on a distributed and shared memory
configuration, and it was designed from scratch to deliver cutting
edge performance, while enabling backward compatibility with
existing software. 
	
## BUILDING AND RUNNING

Nanvix currently supports x86-based PC platforms. You can run it
either in a real platform or in a real machine.

To properly build Nanvix, you need a Linux like programming
environment, the x86 GNU C Compiler and the x86 GNU Binutils.

If you are running a Debian-based Linux distribution, you can run
the following commands.

- To clone this repository (default folder name is nanvix):

```bash
$ cd path/to/clone
$ git clone --recursive https://github.com/nanvix/nanvix.git -b dev [folder-name]
```

- To get the development environment setup:
```bash
$ cd path/folder-name
$ sudo bash tools/dev/setup-toolchain.sh
$ sudo bash tools/dev/setup-qemu.sh
```

If using i386 architecture:
```bash
$ sudo bash tools/dev/arch/setup-toolchain-i386.sh
```

If using or1k architecture:
```bash
$ sudo bash tools/dev/arch/setup-toolchain-or1k.sh
```

- To build Nanvix:

```bash
$ cd path/folder-name
```

If using i386 architecture:
```bash
$ make nanvix TARGET=i386
$ sudo make image TARGET=i386
```

If using or1k architecture:
```bash
$ make nanvix TARGET=or1k
$ sudo make image TARGET=or1k```
```

- To run Nanvix on a virtual machine:

```bash
$ cd path/folder-name
```

If using i386 architecture:
```bash
$ export TARGET=i386
```

If using or1k architecture:
```bash
$ export TARGET=or1k
```

```bash
$ bash tools/run/run-qemu.sh
```

## LICENSE AND MAINTAINERS

Nanvix is a free software that is under the GPL V3 license and is
maintained by Pedro H. Penna. Any questions or suggestions send
him an email: <pedrohenriquepenna@gmail.com>

Join our mailing list at https://groups.google.com/d/forum/nanvix
