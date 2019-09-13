megs: 16
romimage: file="/usr/local/share/bochs/BIOS-bochs-latest"
vgaromimage: file="/usr/local/share/bochs/VGABIOS-lgpl-latest"
boot: cdrom
log: bochsout.txt
mouse: enabled=0
magic_break: enabled=1
ata0: enabled=1, ioaddr1=0x1f0, ioaddr2=0x3f0, irq=14
ata1: enabled=1, ioaddr1=0x170, ioaddr2=0x370, irq=15
ata0-master: type=disk, path=hdd.img, mode=flat, cylinders=130, heads=16, spt=63
ata0-slave: type=cdrom, path=nanvix.iso, status=inserted
