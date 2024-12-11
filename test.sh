SOCKADDR=$1
PORT=$2

# Run nanvixd.
RUST_LOG=trace ./bin/nanvixd.elf -http-addr ${SOCKADDR}:${PORT}  -keep-alive 0 2>&1 2> nanvixd.log &
NANVIXD_PID=$!

# Wait for nanvixd to start.
sleep 0.1;

# Run a client.
curl \
    --silent \
    --header \
    "Content-Type: application/json" \
    --request POST \
    --data \
        '{"clientid":1,\
          "program":"bin/echo.elf",\
          "args":["hello world"]\
        }'\
        http://localhost:${PORT}\
    > curl.log

# Check if curl.log contains the expected output.
if grep -q "hello world" curl.log; then
    echo "Test passed."
    echo "Killing $NANVIXD_PID"
    /usr/bin/kill -s SIGINT $NANVIXD_PID
    echo "Killed $NANVIXD_PID"
    exit 0
else
    echo "Test failed."
    echo "Killing $NANVIXD_PID"
    /usr/bin/kill -s SIGINT $NANVIXD_PID
    echo "Killed $NANVIXD_PID"
    exit 1
fi
