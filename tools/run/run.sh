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
#   - You should run this script with superuser privileges.

VERSION_MAJOR=1
VERSION_MINOR=0
BOCHS_CONFIG="tools/run/bochsrc.txt"
DEBUG=false
RT=false

version()
{
    echo "run (Nanvix tools) $VERSION_MAJOR.$VERSION_MINOR\n"
    echo "Copyright(C) 2011-2014 Pedro H. Penna"
    echo "This is free software under the GNU General Public License Version 3."
    echo "There is NO WARRANTY, to the extent permitted by law."
}

usage()
{
    echo "Usage: run\n"
    echo "Brief: Runs nanvix inside bochs.\n"
    echo "Options:"
    echo "      --help    Display this information and exit"
    echo "      --version Display program version and exit"
}

debug()
{
    DEBUG=true
}
    
real_time()
{
    RT=true
}

update_bosch()
{
    if [ "$DEBUG" = true ]; then
        sed 's/#GDBSTUB_ENABLED#/1/' $BOCHS_CONFIG.tpl > $BOCHS_CONFIG
    else
        sed 's/#GDBSTUB_ENABLED#/0/' $BOCHS_CONFIG.tpl > $BOCHS_CONFIG
    fi;

    if [ "$RT" = true ]; then
        sed -i 's/#RT_ENABLED#/sync=realtime, time0=local, rtc_sync=0/' $BOCHS_CONFIG
    else
        sed -i 's/#RT_ENABLED#/sync=none, time0=utc/' $BOCHS_CONFIG
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
        *)
            echo "ERROR: unknown parameter \"$PARAM\""
            usage
            exit 1
            ;;
    esac
    shift
done

update_bosch

bochs -q -f tools/run/bochsrc.txt
