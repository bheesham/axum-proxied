#!/usr/bin/env bash
set -e

cargo build --example proxy-protocol
cargo run --example proxy-protocol & SERVER_PID=$!
echo "proxy-protocol ($SERVER_PID)"
sleep 3

set +e
sh dev/test-proxy.sh
STATUS=$?
set -e

kill -9 "$SERVER_PID"
exit $STATUS
