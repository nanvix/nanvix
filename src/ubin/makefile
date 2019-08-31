# 
# Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com> 
#              2015-2016 Davidson Francis <davidsondfgl@hotmail.com>
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

# Conflicts.
.PHONY: cat
.PHONY: chgrp
.PHONY: chmod
.PHONY: chown
.PHONY: cp
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
.PHONY: sleep

# Builds everything.
all: cat chgrp chmod chown cp echo kill ln login ls mv nice pwd rm stat sync \
	 touch tsh ps clear nim sleep

# Builds cat.
cat: 
	$(CC) $(CFLAGS) $(LDFLAGS) cat/*.c -o $(UBINDIR)/cat $(LIBDIR)/libc.a
	
# Builds chgrp.
chgrp: 
	$(CC) $(CFLAGS) $(LDFLAGS) chgrp/*.c -o $(UBINDIR)/chgrp $(LIBDIR)/libc.a
	
# Builds chmod.
chmod: 
	$(CC) $(CFLAGS) $(LDFLAGS) chmod/*.c -o $(UBINDIR)/chmod $(LIBDIR)/libc.a
	
# Builds chown.
chown: 
	$(CC) $(CFLAGS) $(LDFLAGS) chown/*.c -o $(UBINDIR)/chown $(LIBDIR)/libc.a
	
# Builds cp.
cp: 
	$(CC) $(CFLAGS) $(LDFLAGS) cp/*.c -o $(UBINDIR)/cp $(LIBDIR)/libc.a
	
# Builds echo.
echo: 
	$(CC) $(CFLAGS) $(LDFLAGS) echo/*.c -o $(UBINDIR)/echo $(LIBDIR)/libc.a
	
# Builds kill.
kill: 
	$(CC) $(CFLAGS) $(LDFLAGS) kill/*.c -o $(UBINDIR)/kill $(LIBDIR)/libc.a
	
# Builds ln.
ln: 
	$(CC) $(CFLAGS) $(LDFLAGS) ln/*.c -o $(UBINDIR)/ln $(LIBDIR)/libc.a
	
# Builds login.
login: 
	$(CC) $(CFLAGS) $(LDFLAGS) login/*.c -o $(UBINDIR)/login $(LIBDIR)/libc.a
	
# Builds ls.
ls: 
	$(CC) $(CFLAGS) $(LDFLAGS) ls/*.c -o $(UBINDIR)/ls $(LIBDIR)/libc.a
	
# Builds mkdir.
mkdir: 
	$(CC) $(CFLAGS) $(LDFLAGS) mkdir/*.c -o $(UBINDIR)/mkdir $(LIBDIR)/libc.a
	
# Builds mv.
mv: 
	$(CC) $(CFLAGS) $(LDFLAGS) mv/*.c -o $(UBINDIR)/mv $(LIBDIR)/libc.a
	
# Builds nice.
nice: 
	$(CC) $(CFLAGS) $(LDFLAGS) nice/*.c -o $(UBINDIR)/nice $(LIBDIR)/libc.a
	
# Builds pwd.
pwd: 
	$(CC) $(CFLAGS) $(LDFLAGS) pwd/*.c -o $(UBINDIR)/pwd $(LIBDIR)/libc.a
	
# Builds rm.
rm: 
	$(CC) $(CFLAGS) $(LDFLAGS) rm/*.c -o $(UBINDIR)/rm $(LIBDIR)/libc.a
	
# Builds rmdir.
rmdir: 
	$(CC) $(CFLAGS) $(LDFLAGS) rmdir/*.c -o $(UBINDIR)/rmdir $(LIBDIR)/libc.a
	
# Builds stat.
stat: 
	$(CC) $(CFLAGS) $(LDFLAGS) stat/*.c -o $(UBINDIR)/stat $(LIBDIR)/libc.a
	
# Builds sync.
sync: 
	$(CC) $(CFLAGS) $(LDFLAGS) sync/*.c -o $(UBINDIR)/sync $(LIBDIR)/libc.a
	
# Builds touch.
touch: 
	$(CC) $(CFLAGS) $(LDFLAGS) touch/*.c -o $(UBINDIR)/touch $(LIBDIR)/libc.a
	
# Builds tsh.
tsh: 
	$(CC) $(CFLAGS) $(LDFLAGS) tsh/*.c -o $(UBINDIR)/tsh $(LIBDIR)/libc.a

# Builds ps.
ps: 
	$(CC) $(CFLAGS) $(LDFLAGS) ps/*.c -o $(UBINDIR)/ps $(LIBDIR)/libc.a

# Builds clear.
clear: 
	$(CC) $(CFLAGS) $(LDFLAGS) clear/*.c -o $(UBINDIR)/clear $(LIBDIR)/libc.a

# Builds nim.
nim: 
	$(CC) $(CFLAGS) $(LDFLAGS) nim/*.c -o $(UBINDIR)/nim $(LIBDIR)/libc.a

# Builds sleep.
sleep: 
	$(CC) $(CFLAGS) $(LDFLAGS) -D TESTE sleep/*.c -o $(UBINDIR)/sleep $(LIBDIR)/libc.a


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
	@rm -f $(UBINDIR)/sleep
