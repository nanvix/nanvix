#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

RULE=${1:-build}
NANVIX_HOME=${2:-`git rev-parse --show-toplevel`}

#===================================================================================================
# Global Variables
#===================================================================================================

BINARIES_DIR=${NANVIX_HOME}/bin
SCRIPTS_DIR=${NANVIX_HOME}/scripts
SERVICE_FILE=${SCRIPTS_DIR}/linuxd.service
SERVICE_ELF_BASENAME=linuxd.elf
SERVICE_ELF=${BINARIES_DIR}/${SERVICE_ELF_BASENAME}
CONTRIB_DIR=${NANVIX_HOME}/contrib
CLOUD_HYPERVISOR_HOME=${CONTRIB_DIR}/cloud-hypervisor
IMAGES_DIR=${NANVIX_HOME}/images
# The VM will be configure to use the IP 192.168.249.2 when using this specific MAC Address
GUEST_MAC_ADDRESS=12:34:56:78:90:ab
# This is the IP that the VM will use when using this specific MAC Address
GUEST_TAP_IP_ADDRESS=192.168.249.2
# This is the IP that host applications can bind to so that the guest applications can connect to them
HOST_TAP_IP_ADDRESS=192.168.249.3
IMAGE_NAME=custom-ubuntu.raw

#===================================================================================================
# Distclean
#===================================================================================================

distclean() {
	git clean -fdx
}

#===================================================================================================
# Clean
#===================================================================================================

clean() {
    cargo clean
}

#===================================================================================================
# Build
#===================================================================================================

build() {
	pushd $PWD

	cd $IMAGES_DIR

	IMAGE_NAME_BASE=jammy-server-cloudimg-amd64

	# Download the image if it does not exist.
	if [ ! -f "$IMAGE_NAME_BASE.img" ];
	then
		echo "Downloading $IMAGE_NAME_BASE.img"
		wget -N https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64.img
	fi

	qemu-img convert -p -f qcow2 -O raw $IMAGE_NAME_BASE.img $IMAGE_NAME_BASE.raw

	mkdir -p mnt
	ROOTFS=/dev/mapper/$(sudo kpartx -v -a $IMAGE_NAME_BASE.raw | grep "p1 " | cut -f 3 -d " ")
	sudo mount $ROOTFS mnt
	sudo mv mnt/etc/resolv.conf mnt/etc/resolv.conf.backup

	touch extra_commands

cat >${SERVICE_FILE} <<EOF
[Unit]
Description=Linux Daemon
After=network.target

[Service]
ExecStart=/usr/bin/${SERVICE_ELF_BASENAME} -bind-addr ${GUEST_TAP_IP_ADDRESS}:1234 -bind-socket-type tcp -gateway-addr ${HOST_TAP_IP_ADDRESS}:1234 -gateway-socket-type tcp
StandardOutput=journal
StandardError=journal
Restart=always
RestartSec=5
User=cloud
Environment="PATH=/usr/bin:/usr/local/bin" "RUST_LOG=trace"

[Install]
WantedBy=multi-user.target
EOF

# TODO: Investigate if we need to add `apt remove -y --purge snapd pollinate` to the script bellow.
cat >script <<EOF
#!/bin/bash
set -xe
mount -t proc proc /proc
mount -t devpts devpts /dev/pts
echo "nameserver 1.1.1.1" > /etc/resolv.conf
export DEBIAN_FRONTEND=noninteractive
systemctl daemon-reexec
systemctl daemon-reload
systemctl enable $(basename "${SERVICE_FILE}")
systemctl start $(basename "${SERVICE_FILE}")
source extra_commands
umount /dev/pts
umount /proc
history -c
exit
EOF

	sudo cp script extra_commands mnt
	sudo cp ${SERVICE_FILE} mnt/etc/systemd/system/$(basename "${SERVICE_FILE}")
	sudo cp ${SERVICE_ELF} mnt/usr/bin/${SERVICE_ELF_BASENAME}
	sudo chmod +x mnt/script
	sudo chroot mnt ./script
	sudo mv mnt/etc/resolv.conf.backup mnt/etc/resolv.conf
	sudo umount mnt
	sudo kpartx -d $IMAGE_NAME_BASE.raw
	cp $IMAGE_NAME_BASE.raw $IMAGE_NAME

	popd
}

#===================================================================================================
# Run
#===================================================================================================

run() {
	./target/release/cloud-hypervisor \
		--kernel $IMAGES_DIR/hypervisor-fw \
		--disk path=$IMAGES_DIR/$IMAGE_NAME path=$IMAGES_DIR/ubuntu-cloudinit.img \
		--cpus boot=2 \
		--memory size=1024M \
		--net tap=,mac=$GUEST_MAC_ADDRESS,ip=$HOST_TAP_IP_ADDRESS,mask=255.255.255.0,num_queues=2,queue_size=256
}

#===================================================================================================
# Build
#===================================================================================================

init() {
	cargo build --release

	sudo setcap cap_net_admin+ep ./target/release/cloud-hypervisor

	mkdir -p $IMAGES_DIR

	bash ./scripts/create-cloud-init.sh
	mv /tmp/ubuntu-cloudinit.img $IMAGES_DIR

	cd $IMAGES_DIR
	wget https://github.com/cloud-hypervisor/rust-hypervisor-firmware/releases/download/0.5.0/hypervisor-fw
}

#===================================================================================================

# Fetch submodule if needed.
git submodule update --init $CLOUD_HYPERVISOR_HOME

# Switch to submodule directory.
cd ${CLOUD_HYPERVISOR_HOME}

case $RULE in
	build)
		build
		;;
	clean)
		clean
		;;
	distclean)
		distclean
		;;
	init)
		init
		;;
	run)
		run
		;;
esac
