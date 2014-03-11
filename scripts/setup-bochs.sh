# 
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#
# scripts/setup-bochs.sh - Setup Bochs.
# 
# NOTES:
#   - This script should work in any Debian-based Linux distribution.
#   - You should run this script with superuser privileges.
#

# Set working directory.
mkdir ~/nanvix-toolchain
export WORKDIR=~/nanvix-toolchain
cd $WORKDIR

# Get required packages.
apt-get install libgmp3-dev libmpfr-dev libgtk2.0-dev

# Get bochs.
wget "http://sourceforge.net/projects/bochs/files/bochs/2.6.2/bochs-2.6.2.tar.gz"

# Build Bochs
tar -xvf bochs-2.6.2.tar.gz
cd bochs-2.6.2/
./configure --enable-x86-debugger --enable-debugger --enable-debugger-gui
make all
make install


