## WHAT IS NANVIX? [![Build Status](https://travis-ci.org/nanvix/nanvix.svg?branch=dev)](https://travis-ci.org/nanvix/nanvix)

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

- To clone this repository:

```bash
$ cd ~
$ git clone --recursive https://github.com/ppenna/nanvix
```

- To get the development environment setup:

```bash
$ cd ~/nanvix
$ sudo bash tools/dev/build-toolchain.sh
$ sudo bash tools/dev/setup-bochs.sh
$ sudo reboot now
```

- To build Nanvix:

```bash
$ cd ~/nanvix
$ make nanvix
$ sudo make image
```
	
- To run Nanvix on a virtual machine:

```bash
$ cd ~/nanvix
$ make nanvix
$ sudo make image
$ sudo bash tools/run/run-bochs.sh
```

## LICENSE AND MAINTAINERS

Nanvix is a free software that is under the GPL V3 license and is
maintained by Pedro H. Penna. Any questions or suggestions send
him an email: <pedrohenriquepenna@gmail.com>

Join our mailing list at https://groups.google.com/d/forum/nanvix

Join our chat room at https://gitter.im/nanvix-kernel/Lobby
