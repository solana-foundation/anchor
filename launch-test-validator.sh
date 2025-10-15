#!/bin/sh

# Launches the test validator in the background, waiting for it to be ready
# If the validator is not ready and healthy after 5 retries, exit with failure

set -o pipefail

solana-test-validator -r --quiet 2>&1 >/dev/null &
curl http://localhost:8899 \
    --json '{"jsonrpc":"2.0","id":1, "method":"getHealth"}' \
    --retry-connrefused \
    --retry 5 \
    -s | grep '"ok"' >/dev/null

if [ $? -eq 0 ]; then
    exit 0
else
    echo "Validator didn't launch" >&2
    exit 1
fi
