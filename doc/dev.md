# Developing Nanvix

## Testing syscalls

Tests for syscalls can be found in `src/benchmarks/linux-app`.

To run these tests, you can use the following commands:

```bash
# (1) Build the project
make MACHINE=<some machine> LOG_LEVEL=trace all
# (2) Run nanvixd
RUST_LOG=debug ./bin/nanvixd.elf -http-addr 127.0.0.1:8080 -linuxd-addr 127.0.0.1:7070 -sandbox-addr 127.0.0.1:1234 -keep-alive 0
# (3) Send a curl to nanvixd specifying the program to run (e.g., linux-app.elf)
curl -w "\n" --header "Content-Type: application/json" --request POST --data '{"clientid":1, "program":"bin/linux-app.elf", "args":[]}' http://localhost:8080
```
