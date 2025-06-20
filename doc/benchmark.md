# Nanvix Benchmarks

Nanvix ships with a small benchmarking tool, `nanvix-bench`, that you can use to measure the system's baseline performance.

The experiments pin threads to cores. We recommend pinning `linuxd`, the Nanvix VM, and the client on a different set of cores, the latter preferably in a different NUMA domain. You may see, and modify, the current pinning strategy in [`./src/utils/nanvix-bench/src/hwloc.rs`](./src/utils/nanvix-bench/src/hwloc.rs).

`nanvix-bench` currently supports the following benchmarks:

1. `cold-start`: measure the latency to start a nanvix VM from scratch, including `linuxd`.
2. `warm-start`: measure the latency to send an echo to the guest application from an outside client.
3. `warm-start-vmm`: measure the latency to send an echo to the guest application from the VMM.
4. `echo-breakdown`: breaks down the overheads of a warm start by time-stamping messages as they flow through the system.

you may run each experiment with:

```bash
./bin/nanvix-bench.elf -benchmark [cold-start,warm-start,warm-start-vmm,echo-breakdown]
```

> ℹ️ **Note:** To run `cold-start`, `warm-start`, and `warm-start-vmm`, compile Nanvix with `make all RELEASE=yes LOG_LEVEL=panic`.

> ℹ️ **Note:** To run `echo-breakdown`, compile Nanvix with `make all RELEASE=yes LOG_LEVEL=panic TIMESTAMP_MSG=yes`.
