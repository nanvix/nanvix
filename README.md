# Nanvix [![Build Status](https://travis-ci.org/nanvix/nanvix.svg?branch=master)](https://travis-ci.org/nanvix/nanvix) [![Join us on Slack!](https://img.shields.io/badge/chat-on%20Slack-e01563.svg)](https://join.slack.com/t/nanvix/shared_invite/enQtMzY2Nzg5OTQ4NTAyLWY1NWNkNjlmMDU1NjgwZmU2Njk5ODQxOTFmMGY0YjNlZWU0NWUxNThiNzgwNTAyMWE5YTcyOTc4Njk2NjBiZjk)

## What Is Nanvix

Nanvix is a Unix-like operating system written by [Pedro H. Penna](https://github.com/ppenna) for
educational purposes. It is designed to be simple and small, but also
modern and fully featured.

## What Hardware Is Required?

Nanvix targets 32-bit x86-based PCs and only requires a floppy or
CD-ROM drive and 16 MB of RAM. You can run it either in a real PC
or in a virtual machine, using a Live System's Image.

## Building

### Linux-like systems
In order to build Nanvix, you will need a Linux like programming
environment, the x86 GCC compiler and the x86 GNU binutils.

If you are running a Debian-based Linux distribution, like Ubuntu,
you can simply run the following commands at the root directory:

```sh
sudo apt-get install make
sudo bash tools/dev/setup-toolchain.sh
sudo bash tools/dev/setup-bochs.sh
sudo reboot now
```

When done, you can build Nanvix by typing, at the root directory:

```sh
make nanvix
```

Or you can build a Live System's Image by typing, at the same directory:

```sh
make image
```

### macOS

The support of Nanvix on macOS is still experimental.
It relies on the Homebrew package manager (https://brew.sh), to get
the necessary UNIX-like packages to build Nanvix.

The general flow for building Nanvix on macOS from the root directory
is as follows:

```sh
bash tools/dev/setup-toolchain.sh
make TARGET=i386-elf nanvix
make image
```

Unlike the Linux-like systems procedure, this flow does not build
bochs from source. Instead, it installs the bochs package using brew.
As this package does not come with gdb-stubs support, you cannot run
Nanvix with `--debug` option (see details below).

## Running

To run Nanvix, type the following command at the root directory:

```sh
bash tools/run/run.sh
```

The script above accepts some optional parameters to configure bochs.
Please, run `bash tools/run/run.sh --help` for more details.

## License and Maintainers

Nanvix is a free software that is under the GPL V3 license and is
maintained by Pedro H. Penna. Any questions or suggestions send him an
email: <pedrohenriquepenna@gmail.com>
