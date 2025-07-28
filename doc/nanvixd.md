# Nanvixd HTTP API

Nanvixd exposes an HTTP API to manage the life-cycle of applications running on
top of Nanvix.

To start a new application, you must POST a `NEW` request with a JSON payload
with three (string) fields: `tenant_id`, `app_name`, and `program`, where the
latter is the path (relative to the directory root) of the `.elf` file to run.
The POST request, if successful, returns a JSON with the ID of the newly
created VM.

Below is an example of a `NEW` request using `jq` for clarity:

```bash
NEW_JSON=$(jq -n \
    --arg tenant_id "foo" \
    --arg app_name "bar" \
    --arg program "./bin/hello-c.elf" \
    --arg program_args "" \
    '{tenant_id: $tenant_id, app_name: $app_name, program: $program, program_args: $program_args}'
)
NEW_RESPONSE=$(curl \
    --silent \
    --header "Content-Type: application/json" \
    --header "X-NVX-Message-Type: NEW" \
    --request POST \
    --data "${NEW_JSON}" \
    http://${NANVIX_HTTP_ADDR})
VM_ID=$(echo ${NEW_RESPONSE} | jq -r '.user_vm_id')
GATEWAY_SOCKADDR=$(echo ${NEW_RESPONSE} | jq -r '.gateway_sockaddr')
```

where `${NANVIX_HTTP_ADDR}` corresponds to the argument passed to `-http-addr`
when starting `./bin/nanvixd.elf`.

Once the user VM is running, you can feed input to its STDIN (and read from
its STDOUT) by opening a socket to the address returned by `curl`:

```bash
# Interactive session.
nc -U ${GATEWAY_SOCKADDR}

# One-off input.
echo "Hello World!" | nc -U -q 0 ${GATEWAY_SOCKADDR}
```

once you are done, you can kill the user VM by sending a `KILL` POST:

```bash
KILL_JSON=$(jq -n \
    --arg user_vm_id "${USER_VM_ID}" \
    '{user_vm_id: $user_vm_id}'
)
curl \
    --silent \
    --header "Content-Type: application/json" \
    --header "X-NVX-Message-Type: KILL" \
    --request POST \
    --data "${KILL_JSON}" \
    http://localhost:${NANVIXD_PORT_NUMBER}
```
