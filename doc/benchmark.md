# Nanvix Benchmarks

Nanvix ships with a small benchmarking tool, `nanvix-bench`, that you can use to measure the system's baseline performance.

To get the best performance out of Nanvix, we recommend pinning different components to different sets of cores. In particular, we recommend pinning `linuxd`, the Nanvix VM, and the client to different core dies, the latter preferably in a different NUMA domain. For example, in a server with 4 CPU dies, each with 5 cores, the following is a good pinning strategy:

```json
{
    "client_core_str": "0-9",
    "linuxd_core_str": "10-14",
    "nanovm_core_str": "15-19"
}
```

You will need to save this JSON file.

`nanvix-bench` currently supports the following benchmarks:

1. `cold-start`: measure the latency to start a nanvix VM from scratch, including `linuxd`.
2. `warm-start`: measure the latency to send an echo to the guest application from an outside client.
3. `warm-start-vmm`: measure the latency to send an echo to the guest application from the VMM.
4. `echo-breakdown`: breaks down the overheads of a warm start by time-stamping messages as they flow through the system.

you may run each experiment with:

```bash
./bin/nanvix-bench.elf -benchmark [cold-start,warm-start,warm-start-vmm,echo-breakdown]
```

if you are pinning cores, make sure to also pass the path to your JSON config file:

```bash
./bin/nanvix-bench.elf -benchmark [cold-start,warm-start,warm-start-vmm,echo-breakdown] -hwloc <path_to_file.json>
```

> ℹ️ **Note:** To run `cold-start`, `warm-start`, and `warm-start-vmm`, compile Nanvix with `make all RELEASE=yes LOG_LEVEL=panic`.

> ℹ️ **Note:** To run `echo-breakdown`, compile Nanvix with `make all RELEASE=yes LOG_LEVEL=panic TIMESTAMP_MSG=yes`.
