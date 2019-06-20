#
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#! /bin/bash

if [ "x$(uname -rv | grep Darwin)" == "x" ];
then
    cc=gcc
else
    # macOS compatibility stuff. Ok, that's ugly.
    cc=$(find /usr/local/Cellar/gcc/*/bin -name "gcc-*" | grep -v ranlib | grep -v nm | grep -v "\-ar\-")
    if [ "x$cc" == "x" ];
    then
        echo "Couldn't find gcc. Please run 'brew install gcc' and try again."
        exit 1
    fi
fi

cd .. && make CC=$cc
