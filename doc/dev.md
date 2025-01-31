# Developing Nanvix

## Testing syscalls

Tests for syscalls can be found in `src/benchmarks/linux-app`.

To run these tests, you can use the following commands:

```bash
# (1) Set up the environment using Docker
docker build -t nanvix/toolchain build/scripts/setup/

# (2) Build the project inside the Docker container
docker run --rm -v"$(pwd):/mnt" nanvix/toolchain /bin/bash -l -c "cd /mnt ; git config --global --add safe.directory /mnt ; make TOOLCHAIN_DIR=/opt MACHINE=microvm RELEASE=no PROFILER=yes LOG_LEVEL=error all"

# (3) Run nanvixd
RUST_LOG=debug ./bin/nanvixd.elf -http-addr 127.0.0.1:8080 -linuxd-addr 127.0.0.1:7070 -sandbox-addr 127.0.0.1:1234 -keep-alive 0

# (4) Send a curl to nanvixd specifying the program to run (e.g., linux-app.elf)
curl -w "\n" --header "Content-Type: application/json" --request POST --data '{"clientid":1, "program":"bin/linux-app.elf", "args":[]}' http://localhost:8080

