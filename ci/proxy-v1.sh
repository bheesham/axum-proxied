#!/usr/bin/env bash
set -e

cargo build --example proxy
cargo run --example proxy & SERVER_PID=$!
echo "running proxy example ($SERVER_PID)"
sleep 3

set +e
sh dev/test-proxy-v1.sh
STATUS=$?
set -e

kill -9 "$SERVER_PID"
exit $STATUS
