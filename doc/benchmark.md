# Nanvix Benchmarks

Nanvix ships with a small benchmarking tool, `nanvix-bench`, that you can use to measure the system's baseline performance.

To get the best performance out of Nanvix, we recommend pinning different components to different sets of cores. In particular, we recommend pinning `linuxd`, the user VM, and the client to different core dies, the latter preferably in a different NUMA domain. For example, in a server with 4 CPU dies, each with 5 cores, the following is a good pinning strategy:

```json
{
    "client_core_str": "0-9",
    "linuxd_core_str": "10-14",
    "nanovm_core_str": "15-19"
}
```

You will need to save this JSON file.

`nanvix-bench` currently supports the following benchmarks:

1. `boot-time`: measure the time to start a user VM (excluding `nanvixd`).
1. `cold-start`: measure the latency to start a linuxd and a user VM from scratch and send an HTTP echo to the guest.
1. `cold-start-l2`: same as `cold-start`, but deploy linuxd inside an L2 VM.
1. `cold-start-uvm`: same as `cold-start`, but reuse an existing linuxd instance.
1. `concurrent`: same as `cold-start`, but keep the (one) linuxd and (many) user VM instances alive after each iteration.
1. `concurrent-l2`: same as `concurrent`, but deploy the one linuxd instance inside an L2 VM.
1. `echo-breakdown`: breakdown the contribution of each step in the data-path when sending an HTTP echo (requires re-compilation with `TIMESTAMP_MSG=yes`).
1. `round-trip-latency`: measure the latency as we increase the size of the HTTP echo payload.
2. `warm-start`: measure only the latency to send a fixed-size HTTP echo.
3. `warm-start-vmm`: same as above, but excluding `nanvixd`.

you may see all the optional flags with:

```bash
./bin/nanvix-bench.elf -help
```

most importantly, if you are pinning cores, make sure to also pass the path to your JSON config file:

```bash
./bin/nanvix-bench.elf -benchmark <benchmark> -hwloc <path_to_file.json>
```

> ℹ️ **Note:** All benchmarks require compiling Nanvix with `RELEASE=yes` and `LOG_LEVEL=panic`.
> ℹ️ **Note:** If you are running the benchmarks with a high number of iterations, consider setting high system limits in the process spawning `nanvix-bench.elf` (i.e. `ulimit -u` and `ulimit -n`).
