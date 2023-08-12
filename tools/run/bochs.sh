#
# Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#
# This file is part of Nanvix.
#
# Nanvix is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Nanvix is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with Nanvix.  If not, see <http://www.gnu.org/licenses/>.
#

# NOTES:
#   - This script should work in any Linux distribution.

VERSION_MAJOR=1
VERSION_MINOR=0
BOCHS_CONFIG="tools/run/bochsrc.txt"
DEBUG=false
RT=false
SDL=false

version()
{
    echo "run (Nanvix tools) $VERSION_MAJOR.$VERSION_MINOR"
    echo "Copyright(C) 2011-2014 Pedro H. Penna"
    echo "This is free software under the GNU General Public License Version 3."
    echo "There is NO WARRANTY, to the extent permitted by law."
}

usage()
{
    echo "Usage: run"
    echo "Brief: Runs nanvix inside bochs."
    echo "Options:"
    echo "      --help       Display this information and exit"
    echo "      --version    Display program version and exit"
    echo "      --debug      Enables GDB support (default=false)"
    echo "      --real-time  Enables real-time clock support (default=false)"
    echo "      --sdl        Uses sdl2 display library (default=term)"
}

debug()
{
    DEBUG=true
}

real_time()
{
    RT=true
}

sdl()
{
    SDL=true
}

config_bochs()
{
    cat $BOCHS_CONFIG.tpl > $BOCHS_CONFIG
    if [ "$DEBUG" = true ]; then
        echo "gdbstub: enabled=1, port=1234/" >> $BOCHS_CONFIG
    fi;

    if [ "$RT" = true ]; then
        echo "clock: sync=realtime, time0=local, rtc_sync=0" >> $BOCHS_CONFIG
    else
        echo "clock: sync=none, time0=utc" >> $BOCHS_CONFIG
    fi;

    if [ "$SDL" = true ]; then
        echo "display_library: sdl2" >> $BOCHS_CONFIG
    else
        echo "display_library: term" >> $BOCHS_CONFIG
    fi;
}

while [ "$1" != "" ]; do
    PARAM=`echo $1 | awk -F= '{print $1}'`
    VALUE=`echo $1 | awk -F= '{print $2}'`
    case $PARAM in
        -h | --help)
            usage
            exit
            ;;
        -v | --version)
            version
            exit
            ;;
        -d | --debug | --gdb)
            debug
            ;;
        -rt | --real-time)
            real_time
            ;;
        -s  | --sdl)
            sdl
            ;;
        *)
            echo "ERROR: unknown parameter \"$PARAM\""
            usage
            exit 1
            ;;
    esac
    shift
done

config_bochs

$TOOLCHAIN_DIR/bochs/bin/bochs -q -f tools/run/bochsrc.txt
