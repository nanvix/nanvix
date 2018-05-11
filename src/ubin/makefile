# 
# Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com> 
#              2015-2018 Davidson Francis <davidsondfgl@hotmail.com>
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

# Toolchain configuration.
export CFLAGS  += -Os -D_POSIX_C_SOURCE
export LDFLAGS += -Wl,-e,_start

# Conflicts.
.PHONY: cat
.PHONY: chgrp
.PHONY: chmod
.PHONY: chown
.PHONY: cp
.PHONY: date
.PHONY: echo
.PHONY: kill
.PHONY: ln
.PHONY: login
.PHONY: ls
.PHONY: mkdir
.PHONY: mv
.PHONY: nice
.PHONY: pwd
.PHONY: rm
.PHONY: rmdir
.PHONY: stat
.PHONY: sync
.PHONY: touch
.PHONY: tsh
.PHONY: ps
.PHONY: clear
.PHONY: nim
.PHONY: mount
.PHONY: unmount
.PHONY: mkfs

# Newlib considers some POSIX functions as not strict
export CFLAGS += -U__STRICT_ANSI__

# Builds everything.
all: cat chgrp chmod chown cp echo kill ln login ls mv nice pwd rm stat \
	sync tsh ps mount unmount mkfs

# Builds cat.
cat: 
	$(CC) $(CFLAGS) $(LDFLAGS) cat/*.c -o $(UBINDIR)/cat $(LIBDIR)/libc.a -lgcc
	
# Builds chgrp.
chgrp: 
	$(CC) $(CFLAGS) $(LDFLAGS) chgrp/*.c -o $(UBINDIR)/chgrp $(LIBDIR)/libc.a -lgcc
	
# Builds chmod.
chmod: 
	$(CC) $(CFLAGS) $(LDFLAGS) chmod/*.c -o $(UBINDIR)/chmod $(LIBDIR)/libc.a -lgcc
	
# Builds chown.
chown: 
	$(CC) $(CFLAGS) $(LDFLAGS) chown/*.c -o $(UBINDIR)/chown $(LIBDIR)/libc.a -lgcc
	
# Builds cp.
cp: 
	$(CC) $(CFLAGS) $(LDFLAGS) cp/*.c -o $(UBINDIR)/cp $(LIBDIR)/libc.a -lgcc

# Builds date.
date: 
	$(CC) $(CFLAGS) $(LDFLAGS) date/*.c -o $(UBINDIR)/date $(LIBDIR)/libc.a -lgcc

# Builds echo.
echo: 
	$(CC) $(CFLAGS) $(LDFLAGS) echo/*.c -o $(UBINDIR)/echo $(LIBDIR)/libc.a -lgcc
	
# Builds kill.
kill: 
	$(CC) $(CFLAGS) $(LDFLAGS) kill/*.c -o $(UBINDIR)/kill $(LIBDIR)/libc.a -lgcc
	
# Builds ln.
ln: 
	$(CC) $(CFLAGS) $(LDFLAGS) ln/*.c -o $(UBINDIR)/ln $(LIBDIR)/libc.a -lgcc
	
# Builds login.
login: 
	$(CC) $(CFLAGS) $(LDFLAGS) login/*.c -o $(UBINDIR)/login $(LIBDIR)/libc.a -lgcc
	
# Builds ls.
ls: 
	$(CC) $(CFLAGS) $(LDFLAGS) ls/*.c -o $(UBINDIR)/ls $(LIBDIR)/libc.a -lgcc
	
# Builds mkdir.
mkdir: 
	$(CC) $(CFLAGS) $(LDFLAGS) mkdir/*.c -o $(UBINDIR)/mkdir $(LIBDIR)/libc.a -lgcc
	
# Builds mv.
mv: 
	$(CC) $(CFLAGS) $(LDFLAGS) mv/*.c -o $(UBINDIR)/mv $(LIBDIR)/libc.a -lgcc
	
# Builds nice.
nice: 
	$(CC) $(CFLAGS) $(LDFLAGS) nice/*.c -o $(UBINDIR)/nice $(LIBDIR)/libc.a -lgcc
	
# Builds pwd.
pwd: 
	$(CC) $(CFLAGS) $(LDFLAGS) pwd/*.c -o $(UBINDIR)/pwd $(LIBDIR)/libc.a -lgcc
	
# Builds rm.
rm: 
	$(CC) $(CFLAGS) $(LDFLAGS) rm/*.c -o $(UBINDIR)/rm $(LIBDIR)/libc.a -lgcc
	
# Builds rmdir.
rmdir: 
	$(CC) $(CFLAGS) $(LDFLAGS) rmdir/*.c -o $(UBINDIR)/rmdir $(LIBDIR)/libc.a -lgcc
	
# Builds stat.
stat: 
	$(CC) $(CFLAGS) $(LDFLAGS) stat/*.c -o $(UBINDIR)/stat $(LIBDIR)/libc.a -lgcc
	
# Builds sync.
sync: 
	$(CC) $(CFLAGS) $(LDFLAGS) sync/*.c -o $(UBINDIR)/sync $(LIBDIR)/libc.a -lgcc
	
# Builds touch.
touch: 
	$(CC) $(CFLAGS) $(LDFLAGS) touch/*.c -o $(UBINDIR)/touch $(LIBDIR)/libc.a -lgcc
	
# Builds tsh.
tsh: 
	$(CC) $(CFLAGS) $(LDFLAGS) tsh/*.c -o $(UBINDIR)/tsh $(LIBDIR)/libc.a -lgcc

# Builds ps.
ps: 
	$(CC) $(CFLAGS) $(LDFLAGS) ps/*.c -o $(UBINDIR)/ps $(LIBDIR)/libc.a -lgcc

# Builds clear.
clear: 
	$(CC) $(CFLAGS) $(LDFLAGS) clear/*.c -o $(UBINDIR)/clear $(LIBDIR)/libc.a -lgcc

# Builds nim.
nim: 
	$(CC) $(CFLAGS) $(LDFLAGS) nim/*.c -o $(UBINDIR)/nim $(LIBDIR)/libc.a -lgcc

# Builds mount.
mount: 
	$(CC) $(CFLAGS) $(LDFLAGS) mount/*.c -o $(UBINDIR)/mount $(LIBDIR)/libc.a -lgcc

# Builds unmount.
unmount: 
	$(CC) $(CFLAGS) $(LDFLAGS) unmount/*.c -o $(UBINDIR)/unmount $(LIBDIR)/libc.a -lgcc

# Builds unmount.
mkfs: 
	$(CC) $(CFLAGS) $(LDFLAGS) mkfs/*.c -o $(UBINDIR)/mkfs $(LIBDIR)/libc.a -lgcc

# Clean compilation files.
clean:
	@rm -f $(UBINDIR)/cat
	@rm -f $(UBINDIR)/chgrp
	@rm -f $(UBINDIR)/chmod
	@rm -f $(UBINDIR)/chown
	@rm -f $(UBINDIR)/cp
	@rm -f $(UBINDIR)/echo
	@rm -f $(UBINDIR)/kill
	@rm -f $(UBINDIR)/ln
	@rm -f $(UBINDIR)/login
	@rm -f $(UBINDIR)/ls
	@rm -f $(UBINDIR)/mkdir
	@rm -f $(UBINDIR)/mv
	@rm -f $(UBINDIR)/nice
	@rm -f $(UBINDIR)/pwd
	@rm -f $(UBINDIR)/rm
	@rm -f $(UBINDIR)/rmdir
	@rm -f $(UBINDIR)/stat
	@rm -f $(UBINDIR)/sync
	@rm -f $(UBINDIR)/touch
	@rm -f $(UBINDIR)/tsh
	@rm -f $(UBINDIR)/ps
	@rm -f $(UBINDIR)/clear
	@rm -f $(UBINDIR)/nim
	@rm -f $(UBINDIR)/mount
	@rm -f $(UBINDIR)/unmount
	@rm -f $(UBINDIR)/mkfs
