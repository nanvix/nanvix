FROM ubuntu:latest

# Clone nanvix repository and setup environment

WORKDIR /nanvix

RUN export WORKSPACE=$PWD
RUN export INSTALL_DIR=$WORKSPACE
RUN export TOOLCHAIN_DIR=/nanvix/toolchain

# Copy all repository from local host to docker container
COPY . .

# Install requirements
RUN apt update -y
RUN apt install sgabios sudo wget python3 python3-venv qemu-system -y
RUN sudo -E ./tools/dev/setup/ubuntu.sh

# Build Toolchain
RUN ./tools/dev/setup/binutils.sh $INSTALL_DIR
RUN ./tools/dev/setup/gcc.sh      $INSTALL_DIR
RUN ./tools/dev/setup/gdb.sh      $INSTALL_DIR

#Build Nanvix binaries
RUN make nanvix

# Build System Image
RUN make image

# Startup Nanvix
RUN echo "qemu-system-x86_64 -hda /nanvix/hdd.img -initrd /nanvix/initrd.img -kernel /nanvix/bin/kernel -m 200M -machine pc -serial mon:stdio -append 'console=ttyS0' -nographic -device sga -display curses" >> ~/.bashrc
