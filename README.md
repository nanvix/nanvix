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
  
  - If using i386 architecture:
    ```bash
    $ sudo bash tools/dev/arch/setup-toolchain-i386.sh
    ```

  - If using or1k architecture:
	```bash
	$ sudo bash tools/dev/arch/setup-toolchain-or1k.sh
	```

- To build Nanvix:
  ```bash
  $ cd path/folder-name
  ```

  - If using i386 architecture:
	```bash
	$ make nanvix TARGET=i386
	$ sudo make image TARGET=i386
	```
	
  - If using or1k architecture:
	```bash
	$ make nanvix TARGET=or1k
	$ sudo make image TARGET=or1k
	```

- To run Nanvix on a virtual machine:
  ```bash
  $ cd path/folder-name
  ```

  - If using i386 architecture:
	```bash
	$ export TARGET=i386
	```

  - If using or1k architecture:
	```bash
	$ export TARGET=or1k
	```

  ```bash
  $ bash tools/run/run-qemu.sh
  ```
  
## TIPS
Some people may feel that those commands to build and run nanvix are a litle tricky or that is annoying to remember every time if `TARGET` was already exported or not. If that is your case, you can always create an function (assigning an alias to it) and put it in your `<.bashrc>`file. In case you don't know how to do that, follow the steps below:

- Open your `<.bashrc>` file with your preference text editor:
  ```bash
  $ vim ~/.bashrc
  ```
  
- Add the following lines at the end of `<.bashrc>`file:
  ```bash
  # Feel free to modify this function as you wish
  function __nanvix() {
      NANVIX_FOLDER="/home/luigidcsoares/dev/nanvix"
      CURDIR=$(pwd)

      if [ $CURDIR != $NANVIX_FOLDER ]; then
	      cd $NANVIX_FOLDER
      fi

      target_defined=false
      build=false
      run=false

      while [ $# -gt 0 ]; do
          case "$1" in
              -h|--help)
                  echo '************** Nanvix script **************'
                  echo '-h, --help        show an brief help'
                  echo '--i386, --or1k    set target architecture'
                  echo '-b, --build       build nanvix and generate its image'
                  echo '-r, --run         run nanvix'
                  return 0
                  ;;
              --i386)
                  export TARGET=i386
                  target_defined=true
                  shift
                  ;;
              --or1k)
                  export TARGET=or1k
                  target_defined=true
                  shift
                  ;;
              -b|--build)
                  build=true
                  shift
                  ;;
              -r|--run)
                  run=true
                  shift
                  ;;
              *)
                  echo "ERROR: undefined option!"
                  return 1
          esac
      done

      # Raises error if target wasn't passed by params
      if [ $target_defined = false ]; then
          echo "ERROR: target undefined!"
          return 1
      fi
    
      if [ $build = true ]; then
          make nanvix TARGET=$TARGET && sudo make image TARGET=$TARGET
      fi
    
      if [ $run = true ]; then
          bash tools/run/run-qemu.sh
      fi
  }
  alias nanvix=__nanvix
  ```
  
- Save the changes and run the following command on terminal to for the changes take effect:
  ```bash
  $ source ~/.bashrc
  ```
  
Now you are able to build and run nanvix with a single command. Following are the options that the above script takes:

- `-h|--help`: show an brief help about all the commands.
- `--i386`: set target architecture to i386.
- `--or1k`: set target architecture to or1k.
- `-b|--build`: build nanvix and generate its image.
- `-r|--run`: run nanvix.

## LICENSE AND MAINTAINERS

Nanvix is a free software that is under the GPL V3 license and is
maintained by Pedro H. Penna. Any questions or suggestions send
him an email: <pedrohenriquepenna@gmail.com>

Join our mailing list at https://groups.google.com/d/forum/nanvix
