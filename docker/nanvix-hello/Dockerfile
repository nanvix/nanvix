# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

FROM scratch
COPY bin/hello-rust-nostd.elf /initrd/hello-rust-nostd.elf
LABEL com.nanvix.os="nanvix"
LABEL com.nanvix.arch="x86"
LABEL com.nanvix.initrd.path="/initrd/hello-rust-nostd.elf"
ENTRYPOINT ["/initrd/hello-rust-nostd.elf"]
